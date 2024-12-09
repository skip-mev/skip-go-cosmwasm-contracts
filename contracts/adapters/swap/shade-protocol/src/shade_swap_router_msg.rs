use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    Addr, Binary, ContractInfo, Decimal, Decimal256, Deps, DepsMut, Env, MessageInfo, Response,
    Uint128, Uint256, WasmMsg,
};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cw_serde]
pub struct TokenAmount {
    pub token: TokenType,
    pub amount: Uint128,
}

#[cw_serde]
pub enum TokenType {
    CustomToken {
        contract_addr: Addr,
        token_code_hash: String,
    },
    NativeToken {
        denom: String,
    },
}

#[cw_serde]
pub struct StableTokenData {
    pub oracle_key: String,
    pub decimals: u8,
}

#[cw_serde]
pub struct StableTokenType {
    pub token: TokenType,
    pub stable_token_data: StableTokenData,
}

#[derive(Clone, Debug, JsonSchema)]
pub struct TokenPair(pub TokenType, pub TokenType, pub bool);

pub struct TokenPairIterator<'a> {
    pair: &'a TokenPair,
    index: u8,
}

impl Serialize for TokenPair {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (self.0.clone(), self.1.clone(), self.2.clone()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TokenPair {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
            .map(|(token_0, token_1, is_stable)| TokenPair(token_0, token_1, is_stable))
    }
}

#[cw_serde]
pub enum ExecuteMsgResponse {
    SwapResult {
        amount_in: Uint128,
        amount_out: Uint128,
    },
}

#[cw_serde]
pub enum InvokeMsg {
    SwapTokensForExact {
        path: Vec<Hop>,
        expected_return: Option<Uint128>,
        recipient: Option<String>,
    },
}

#[cw_serde]
pub struct Hop {
    pub addr: String,
    pub code_hash: String,
}

#[cw_serde]
pub struct SwapResult {
    pub return_amount: Uint128,
}

/*
#[cw_serde]
pub enum ExecuteMsg {
    // SNIP20 receiver interface
    Receive(Snip20ReceiveMsg),
    SwapTokensForExact {
        /// The token type to swap from.
        offer: TokenAmount,
        expected_return: Option<Uint128>,
        path: Vec<Hop>,
        recipient: Option<String>,
        padding: Option<String>,
    },
}
*/

#[cw_serde]
pub enum QueryMsg {
    SwapSimulation {
        offer: TokenAmount,
        path: Vec<Hop>,
        exclude_fee: Option<bool>,
    },
}

#[cw_serde]
pub enum QueryMsgResponse {
    SwapSimulation {
        total_fee_amount: Uint128,
        lp_fee_amount: Uint128,
        shade_dao_fee_amount: Uint128,
        result: SwapResult,
        price: String,
    },
}
