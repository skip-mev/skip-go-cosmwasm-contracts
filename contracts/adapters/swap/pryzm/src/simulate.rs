use std::str::FromStr;

use cosmwasm_std::{Decimal, Deps, Uint128};
use pryzm_std::types::pryzm::amm::v1::{
    AmmQuerier, QuerySimulateBatchSwapResponse, QuerySpotPriceResponse, SwapType,
};
use pryzm_std::types::pryzm::icstaking::v1::{IcstakingQuerier, QuerySimulateStakeResponse};

use skip::asset::Asset;
use skip::swap::{
    Route, SimulateSmartSwapExactAssetInResponse, SimulateSwapExactAssetInResponse,
    SimulateSwapExactAssetOutResponse, SwapOperation,
};

use crate::error::{ContractError, ContractResult};
use crate::execution::{extract_execution_steps, parse_coin, SwapExecutionStep};

// Simulates a swap given the exact amount in
pub fn simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    // Error if swap operations is empty
    let Some(first_op) = swap_operations.first() else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Get coin in from asset in, error if asset in is not a
    // native coin because Pryzm does not support CW20 tokens.
    let coin_in = match asset_in {
        Asset::Native(coin) => coin,
        _ => return Err(ContractError::AssetNotNative),
    };

    // Ensure coin_in's denom is the same as the first swap operation's denom in
    if coin_in.denom != first_op.denom_in {
        return Err(ContractError::CoinInDenomMismatch);
    }

    // instantiate module queriers
    let amm_querier = &AmmQuerier::new(&deps.querier);
    let icstaking_querier = &IcstakingQuerier::new(&deps.querier);

    // Extract the execution steps from the provided swap operations
    let execution_steps = extract_execution_steps(swap_operations)?;

    // Iterate over steps and simulate the step given the output of the last step
    // The first step uses the coin_in as input
    let mut step_amount = coin_in;
    for step in execution_steps {
        match step {
            SwapExecutionStep::Swap { swap_steps } => {
                // Set the amount on the first step of the batch swap
                let mut vec = swap_steps.clone();
                if let Some(first_step) = vec.first_mut() {
                    first_step.amount = step_amount.amount.to_string().into();
                }
                // execute the simulation query on the amm module
                let res: QuerySimulateBatchSwapResponse =
                    amm_querier.simulate_batch_swap(SwapType::GivenIn.into(), vec)?;
                if res.amounts_out.len() != 1 {
                    return Err(ContractError::InvalidQueryResponse {
                        msg: "unexpected amounts out length is batch swap simulation".to_string(),
                    });
                }
                // set the output of the simulation as the input for the next step
                step_amount = parse_coin(res.amounts_out.first().unwrap());
            }
            SwapExecutionStep::Stake {
                host_chain_id,
                transfer_channel,
            } => {
                // execute the simulation query on the icstaking module
                let res: QuerySimulateStakeResponse = icstaking_querier.simulate_stake(
                    host_chain_id,
                    transfer_channel,
                    step_amount.amount.to_string().into(),
                    None,
                )?;
                if let Some(amount_out) = res.amount_out {
                    // set the output of the simulation as the input for the next step
                    step_amount = parse_coin(&amount_out);
                } else {
                    return Err(ContractError::InvalidQueryResponse {
                        msg: "unexpected amount_out in liquid staking simulation".to_string(),
                    });
                }
            }
        }
    }

    // return the last step output as the result of the simulation
    Ok(Asset::from(step_amount))
}

