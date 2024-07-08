use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Coin, CosmosMsg, StdResult, Uint128};
use pryzm_std::types::pryzm::{
    amm::v1::{MsgBatchSwap, SwapStep, SwapType},
    icstaking::v1::MsgStake,
};
use pryzm_std::types::cosmos::base::v1beta1::{Coin as CosmosCoin};

use crate::error::ContractError;
use crate::consts;

#[cw_serde]
pub enum SwapExecutionStep {
    Swap {
        swap_steps: Vec<SwapStep>,
    },
    Stake {
        host_chain_id: String,
        transfer_channel: String,
    },
}

impl SwapExecutionStep {
    pub fn to_cosmos_msg(self, address: String, coin_in: Coin) -> Result<CosmosMsg, ContractError> {
        match self {
            SwapExecutionStep::Swap {swap_steps} =>
                create_amm_swap_msg(swap_steps, address, coin_in),
            SwapExecutionStep::Stake {host_chain_id, transfer_channel} =>
                create_icstaking_stake_msg(host_chain_id, transfer_channel, address, coin_in),
        }
    }

    pub fn get_return_denom(self) -> Result<String, ContractError> {
        return match self {
            SwapExecutionStep::Swap {swap_steps} => {
                let token_out = match swap_steps.last() {
                    Some(last_op) => last_op.token_out.clone(),
                    None => return Err(ContractError::SwapOperationsEmpty),
                };
                Ok(token_out)
            },
            SwapExecutionStep::Stake {host_chain_id, transfer_channel: _ } => {
                Ok(format!("{}{}", consts::C_ASSET_PREFIX, host_chain_id))
            },
        }
    }
}

fn create_amm_swap_msg(
    swap_steps: Vec<SwapStep>,
    address: String,
    coin_in: Coin,
) -> Result<CosmosMsg, ContractError> {
    let token_out = match swap_steps.last() {
        Some(last_op) => last_op.token_out.clone(),
        None => return Err(ContractError::SwapOperationsEmpty),
    };
    let mut swap_steps = swap_steps.clone();
    if let Some(first_step) = swap_steps.get_mut(0) {
        first_step.amount = coin_in.amount.to_string().into();
    }
    let swap_msg: CosmosMsg = MsgBatchSwap {
        creator: address,
        swap_type: SwapType::GivenIn.into(),
        max_amounts_in: vec![format_coin(coin_in)],
        min_amounts_out: vec![format_coin(Coin::new(1, token_out))],
        steps: swap_steps,
    }
    .into();

    Ok(swap_msg)
}

fn create_icstaking_stake_msg(
    host_chain_id: String,
    transfer_channel: String,
    address: String,
    coin_in: Coin,
) -> Result<CosmosMsg, ContractError> {
    let msg: CosmosMsg = MsgStake {
        creator: address,
        host_chain: host_chain_id,
        transfer_channel,
        amount: coin_in.amount.to_string().into(),
    }
    .into();

    Ok(msg)
}

pub fn parse_coin(c: CosmosCoin) -> StdResult<Coin> {
    let p: Uint128 = c.clone().amount.parse().unwrap();
    Ok(coin(p.u128(), c.clone().denom))
}

pub fn format_coin(c: Coin) -> CosmosCoin {
    CosmosCoin {
        amount: c.amount.to_string(),
        denom: c.denom,
    }
}
