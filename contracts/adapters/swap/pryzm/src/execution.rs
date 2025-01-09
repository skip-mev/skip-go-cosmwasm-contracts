use crate::consts;
use crate::error::ContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Coin, CosmosMsg, Uint128};
use pryzm_std::types::cosmos::base::v1beta1::Coin as CosmosCoin;
use pryzm_std::types::pryzm::{
    amm::v1::{MsgBatchSwap, SwapStep, SwapType},
    icstaking::v1::MsgStake,
};
use skip::swap::SwapOperation;
use std::collections::VecDeque;

// SwapExecutionStep is an enum that represents the different types of swap operations that can be executed
#[cw_serde]
pub enum SwapExecutionStep {
    // Swap represents a batch swap operation on the AMM module
    Swap {
        swap_steps: Vec<SwapStep>, // the batch swap steps
    },
    // Stake represents a liquid staking operation on Pryzm's icstaking module
    Stake {
        host_chain_id: String,    // the host chain id for staking
        transfer_channel: String, // the transfer channel of the tokens
    },
}

impl SwapExecutionStep {
    // Converts the step to the appropriate Pryzm message
    pub fn to_cosmos_msg(
        &self,
        address: String,
        coin_in: Coin,
    ) -> Result<CosmosMsg, ContractError> {
        match self {
            SwapExecutionStep::Swap { swap_steps } => {
                create_amm_swap_msg(swap_steps, address, coin_in)
            }
            SwapExecutionStep::Stake {
                host_chain_id,
                transfer_channel,
            } => create_icstaking_stake_msg(
                host_chain_id.clone(),
                transfer_channel.clone(),
                address,
                coin_in,
            ),
        }
    }

    // Returns the output denom of the step
    pub fn get_return_denom(&self) -> Result<String, ContractError> {
        match self {
            SwapExecutionStep::Swap { swap_steps } => {
                // take the last step token_out as the return denom
                let token_out = match swap_steps.last() {
                    Some(last_op) => last_op.token_out.clone(),
                    None => return Err(ContractError::SwapOperationsEmpty),
                };
                Ok(token_out)
            }
            SwapExecutionStep::Stake { host_chain_id, .. } => {
                // calculate the cAsset denom by prefixing "c:" to the host chain id
                Ok(format!("{}{}", consts::C_ASSET_PREFIX, host_chain_id))
            }
        }
    }
}

// Iterates over the swap operations and aggregates the operations into execution steps
pub fn extract_execution_steps(
    operations: Vec<SwapOperation>,
) -> Result<VecDeque<SwapExecutionStep>, ContractError> {
    // Return error if swap operations is empty
    if operations.is_empty() {
        return Err(ContractError::SwapOperationsEmpty);
    }

    // Create a vector to push the steps into
    let mut execution_steps: VecDeque<SwapExecutionStep> = VecDeque::new();

    // Create a vector to keep consecutive AMM operations in order to batch them into a single step
    let mut amm_swap_steps: Vec<SwapStep> = Vec::new();

    // Iterate over the swap operations
    let swap_operations_iter = operations.iter();
    for swap_op in swap_operations_iter {
        if swap_op.pool.starts_with(consts::ICSTAKING_POOL_PREFIX) {
            // Validate that the icstaking operation is converting an asset to a cAsset,
            // not a cAsset to an asset which is not supported
            if swap_op.denom_in.starts_with(consts::C_ASSET_PREFIX)
                || !swap_op.denom_out.starts_with(consts::C_ASSET_PREFIX)
            {
                return Err(ContractError::InvalidPool {
                    msg: format!(
                        "icstaking swap operation can only convert an asset to cAsset: cannot convert {} to {}",
                        swap_op.denom_in, swap_op.denom_out
                    )
                });
            }

            // If there are AMM swap steps from before, aggregate and push them into the execution steps
            if !amm_swap_steps.is_empty() {
                execution_steps.push_back(SwapExecutionStep::Swap {
                    swap_steps: amm_swap_steps,
                });
                amm_swap_steps = Vec::new();
            }

            // split and validate the pool string
            let split: Vec<&str> = swap_op.pool.split(':').collect();
            if split.len() != 3 {
                return Err(ContractError::InvalidPool {
                    msg: format!(
                        "icstaking pool string must be in the format \"icstaking:<host_chain_id>:<transfer_channel>\": {}",
                        swap_op.pool
                    )
                });
            }

            // Push the staking operation into the execution steps
            execution_steps.push_back(SwapExecutionStep::Stake {
                host_chain_id: split.get(1).unwrap().to_string(),
                transfer_channel: split.get(2).unwrap().to_string(),
            });
        } else if swap_op.pool.starts_with(consts::AMM_POOL_PREFIX) {
            // replace the pool prefix and parse the pool id
            let pool_id = swap_op.pool.replace(consts::AMM_POOL_PREFIX, "");
            if let Ok(pool) = pool_id.parse() {
                // Add the operation to the amm swap steps
                amm_swap_steps.push(SwapStep {
                    pool_id: pool,
                    token_in: swap_op.denom_in.clone(),
                    token_out: swap_op.denom_out.clone(),
                    amount: None,
                });
            } else {
                return Err(ContractError::InvalidPool {
                    msg: format!("invalid amm pool id {}", pool_id),
                });
            }
        } else {
            return Err(ContractError::InvalidPool {
                msg: format!(
                    "pool must be started with \"amm\" or \"icstaking\": {}",
                    swap_op.pool
                ),
            });
        }
    }

    // If there is any AMM swap steps left, push them into the execution steps
    if !amm_swap_steps.is_empty() {
        execution_steps.push_back(SwapExecutionStep::Swap {
            swap_steps: amm_swap_steps,
        });
    }

    Ok(execution_steps)
}

// create Pryzm MsgBatchSwap using the provided swap steps
pub fn create_amm_swap_msg(
    swap_steps: &[SwapStep],
    address: String,
    coin_in: Coin,
) -> Result<CosmosMsg, ContractError> {
    // take the last step token_out as the return denom
    let token_out = match swap_steps.last() {
        Some(last_op) => last_op.token_out.clone(),
        None => return Err(ContractError::SwapOperationsEmpty),
    };

    // set the amount_in on the first swap step
    let mut steps = swap_steps.to_vec();
    if let Some(first_step) = steps.get_mut(0) {
        first_step.amount = coin_in.amount.to_string().into();
    }

    // construct the message
    let swap_msg: CosmosMsg = MsgBatchSwap {
        creator: address,
        swap_type: SwapType::GivenIn.into(),
        max_amounts_in: vec![format_coin(coin_in)],
        min_amounts_out: vec![CosmosCoin {
            amount: "1".to_string(),
            denom: token_out.to_string(),
        }],
        steps,
    }
    .into();

    Ok(swap_msg)
}

// create Pryzm MsgStake using the provided host chain and transfer channel
fn create_icstaking_stake_msg(
    host_chain_id: String,
    transfer_channel: String,
    address: String,
    coin_in: Coin,
) -> Result<CosmosMsg, ContractError> {
    // Construct the message
    let msg: CosmosMsg = MsgStake {
        creator: address,
        host_chain: host_chain_id,
        transfer_channel,
        amount: coin_in.amount.into(),
    }
    .into();

    Ok(msg)
}

pub fn parse_coin(c: &CosmosCoin) -> Coin {
    let p: Uint128 = c.amount.parse().unwrap();
    coin(p.u128(), &c.denom)
}

pub fn format_coin(c: Coin) -> CosmosCoin {
    CosmosCoin {
        amount: c.amount.to_string(),
        denom: c.denom,
    }
}