// Simulates a swap given the exact amount in, include spot price if requested
pub fn simulate_swap_exact_asset_in_with_metadata(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<SimulateSwapExactAssetInResponse> {
    // simulate the swap
    let mut response = SimulateSwapExactAssetInResponse {
        asset_out: simulate_swap_exact_asset_in(deps, asset_in, swap_operations.clone())?,
        spot_price: None,
    };

    // calculate and include spot price if requested
    if include_spot_price {
        response.spot_price = Some(calculate_spot_price(deps, swap_operations)?)
    }

    Ok(response)
}

// Simulates a swap given exact amount out
pub fn simulate_swap_exact_asset_out(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    // Error if swap operations is empty
    let Some(last_op) = swap_operations.last() else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Get coin out from asset out, error if asset out is not a
    // native coin because Osmosis does not support CW20 tokens.
    let coin_out = match asset_out {
        Asset::Native(coin) => coin,
        _ => return Err(ContractError::AssetNotNative),
    };

    // Ensure coin_out's denom is the same as the last swap operation's denom out
    if coin_out.denom != last_op.denom_out {
        return Err(ContractError::CoinOutDenomMismatch);
    }

    // instantiate module queriers
    let amm_querier = &AmmQuerier::new(&deps.querier);
    let icstaking_querier = &IcstakingQuerier::new(&deps.querier);

    // Iterate over steps starting from the last step and simulate the step given the result of the
    // last step. The first step uses the coin_out as input
    let mut step_amount = coin_out;
    let execution_steps = extract_execution_steps(swap_operations)?;
    let reverse_iter = execution_steps.into_iter().rev();
    for step in reverse_iter {
        match step {
            SwapExecutionStep::Swap { swap_steps } => {
                // make the swap steps reversed and set the amount on the first step
                let mut vec = swap_steps.clone();
                vec.reverse();
                if let Some(first_step) = vec.last_mut() {
                    first_step.amount = step_amount.amount.to_string().into();
                }
                // execute the simulation query on the amm module
                let res: QuerySimulateBatchSwapResponse =
                    amm_querier.simulate_batch_swap(SwapType::GivenOut.into(), vec)?;
                // set the output of the simulation as the input for the next step
                if res.amounts_out.len() != 1 {
                    return Err(ContractError::InvalidQueryResponse {
                        msg: "unexpected amounts out length is batch swap simulation".to_string(),
                    });
                }
                step_amount = parse_coin(res.amounts_in.first().unwrap());
            }
            SwapExecutionStep::Stake {
                host_chain_id,
                transfer_channel,
            } => {
                // execute the simulation query on the icstaking module
                let res: QuerySimulateStakeResponse = icstaking_querier.simulate_stake(
                    host_chain_id,
                    transfer_channel,
                    None,
                    step_amount.amount.to_string().into(),
                )?;
                if let Some(amount_in) = res.amount_in {
                    // set the output of the simulation as the input for the next step
                    step_amount = parse_coin(&amount_in);
                } else {
                    return Err(ContractError::InvalidQueryResponse {
                        msg: "unexpected amount_in in liquid staking simulation".to_string(),
                    });
                }
            }
        }
    }

    // return the last step output as the result of the simulation
    Ok(Asset::from(step_amount))
}

// Simulates a swap given exact amount out, include spot price if requested
pub fn simulate_swap_exact_asset_out_with_metadata(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<SimulateSwapExactAssetOutResponse> {
    // simulate the swap
    let mut response = SimulateSwapExactAssetOutResponse {
        asset_in: simulate_swap_exact_asset_out(deps, asset_out, swap_operations.clone())?,
        spot_price: None,
    };

    // calculate and include spot price if requested
    if include_spot_price {
        response.spot_price = Some(calculate_spot_price(deps, swap_operations)?)
    }

    Ok(response)
}

// Simulates a smart swap given the exact amount in
pub fn simulate_smart_swap_exact_asset_in(
    deps: Deps,
    ask_denom: String,
    routes: Vec<Route>,
) -> ContractResult<Asset> {
    // initialize the total output with zero value
    let mut asset_out = Asset::new(deps.api, &ask_denom, Uint128::zero());

    // Iterate over routes and simulate the swap for each route
    for route in &routes {
        let route_asset_out = simulate_swap_exact_asset_in(
            deps,
            route.offer_asset.clone(),
            route.operations.clone(),
        )?;

        // add the output of swap using this route to the total output
        asset_out.add(route_asset_out.amount())?;
    }

    Ok(asset_out)
}

// Simulates a smart swap given the exact amount in, and includes spot price if requested
pub fn simulate_smart_swap_exact_asset_in_with_metadata(
    deps: Deps,
    asset_in: Asset,
    ask_denom: String,
    routes: Vec<Route>,
    include_spot_price: bool,
) -> ContractResult<SimulateSmartSwapExactAssetInResponse> {
    // simulate the swap
    let asset_out = simulate_smart_swap_exact_asset_in(deps, ask_denom, routes.clone())?;

    // instantiate the response
    let mut response = SimulateSmartSwapExactAssetInResponse {
        asset_out,
        spot_price: None,
    };

    // calculate and include weighted spot price if requested
    if include_spot_price {
        response.spot_price = Some(calculate_weighted_spot_price(deps, asset_in, routes)?)
    }

    Ok(response)
}

// Calculate the spot price for the swap
fn calculate_spot_price(
    deps: Deps,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Decimal> {
    // Extract the execution steps from the provided swap operations
    let execution_steps = extract_execution_steps(swap_operations)?;

    // instantiate module queriers
    let amm_querier = &AmmQuerier::new(&deps.querier);
    let icstaking_querier = &IcstakingQuerier::new(&deps.querier);

    // iterate over execution steps, calculate spot price for each step and multiply all spot prices
    let spot_price = execution_steps.into_iter().try_fold(
        Decimal::one(),
        |curr_spot_price, step| -> ContractResult<Decimal> {
            let step_spot_price = match step {
                SwapExecutionStep::Swap { swap_steps } => swap_steps.into_iter().try_fold(
                    Decimal::one(),
                    |curr_spot_price, step| -> ContractResult<Decimal> {
                        // spot price for a Swap step can be queried from amm module
                        let spot_price_res: QuerySpotPriceResponse = amm_querier.spot_price(
                            step.pool_id,
                            step.token_in,
                            step.token_out,
                            false,
                        )?;
                        // parse the result and multiply the spot price with the current value
                        if let Ok(spot_price) = Decimal::from_str(&spot_price_res.spot_price) {
                            Ok(curr_spot_price.checked_mul(spot_price)?)
                        } else {
                            Err(ContractError::InvalidQueryResponse {
                                msg: "invalid spot price in amm spot price query".to_string(),
                            })
                        }
                    },
                ),
                SwapExecutionStep::Stake {
                    host_chain_id,
                    transfer_channel,
                } => {
                    // calculate spot price for liquid staking, by simulating stake for an amount
                    let amount = Decimal::from_str("1000000000000000000")?; // 1e18
                    let res: QuerySimulateStakeResponse = icstaking_querier.simulate_stake(
                        host_chain_id,
                        transfer_channel,
                        amount.to_string().into(),
                        None,
                    )?;
                    // calculate the spot price by dividing the output of staking by the input amount
                    if let Some(amount_out) = res.amount_out {
                        if let Ok(output) = Decimal::from_str(&amount_out.amount) {
                            Ok(output.checked_div(amount)?)
                        } else {
                            Err(ContractError::InvalidQueryResponse {
                                msg: "invalid amount for amount_out coin in staking simulation response".to_string(),
                            })
                        }
                    } else {
                        return Err(ContractError::InvalidQueryResponse {
                            msg: "unexpected amount_out in liquid staking simulation".to_string(),
                        });
                    }
                }
            };

            Ok(curr_spot_price.checked_mul(step_spot_price?)?)
        },
    )?;

    Ok(spot_price)
}

// Calculate weighted spot price for a set of routes
fn calculate_weighted_spot_price(
    deps: Deps,
    asset_in: Asset,
    routes: Vec<Route>,
) -> ContractResult<Decimal> {
    // iterate over the routes, calculate each route spot price, and multiply them based on the
    // weight of that route
    let spot_price = routes.into_iter().try_fold(
        Decimal::zero(),
        |curr_spot_price, route| -> ContractResult<Decimal> {
            // calculate route's spot price
            let route_spot_price = calculate_spot_price(deps, route.operations)?;

            // calculate the weight of the route, which is equal to the ratio of amount swapped on
            // the route to the total amount being swapped
            let weight = Decimal::from_ratio(route.offer_asset.amount(), asset_in.amount());

            Ok(curr_spot_price + (route_spot_price * weight))
        },
    )?;

    Ok(spot_price)
}
