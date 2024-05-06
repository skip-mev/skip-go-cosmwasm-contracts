use crate::{
    error::{ContractError, ContractResult},
    state::{
        BRIDGED_DENOM, CANONICAL_DENOM, DROP_CORE_CONTRACT_ADDRESS, ENTRY_POINT_CONTRACT_ADDRESS,
    },
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Coin, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::one_coin;
use skip::{
    asset::Asset,
    error::SkipError,
    swap::{
        execute_transfer_funds_back, DropBondInstantiateMsg as InstantiateMsg, ExecuteMsg,
        MigrateMsg, QueryMsg, SimulateSwapExactAssetInResponse, SimulateSwapExactAssetOutResponse,
    },
};

///////////////
/// MIGRATE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> ContractResult<Response> {
    unimplemented!()
}

///////////////////
/// INSTANTIATE ///
///////////////////

// Contract name and version used for migration.
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // Set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate entry point contract address
    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;

    // Validate drop factory contract address
    let checked_drop_factory_contract_address =
        deps.api.addr_validate(&msg.drop_factory_contract_address)?;

    let drop_factory_state: drop_factory::state::State = deps.querier.query_wasm_smart(
        &checked_drop_factory_contract_address,
        &drop_factory::msg::QueryMsg::State {},
    )?;

    // Store the drop core contract address
    DROP_CORE_CONTRACT_ADDRESS.save(
        deps.storage,
        &deps.api.addr_validate(&drop_factory_state.core_contract)?,
    )?;

    let drop_core_config: drop_staking_base::state::core::Config = deps.querier.query_wasm_smart(
        &checked_drop_factory_contract_address,
        &drop_staking_base::msg::core::QueryMsg::Config {},
    )?;

    let canonical_denom = drop_core_config
        .ld_denom
        .ok_or(ContractError::CanonicalDenomNotSet {})?;

    CANONICAL_DENOM.save(deps.storage, &canonical_denom)?;
    BRIDGED_DENOM.save(deps.storage, &drop_core_config.remote_denom)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        )
        .add_attribute(
            "drop_factory_contract_address",
            checked_drop_factory_contract_address.to_string(),
        )
        .add_attribute(
            "drop_core_contract_address",
            drop_factory_state.core_contract,
        )
        .add_attribute("canonical_denom", canonical_denom)
        .add_attribute("bridged_denom", drop_core_config.remote_denom))
}

///////////////
/// EXECUTE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Swap { routes } => {
            if routes.len() != 1 {
                return Err(ContractError::Skip(SkipError::MustBeSingleRoute));
            }

            execute_swap(deps, env, info)
        }
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
        _ => {
            unimplemented!()
        }
    }
}

fn execute_swap(deps: DepsMut, env: Env, info: MessageInfo) -> ContractResult<Response> {
    // Get entry point contract address from storage
    let entry_point_contract_address = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.storage)?;

    // Enforce the caller is the entry point contract
    if info.sender != entry_point_contract_address {
        return Err(ContractError::Unauthorized);
    }

    // Get coin in from the message info, error if there is not exactly one coin sent
    let coin_in = one_coin(&info)?;

    let bridged_denom = BRIDGED_DENOM.load(deps.storage)?;
    let canonical_denom = CANONICAL_DENOM.load(deps.storage)?;

    // Decide which message to Core contract should be emitted
    let (drop_core_msg, return_denom) = if coin_in.denom == bridged_denom {
        (
            drop_staking_base::msg::core::ExecuteMsg::Bond { receiver: None },
            canonical_denom,
        )
    } else {
        return Err(ContractError::UnsupportedDenom);
    };

    let drop_core_contract_address = DROP_CORE_CONTRACT_ADDRESS.load(deps.storage)?;

    let swap_msg = WasmMsg::Execute {
        contract_addr: drop_core_contract_address.to_string(),
        msg: to_json_binary(&drop_core_msg)?,
        funds: vec![coin_in],
    };

    // Create the transfer funds back message
    let transfer_funds_back_msg = WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_json_binary(&ExecuteMsg::TransferFundsBack {
            swapper: info.sender,
            return_denom,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(swap_msg)
        .add_message(transfer_funds_back_msg)
        .add_attribute("action", "dispatch_swap_and_transfer_back"))
}

/////////////
/// QUERY ///
/////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    let bridged_denom = BRIDGED_DENOM.load(deps.storage)?;
    let canonical_denom = CANONICAL_DENOM.load(deps.storage)?;

    match msg {
        QueryMsg::SimulateSwapExactAssetIn { asset_in, .. } => {
            let asset_out_denom =
                get_opposite_denom(asset_in.denom(), &bridged_denom, &canonical_denom);

            let exchange_rate = get_exchange_rate(deps)?;

            to_json_binary(&Asset::Native(Coin::new(
                (exchange_rate * asset_in.amount()).into(),
                asset_out_denom,
            )))
        }
        QueryMsg::SimulateSwapExactAssetOut { asset_out, .. } => {
            let asset_in_denom =
                get_opposite_denom(asset_out.denom(), &bridged_denom, &canonical_denom);

            let exchange_rate = get_exchange_rate(deps)?;

            to_json_binary(&Asset::Native(Coin::new(
                (asset_out.amount().div_floor(exchange_rate)).into(),
                asset_in_denom,
            )))
        }
        QueryMsg::SimulateSwapExactAssetInWithMetadata {
            asset_in,
            include_spot_price,
            ..
        } => {
            let asset_out_denom =
                get_opposite_denom(asset_in.denom(), &bridged_denom, &canonical_denom);

            let exchange_rate = get_exchange_rate(deps)?;

            let spot_price = if include_spot_price {
                Some(exchange_rate)
            } else {
                None
            };

            to_json_binary(&SimulateSwapExactAssetInResponse {
                asset_out: Asset::Native(Coin::new(
                    (exchange_rate * asset_in.amount()).into(),
                    asset_out_denom,
                )),
                spot_price,
            })
        }
        QueryMsg::SimulateSwapExactAssetOutWithMetadata {
            asset_out,
            include_spot_price,
            ..
        } => {
            let asset_in_denom =
                get_opposite_denom(asset_out.denom(), &bridged_denom, &canonical_denom);

            let exchange_rate = get_exchange_rate(deps)?;

            let spot_price = if include_spot_price {
                Some(exchange_rate)
            } else {
                None
            };

            to_json_binary(&SimulateSwapExactAssetOutResponse {
                asset_in: Asset::Native(Coin::new(
                    (asset_out.amount().div_floor(exchange_rate)).into(),
                    asset_in_denom,
                )),
                spot_price,
            })
        }
    }
    .map_err(From::from)
}

fn get_opposite_denom(denom: &str, bridged_denom: &str, canonical_denom: &str) -> String {
    match denom {
        denom if denom == bridged_denom => canonical_denom.to_string(),
        denom if denom == canonical_denom => bridged_denom.to_string(),
        _ => unimplemented!(),
    }
}

fn get_exchange_rate(deps: Deps) -> ContractResult<Decimal> {
    let drop_core_contract_address = DROP_CORE_CONTRACT_ADDRESS.load(deps.storage)?;

    let exchange_rate: Decimal = deps.querier.query_wasm_smart(
        drop_core_contract_address,
        &drop_staking_base::msg::core::QueryMsg::ExchangeRate {},
    )?;

    Ok(exchange_rate)
}
