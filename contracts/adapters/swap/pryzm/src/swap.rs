use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, coin, Coin, CosmosMsg, StdResult, Uint128};
use pryzm_std::types::pryzm::{
    amm::v1::{MsgBatchSwap, SwapStep, SwapType},
    icstaking::v1::MsgStake,
};

use skip::proto_coin::ProtoCoin;
use skip::swap::SwapOperation;
use pryzm_std::types::cosmos;

use crate::error::ContractError;

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
    pub fn to_cosmos_msg(&self, address: String, coin_in: Coin) -> Result<CosmosMsg, Err> {
        match self {
            SwapExecutionStep::Swap => create_amm_swap_msg(self, address, coin_in),
            SwapExecutionStep::Stake => create_icstaking_stake_msg(self, address, coin_in),
        }
    }
}

fn create_amm_swap_msg(
    step: SwapExecutionStep::Swap,
    address: String,
    coin_in: Coin,
) -> Result<CosmosMsg, Err> {
    let token_out = match step.swap_steps.last() {
        Some(last_op) => last_op.token_out.clone(),
        None => return Err(ContractError::SwapOperationsEmpty),
    };
    let mut swap_steps = step.swap_steps.clone();
    if let Some(first_step) = swap_steps.get_mut(0) {
        first_step.amount = coin_in.amount.into();
    }
    let swap_msg: CosmosMsg = MsgBatchSwap {
        creator: address,
        swap_type: SwapType::GivenIn.into(),
        max_amounts_in: vec![ProtoCoin(coin_in).into()],
        min_amounts_out: vec![ProtoCoin(Coin::new(1, token_out)).into()],
        steps: swap_steps,
    }
    .into();

    Ok(swap_msg)
}

fn create_icstaking_stake_msg(
    step: SwapExecutionStep::Stake,
    address: String,
    coin_in: Coin,
) -> Result<CosmosMsg, Err> {
    let msg: CosmosMsg = MsgStake {
        creator: address,
        host_chain: step.host_chain_id.clone(),
        transfer_channel: step.transfer_channel.clone(),
        amount: ProtoCoin(coin_in).into(),
    }
    .into();

    Ok(msg)
}

pub fn parse_coin(c: cosmos::base::v1beta1::Coin) -> StdResult<Coin> {
    let p: Uint128 = c.clone().amount.parse().unwrap();
    Ok(coin(p.u128(), c.clone().denom))
}
