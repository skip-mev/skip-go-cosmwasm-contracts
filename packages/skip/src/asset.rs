use crate::error::SkipError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Uint128, WasmMsg,
};
use cw20::{Cw20Coin, Cw20CoinVerified, Cw20Contract, Cw20ExecuteMsg};
use cw_utils::{one_coin, nonpayable};

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

impl From<Cw20CoinVerified> for Asset {
    fn from(cw20_coin_verified: Cw20CoinVerified) -> Self {
        Asset::Cw20(Cw20Coin {
            address: cw20_coin_verified.address.to_string(),
            amount: cw20_coin_verified.amount,
        })
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
                // Validate that the message is nonpayable
                nonpayable(info)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{
        testing::{mock_dependencies_with_balances, mock_env, mock_info},
        ContractResult, QuerierResult, SystemResult, WasmQuery,
    };
    use cw20::BalanceResponse;

    #[test]
    fn test_asset_native() {
        let asset = Asset::Native(Coin {
            denom: "uatom".to_string(),
            amount: Uint128::new(100),
        });

        assert_eq!(asset.denom(), "uatom");
        assert_eq!(asset.amount(), Uint128::new(100));
    }

    #[test]
    fn test_asset_cw20() {
        let asset = Asset::Cw20(Cw20Coin {
            address: "asset".to_string(),
            amount: Uint128::new(100),
        });

        assert_eq!(asset.denom(), "asset");
        assert_eq!(asset.amount(), Uint128::new(100));
    }

    #[test]
    fn test_asset_transfer_full_native() {
        let asset = Asset::Native(Coin {
            denom: "uatom".to_string(),
            amount: Uint128::new(100),
        });

        let msg = asset.transfer_full("addr".to_string());

        match msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(to_address, "addr");
                assert_eq!(amount.len(), 1);
                assert_eq!(amount[0].denom, "uatom");
                assert_eq!(amount[0].amount, Uint128::new(100));
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_asset_transfer_full_cw20() {
        let asset = Asset::Cw20(Cw20Coin {
            address: "asset".to_string(),
            amount: Uint128::new(100),
        });

        let msg = asset.transfer_full("addr".to_string());

        match msg {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            }) => {
                assert_eq!(contract_addr, "asset");
                assert_eq!(
                    msg,
                    to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "addr".to_string(),
                        amount: Uint128::new(100),
                    })
                    .unwrap()
                );
                assert_eq!(funds.len(), 0);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_asset_transfer_partial_native() {
        let mut asset = Asset::Native(Coin {
            denom: "uatom".to_string(),
            amount: Uint128::new(100),
        });

        let msg = asset.transfer_partial("addr".to_string(), Uint128::new(20));

        match msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(to_address, "addr");
                assert_eq!(amount.len(), 1);
                assert_eq!(amount[0].denom, "uatom");
                assert_eq!(amount[0].amount, Uint128::new(20));
            }
            _ => panic!("Unexpected message type"),
        }

        assert_eq!(asset.amount(), Uint128::new(80));
    }

    #[test]
    fn test_asset_transfer_partial_cw20() {
        let mut asset = Asset::Cw20(Cw20Coin {
            address: "asset".to_string(),
            amount: Uint128::new(100),
        });

        let msg = asset.transfer_partial("addr".to_string(), Uint128::new(20));

        match msg {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            }) => {
                assert_eq!(contract_addr, "asset");
                assert_eq!(
                    msg,
                    to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "addr".to_string(),
                        amount: Uint128::new(20),
                    })
                    .unwrap()
                );
                assert_eq!(funds.len(), 0);
            }
            _ => panic!("Unexpected message type"),
        }

        assert_eq!(asset.amount(), Uint128::new(80));
    }

    #[test]
    fn test_validate_native() {
        let asset = Asset::Native(Coin {
            denom: "uatom".to_string(),
            amount: Uint128::new(100),
        });

        let mut deps = mock_dependencies_with_balances(&[("entry_point", &[])]);

        let env = mock_env();

        let info = mock_info(
            "sender",
            &[Coin {
                denom: "uatom".to_string(),
                amount: Uint128::new(100),
            }],
        );

        assert!(asset.validate(&deps.as_mut(), &env, &info).is_ok());
    }

    #[test]
    fn test_validate_cw20() {
        let asset = Asset::Cw20(Cw20Coin {
            address: "asset".to_string(),
            amount: Uint128::new(100),
        });

        // Create mock wasm handler to handle the cw20 balance query
        let wasm_handler = |query: &WasmQuery| -> QuerierResult {
            match query {
                WasmQuery::Smart { .. } => SystemResult::Ok(ContractResult::Ok(
                    to_binary(&BalanceResponse {
                        balance: Uint128::from(100u128),
                    })
                    .unwrap(),
                )),
                _ => panic!("Unsupported query: {:?}", query),
            }
        };

        let mut deps = mock_dependencies_with_balances(&[("entry_point", &[])]);

        deps.querier.update_wasm(wasm_handler);

        let env = mock_env();

        let info = mock_info("sender", &[]);

        assert!(asset.validate(&deps.as_mut(), &env, &info).is_ok());
    }
}
