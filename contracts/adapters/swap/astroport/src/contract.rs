use crate::{
    error::{ContractError, ContractResult},
    state::{ENTRY_POINT_CONTRACT_ADDRESS, ROUTER_CONTRACT_ADDRESS},
};
use astroport::{
    asset::{Asset as AstroportAsset, AssetInfo},
    pair::{QueryMsg as PairQueryMsg, ReverseSimulationResponse},
    router::{
        ExecuteMsg as RouterExecuteMsg, QueryMsg as RouterQueryMsg, SimulateSwapOperationsResponse,
    },
};
use cosmwasm_std::{
    entry_point, to_binary, Addr, Api, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response,
    Uint128, WasmMsg,
};
use cw20::Cw20Coin;
use cw_utils::one_coin;
use skip::{
    asset::Asset,
    swap::{
        execute_transfer_funds_back, ExecuteMsg, NeutronInstantiateMsg as InstantiateMsg, QueryMsg,
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

    // Validate router contract address
    let checked_router_contract_address = deps.api.addr_validate(&msg.router_contract_address)?;

    // Store the router contract address
    ROUTER_CONTRACT_ADDRESS.save(deps.storage, &checked_router_contract_address)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        )
        .add_attribute(
            "router_contract_address",
            checked_router_contract_address.to_string(),
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
        ExecuteMsg::TransferFundsBack { swapper } => {
            Ok(execute_transfer_funds_back(deps, env, info, swapper)?)
        }
    }
}

fn execute_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operations: Vec<SwapOperation>,
) -> ContractResult<Response> {
    // Get entry point contract address from storage
    let entry_point_contract_address = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.storage)?;

    // Enforce the caller is the entry point contract
    if info.sender != entry_point_contract_address {
        return Err(ContractError::Unauthorized);
    }

    // Get coin in from the message info, error if there is not exactly one coin sent
    // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
    let coin_in = one_coin(&info)?;

    // Create the astroport swap message
    let swap_msg = create_astroport_swap_msg(
        deps.api,
        ROUTER_CONTRACT_ADDRESS.load(deps.storage)?,
        coin_in,
        operations,
    )?;

    // Create the transfer funds back message
    let transfer_funds_back_msg = WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::TransferFundsBack {
            swapper: info.sender,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(swap_msg)
        .add_message(transfer_funds_back_msg)
        .add_attribute("action", "dispatch_swap_and_transfer_back"))
}

////////////////////////
/// HELPER FUNCTIONS ///
////////////////////////

// Converts the swap operations to astroport AstroSwap operations
fn create_astroport_swap_msg(
    api: &dyn Api,
    router_contract_address: Addr,
    coin_in: Coin,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<WasmMsg> {
    // Convert the swap operations to astroport swap operations
    let astroport_swap_operations = swap_operations
        .into_iter()
        .map(|swap_operation| swap_operation.into_astroport_swap_operation(api))
        .collect();

    // Create the astroport router execute message arguments
    // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
    let astroport_router_msg_args = RouterExecuteMsg::ExecuteSwapOperations {
        operations: astroport_swap_operations,
        minimum_receive: None,
        to: None,
        max_spread: None,
    };

    // Create the astroport router swap message
    let swap_msg = WasmMsg::Execute {
        contract_addr: router_contract_address.to_string(),
        msg: to_binary(&astroport_router_msg_args)?,
        funds: vec![coin_in],
    };

    Ok(swap_msg)
}

/////////////
/// QUERY ///
/////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::RouterContractAddress {} => {
            to_binary(&ROUTER_CONTRACT_ADDRESS.load(deps.storage)?)
        }
        QueryMsg::SimulateSwapExactAssetIn {
            asset_in,
            swap_operations,
        } => to_binary(&query_simulate_swap_exact_asset_in(
            deps,
            asset_in,
            swap_operations,
        )?),
        QueryMsg::SimulateSwapExactAssetOut {
            asset_out,
            swap_operations,
        } => to_binary(&query_simulate_swap_exact_asset_out(
            deps,
            asset_out,
            swap_operations,
        )?),
    }
    .map_err(From::from)
}

