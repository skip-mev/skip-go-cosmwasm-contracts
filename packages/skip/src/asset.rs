use cosmwasm_std::{to_binary, BankMsg, Coin, CosmosMsg, Uint128, WasmMsg};
use cw20::{Cw20Coin, Cw20ExecuteMsg};

pub enum Asset {
    Native(Coin),
    Cw20(Cw20Coin),
}

impl From<Coin> for Asset {
    fn from(coin: Coin) -> Self {
        Asset::Native(coin)
    }
}

impl From<Cw20Coin> for Asset {
    fn from(cw20_coin: Cw20Coin) -> Self {
        Asset::Cw20(cw20_coin)
    }
}

impl Asset {
    pub fn denom(&self) -> &str {
        match self {
            Asset::Native(coin) => &coin.denom,
            Asset::Cw20(coin) => &coin.address,
        }
    }

    pub fn amount(&self) -> Uint128 {
        match self {
            Asset::Native(coin) => coin.amount,
            Asset::Cw20(coin) => coin.amount,
        }
    }

    pub fn transfer(&self, to_address: String) -> CosmosMsg {
        match self {
            Asset::Native(coin) => CosmosMsg::Bank(BankMsg::Send {
                to_address,
                amount: vec![coin.clone()],
            }),
            Asset::Cw20(coin) => CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: coin.address.clone(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: to_address,
                    amount: coin.amount,
                })
                .unwrap(),
                funds: vec![],
            }),
        }
    }
}
