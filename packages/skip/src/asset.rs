use crate::error::SkipError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Uint128, WasmMsg,
};
use cw20::{Cw20Coin, Cw20CoinVerified, Cw20Contract, Cw20ExecuteMsg};
use cw_utils::one_coin;

#[cw_serde]
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

    pub fn transfer_full(&self, to_address: String) -> CosmosMsg {
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

    pub fn transfer_partial(&mut self, to_address: String, amount: Uint128) -> CosmosMsg {
        match self {
            Asset::Native(coin) => {
                coin.amount = coin.amount.checked_sub(amount).unwrap();

                CosmosMsg::Bank(BankMsg::Send {
                    to_address,
                    amount: vec![Coin {
                        denom: coin.denom.clone(),
                        amount,
                    }],
                })
            }
            Asset::Cw20(coin) => {
                coin.amount = coin.amount.checked_sub(amount).unwrap();

                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: coin.address.clone(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: to_address,
                        amount,
                    })
                    .unwrap(),
                    funds: vec![],
                })
            }
        }
    }

    pub fn validate(&self, deps: &DepsMut, env: &Env, info: &MessageInfo) -> Result<(), SkipError> {
        match self {
            Asset::Native(coin) => {
                let compare_coin = one_coin(info)?;

                if compare_coin.eq(coin) {
                    Ok(())
                } else {
                    Err(SkipError::InvalidNativeCoin)
                }
            }
            Asset::Cw20(coin) => {
                let verified_cw20_coin_addr = deps.api.addr_validate(&coin.address)?;

                let verified_cw20_coin = Cw20CoinVerified {
                    address: verified_cw20_coin_addr,
                    amount: coin.amount,
                };

                let cw20_contract = Cw20Contract(verified_cw20_coin.address.clone());

                let balance = cw20_contract.balance(&deps.querier, env.contract.address.clone())?;

                let compare_coin = Cw20Coin {
                    address: verified_cw20_coin.address.to_string(),
                    amount: balance,
                };

                if compare_coin.eq(coin) {
                    Ok(())
                } else {
                    Err(SkipError::InvalidCw20Coin)
                }
            }
        }
    }
}
