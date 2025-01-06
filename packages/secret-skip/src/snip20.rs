use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Uint128};

#[cw_serde]
pub struct Snip20ReceiveMsg {
    pub sender: Addr,
    pub from: Addr,
    pub amount: Uint128,
    pub memo: Option<String>,
    pub msg: Option<Binary>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Transfer {
        recipient: String,
        amount: Uint128,
        memo: Option<String>,
        padding: Option<String>,
    },
    Send {
        recipient: String,
        recipient_code_hash: Option<String>,
        amount: Uint128,
        msg: Option<Binary>,
        memo: Option<String>,
        padding: Option<String>,
    },
}

#[cw_serde]
pub enum QueryResponse {
    Balance { amount: Uint128 },
}

#[cw_serde]
pub struct BalanceResponse {
    pub amount: Uint128,
}
