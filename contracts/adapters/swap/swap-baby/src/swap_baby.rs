use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Coin, CosmosMsg, Decimal, StdResult, Uint128, WasmMsg};
use skip::swap::SwapOperation;

#[cw_serde]
pub enum ExecuteMsg {
    /// Swaps the provided funds for the specified output
    /// going through the provided steps. Fails if the
    /// swap does not meet the minimum amount out.
    SwapExactAmountInWithHops {
        receiver: Option<String>,
        min_out: Uint128,
        hops: Vec<Hop>,
    },
}

impl ExecuteMsg {
    pub fn into_cosmos_msg(
        self,
        addr: impl Into<String>,
        funds: Vec<Coin>,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: addr.into(),
            msg: to_json_binary(&self)?,
            funds,
        }))
    }
}

#[cw_serde]
pub struct SwapExactAmountInWithHopsResponse {
    pub coin_out: Uint128,
    pub spot_price: Decimal,
    pub fees: Vec<Coin>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(QuerySimulateSwapExactAmountInWithHopsResponse)]
    SimulateSwapExactAmountInWithHops {
        input: Coin,
        hops: Vec<Hop>,
    },
    #[returns(QuerySimulateSwapExactAmountOutWithHopsResponse)]
    SimulateSwapExactAmountOutWithHops {
        want_out: Coin,
        hops: Vec<Hop>,
    },
}

#[cw_serde]
pub struct QuerySimulateSwapExactAmountInWithHopsResponse {
    pub coin_out: Uint128,
    pub fees: Vec<Coin>,
    pub spot_price: Decimal,
}
#[cw_serde]
pub struct QuerySimulateSwapExactAmountOutWithHopsResponse {
    pub need_input: Uint128,
    pub fees: Vec<Coin>,
    pub spot_price: Decimal,
}
#[cw_serde]
pub struct Hop {
    pub pool: String,
    pub denom: String,
}
