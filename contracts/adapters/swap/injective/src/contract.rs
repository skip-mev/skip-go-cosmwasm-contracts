use crate::error::*;
use crate::state::{ENTRY_POINT_CONTRACT_ADDRESS, INJECTIVE_SWAP_CONTRACT_ADDRESS};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    ensure, entry_point, to_json_binary, wasm_execute, Binary, Deps, DepsMut, Env, MessageInfo,
    Response, Uint128,
};
use cw2::set_contract_version;
use cw_utils::one_coin;
use skip::asset::Asset;
use skip::swap::{
    execute_transfer_funds_back, get_ask_denom_for_routes, ExecuteMsg, InjectiveInstantiateMsg,
    MigrateMsg, QueryMsg, Route, SwapOperation,
};

#[cw_serde]
#[allow(non_camel_case_types)]
pub enum InjectiveExecuteMsg {
    SwapMinOutput {
        target_denom: String,
        min_output_quantity: String,
    },
    SwapExactOutput {
        target_denom: String,
        target_output_quantity: String,
    },
    SetRoute {
        source_denom: String,
        target_denom: String,
        route: Vec<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum InjectiveQueryMsg {
    #[returns(OutputQuantityResponse)]
    GetOutputQuantity {
        from_quantity: Uint128,
        source_denom: String,
        target_denom: String,
    },
    #[returns(InputQuantityResponse)]
    GetInputQuantity {
        to_quantity: Uint128,
        source_denom: String,
        target_denom: String,
    },
}

#[cw_serde]
pub struct OutputQuantityResponse {
    pub quantity: Uint128,
}

#[cw_serde]
pub struct InputQuantityResponse {
    pub quantity: Uint128,
}

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> ContractResult<Response> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InjectiveInstantiateMsg,
) -> ContractResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let entry = deps.api.addr_validate(&msg.entry_point_contract_address)?;
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &entry)?;

    let injective_swap_addr = deps.api.addr_validate(&msg.swap_contract_address)?;
    INJECTIVE_SWAP_CONTRACT_ADDRESS.save(deps.storage, &injective_swap_addr)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("entry_point_contract_address", entry)
        .add_attribute("injective_swap_contract_address", injective_swap_addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Swap { operations } => execute_swap(deps, env, info, operations),
        ExecuteMsg::TransferFundsBack {
            swapper,
            return_denom,
        } => Ok(execute_transfer_funds_back(
            deps,
            env,
            info,
            swapper,
            return_denom,
        )?),
        _ => unimplemented!(),
    }
}

fn execute_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operations: Vec<SwapOperation>,
) -> ContractResult<Response> {
    let entry_point = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.storage)?;
    if info.sender != entry_point {
        return Err(ContractError::Unauthorized);
    }

    let coin_in = one_coin(&info)?;
    ensure!(
        coin_in.amount != Uint128::zero(),
        ContractError::Skip(skip::error::SkipError::InvalidNativeCoin)
    );

    ensure!(!operations.is_empty(), ContractError::SwapOperationsEmpty);

    let injective = INJECTIVE_SWAP_CONTRACT_ADDRESS.load(deps.storage)?;
    let target_denom = operations.last().unwrap().denom_out.clone();

    let injective_msg = InjectiveExecuteMsg::SwapMinOutput {
        target_denom: target_denom.clone(),
        min_output_quantity: "0".to_string(),
    };

    let injective_wasm_msg = wasm_execute(injective, &injective_msg, vec![coin_in])?;

    let transfer_back_msg = ExecuteMsg::TransferFundsBack {
        swapper: entry_point,
        return_denom: target_denom.clone(),
    };
    let transfer_back_wasm_msg = wasm_execute(env.contract.address, &transfer_back_msg, vec![])?;

    Ok(Response::new()
        .add_message(injective_wasm_msg)
        .add_message(transfer_back_wasm_msg)
        .add_attribute("action", "swap")
        .add_attribute("target_denom", target_denom))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::SimulateSwapExactAssetIn {
            asset_in,
            swap_operations,
        } => to_json_binary(&query_simulate_swap_exact_asset_in(
            deps,
            asset_in,
            swap_operations,
        )?)
        .map_err(Into::into),
        QueryMsg::SimulateSwapExactAssetOut {
            asset_out,
            swap_operations,
        } => to_json_binary(&query_simulate_swap_exact_asset_out(
            deps,
            asset_out,
            swap_operations,
        )?)
        .map_err(Into::into),
        QueryMsg::SimulateSmartSwapExactAssetIn { asset_in, routes } => {
            let ask_denom = get_ask_denom_for_routes(&routes)?;
            to_json_binary(&query_simulate_smart_swap_exact_asset_in(
                deps, ask_denom, routes, asset_in,
            )?)
            .map_err(Into::into)
        }
        _ => unimplemented!(),
    }
}

fn query_simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    ensure!(
        !swap_operations.is_empty(),
        ContractError::SwapOperationsEmpty
    );

    if asset_in.denom() != swap_operations[0].denom_in {
        return Err(ContractError::CoinInDenomMismatch);
    }

    let target_denom = swap_operations.last().unwrap().denom_out.clone();
    let injective = INJECTIVE_SWAP_CONTRACT_ADDRESS.load(deps.storage)?;

    let resp: OutputQuantityResponse = deps.querier.query_wasm_smart(
        &injective,
        &InjectiveQueryMsg::GetOutputQuantity {
            from_quantity: asset_in.amount(),
            source_denom: asset_in.denom().to_string(),
            target_denom: target_denom.clone(),
        },
    )?;

    Ok(Asset::new(deps.api, &target_denom, resp.quantity))
}

fn query_simulate_swap_exact_asset_out(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    ensure!(
        !swap_operations.is_empty(),
        ContractError::SwapOperationsEmpty
    );

    if asset_out.denom() != swap_operations.last().unwrap().denom_out {
        return Err(ContractError::CoinOutDenomMismatch);
    }

    let source_denom = swap_operations.first().unwrap().denom_in.clone();
    let injective = INJECTIVE_SWAP_CONTRACT_ADDRESS.load(deps.storage)?;

    let resp: InputQuantityResponse = deps.querier.query_wasm_smart(
        &injective,
        &InjectiveQueryMsg::GetInputQuantity {
            to_quantity: asset_out.amount(),
            source_denom: source_denom.clone(),
            target_denom: asset_out.denom().to_string(),
        },
    )?;

    Ok(Asset::new(deps.api, &source_denom, resp.quantity))
}

fn query_simulate_smart_swap_exact_asset_in(
    deps: Deps,
    ask_denom: String,
    routes: Vec<Route>,
    _asset_in: Asset,
) -> ContractResult<Asset> {
    let mut total = Uint128::zero();
    for route in routes {
        let out =
            query_simulate_swap_exact_asset_in(deps, route.offer_asset.clone(), route.operations)?;
        total += out.amount();
    }
    Ok(Asset::new(deps.api, &ask_denom, total))
}
