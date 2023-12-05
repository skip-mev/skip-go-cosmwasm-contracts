use crate::{
    error::{ContractError, ContractResult},
    state::{
        BRIDGED_DENOM, CANONICAL_DENOM, ENTRY_POINT_CONTRACT_ADDRESS,
        LIDO_SATELLITE_CONTRACT_ADDRESS,
    },
};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Coin, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    WasmMsg,
};
use cw_utils::one_coin;
use skip::{
    asset::Asset,
    swap::{
        execute_transfer_funds_back, ExecuteMsg, LidoSatelliteInstantiateMsg as InstantiateMsg,
        QueryMsg, SimulateSwapExactAssetInResponse, SimulateSwapExactAssetOutResponse,
        SwapOperation,
    },
};

///////////////////
/// INSTANTIATE ///
///////////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // Validate entry point contract address
    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;

    // Validate satellite contract address
    let checked_lido_satellite_contract_address = deps
        .api
        .addr_validate(&msg.lido_satellite_contract_address)?;

    // Store the satellite contract address
    LIDO_SATELLITE_CONTRACT_ADDRESS.save(deps.storage, &checked_lido_satellite_contract_address)?;

    // Cache Lido Satellite denoms to avoid quering them at each swap
    let lido_satellite_config: lido_satellite::msg::ConfigResponse =
        deps.querier.query_wasm_smart(
            &checked_lido_satellite_contract_address,
            &lido_satellite::msg::QueryMsg::Config {},
        )?;
    CANONICAL_DENOM.save(deps.storage, &lido_satellite_config.canonical_denom)?;
    BRIDGED_DENOM.save(deps.storage, &lido_satellite_config.bridged_denom)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        )
        .add_attribute(
            "lido_satellite_contract_address",
            checked_lido_satellite_contract_address.to_string(),
        ))
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
        _ => {
            unimplemented!()
        }
    }
}

fn execute_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    // FIXME: seems like we are doomed to ignore this field at all?
    _operations: Vec<SwapOperation>,
) -> ContractResult<Response> {
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

    // Decide which message to Lido Satellite should be emitted
    let (lido_satellite_msg, return_denom) = if coin_in.denom == bridged_denom {
        (
            lido_satellite::msg::ExecuteMsg::Mint { receiver: None },
            canonical_denom,
        )
    } else if coin_in.denom == canonical_denom {
        (
            lido_satellite::msg::ExecuteMsg::Burn { receiver: None },
            bridged_denom,
        )
    } else {
        return Err(ContractError::UnsupportedDenom);
    };

    let lido_satellite_contract_address = LIDO_SATELLITE_CONTRACT_ADDRESS.load(deps.storage)?;

    let swap_msg = WasmMsg::Execute {
        contract_addr: lido_satellite_contract_address.to_string(),
        msg: to_binary(&lido_satellite_msg)?,
        funds: vec![coin_in],
    };

    // Create the transfer funds back message
    let transfer_funds_back_msg = WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::TransferFundsBack {
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
            if asset_in.denom() == bridged_denom {
                to_binary(&Asset::Native(Coin::new(
                    asset_in.amount().u128(),
                    canonical_denom,
                )))
            } else if asset_in.denom() == canonical_denom {
                to_binary(&Asset::Native(Coin::new(
                    asset_in.amount().u128(),
                    bridged_denom,
                )))
            } else {
                unimplemented!()
            }
        }
        QueryMsg::SimulateSwapExactAssetOut { asset_out, .. } => {
            if asset_out.denom() == bridged_denom {
                to_binary(&Asset::Native(Coin::new(
                    asset_out.amount().u128(),
                    canonical_denom,
                )))
            } else if asset_out.denom() == canonical_denom {
                to_binary(&Asset::Native(Coin::new(
                    asset_out.amount().u128(),
                    bridged_denom,
                )))
            } else {
                unimplemented!()
            }
        }
        QueryMsg::SimulateSwapExactAssetInWithMetadata { asset_in, .. } => {
            if asset_in.denom() == bridged_denom {
                to_binary(&SimulateSwapExactAssetInResponse {
                    asset_out: Asset::Native(Coin::new(asset_in.amount().u128(), canonical_denom)),
                    spot_price: Some(Decimal::one()),
                })
            } else if asset_in.denom() == canonical_denom {
                to_binary(&SimulateSwapExactAssetInResponse {
                    asset_out: Asset::Native(Coin::new(asset_in.amount().u128(), bridged_denom)),
                    spot_price: Some(Decimal::one()),
                })
            } else {
                unimplemented!()
            }
        }
        QueryMsg::SimulateSwapExactAssetOutWithMetadata { asset_out, .. } => {
            if asset_out.denom() == bridged_denom {
                to_binary(&SimulateSwapExactAssetOutResponse {
                    asset_in: Asset::Native(Coin::new(asset_out.amount().u128(), canonical_denom)),
                    spot_price: Some(Decimal::one()),
                })
            } else if asset_out.denom() == canonical_denom {
                to_binary(&SimulateSwapExactAssetOutResponse {
                    asset_in: Asset::Native(Coin::new(asset_out.amount().u128(), bridged_denom)),
                    spot_price: Some(Decimal::one()),
                })
            } else {
                unimplemented!()
            }
        }
        _ => {
            unimplemented!()
        }
    }
    .map_err(From::from)
}