// Queries the astroport router contract to simulate a swap exact amount in
// @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
fn query_simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    // Error if swap operations is empty
    let (Some(first_op), Some(last_op)) = (swap_operations.first(), swap_operations.last()) else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Ensure asset_in's denom is the same as the first swap operation's denom in
    if asset_in.denom() != first_op.denom_in {
        return Err(ContractError::CoinInDenomMismatch);
    }

    // Get the router contract address
    let router_contract_address = ROUTER_CONTRACT_ADDRESS.load(deps.storage)?;

    // Get denom out from last swap operation
    let denom_out = last_op.denom_out.clone();

    // Convert the swap operations to astroport swap operations
    let astroport_swap_operations = swap_operations
        .into_iter()
        .map(|swap_op| swap_op.into_astroport_swap_operation(deps.api))
        .collect();

    // Query the astroport router contract to simulate the swap operations
    let res: SimulateSwapOperationsResponse = deps.querier.query_wasm_smart(
        router_contract_address,
        &RouterQueryMsg::SimulateSwapOperations {
            offer_amount: asset_in.amount(),
            operations: astroport_swap_operations,
        },
    )?;

    // Return the asset out depending on whether the last swap
    // operation's denom out is a cw-20 token or a native token
    match deps.api.addr_validate(&denom_out) {
        Ok(addr) => {
            return Ok(Asset::Cw20(Cw20Coin {
                address: addr.to_string(),
                amount: res.amount,
            }))
        }
        Err(_) => {
            return Ok(Asset::Native(Coin {
                denom: denom_out,
                amount: res.amount,
            }))
        }
    }
}

// Queries the astroport pool contracts to simulate a multi-hop swap exact amount out
// @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
fn query_simulate_swap_exact_asset_out(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    // Error if swap operations is empty
    let Some(last_op) = swap_operations.last() else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Ensure coin_out's denom is the same as the last swap operation's denom out
    if asset_out.denom() != last_op.denom_out {
        return Err(ContractError::CoinOutDenomMismatch);
    }

    // Iterate through the swap operations in reverse order, querying the astroport pool contracts
    // contracts to get the coin in needed for each swap operation, and then updating the coin in
    // needed for the next swap operation until the coin in needed for the first swap operation is found.
    let asset_in_needed = swap_operations.iter().rev().try_fold(
        asset_out,
        |asset_in_needed, operation| -> Result<_, ContractError> {
            // Get ask_asset depending on whether the asset in
            // needed is a native token or a cw-20 token
            let ask_asset = match asset_in_needed {
                Asset::Native(coin) => AstroportAsset {
                    info: AssetInfo::NativeToken {
                        denom: coin.denom.clone(),
                    },
                    amount: coin.amount,
                },
                Asset::Cw20(cw20_coin) => AstroportAsset {
                    info: AssetInfo::Token {
                        contract_addr: deps.api.addr_validate(&cw20_coin.address)?,
                    },
                    amount: cw20_coin.amount,
                },
            };

            // Query the astroport pool contract to get the coin in needed for the swap operation
            let res: ReverseSimulationResponse = deps.querier.query_wasm_smart(
                &operation.pool,
                &PairQueryMsg::ReverseSimulation {
                    offer_asset_info: None,
                    ask_asset,
                },
            )?;

            match deps.api.addr_validate(&operation.denom_in) {
                Ok(addr) => {
                    return Ok(Asset::Cw20(Cw20Coin {
                        address: addr.to_string(),
                        amount: res.offer_amount.checked_add(Uint128::one())?,
                    }))
                }
                Err(_) => {
                    return Ok(Asset::Native(Coin {
                        denom: operation.denom_in.clone(),
                        amount: res.offer_amount.checked_add(Uint128::one())?,
                    }))
                }
            }
        },
    )?;

    // Return the coin in needed
    Ok(asset_in_needed)
}
