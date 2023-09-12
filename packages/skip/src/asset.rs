use crate::error::SkipError;
use astroport::asset::{Asset as AstroportAsset, AssetInfo};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Api, BankMsg, Binary, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Uint128, WasmMsg,
};
use cw20::{Cw20Coin, Cw20CoinVerified, Cw20Contract, Cw20ExecuteMsg};
use cw_utils::{nonpayable, one_coin};

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
    pub fn new(api: &dyn Api, denom: &str, amount: Uint128) -> Self {
        match api.addr_validate(denom) {
            Ok(addr) => Asset::Cw20(Cw20Coin {
                address: addr.to_string(),
                amount,
            }),
            Err(_) => Asset::Native(Coin {
                denom: denom.to_string(),
                amount,
            }),
        }
    }

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

    pub fn add(&mut self, amount: Uint128) -> Result<Uint128, SkipError> {
        match self {
            Asset::Native(coin) => {
                coin.amount = coin.amount.checked_add(amount)?;
                Ok(coin.amount)
            }
            Asset::Cw20(coin) => {
                coin.amount = coin.amount.checked_add(amount)?;
                Ok(coin.amount)
            }
        }
    }

    pub fn sub(&mut self, amount: Uint128) -> Result<Uint128, SkipError> {
        match self {
            Asset::Native(coin) => {
                coin.amount = coin.amount.checked_sub(amount)?;
                Ok(coin.amount)
            }
            Asset::Cw20(coin) => {
                coin.amount = coin.amount.checked_sub(amount)?;
                Ok(coin.amount)
            }
        }
    }

    pub fn transfer(self, to_address: &str) -> CosmosMsg {
        match self {
            Asset::Native(coin) => CosmosMsg::Bank(BankMsg::Send {
                to_address: to_address.to_string(),
                amount: vec![coin],
            }),
            Asset::Cw20(coin) => CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: coin.address.clone(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: to_address.to_string(),
                    amount: coin.amount,
                })
                .unwrap(),
                funds: vec![],
            }),
        }
    }

    pub fn into_wasm_msg(self, contract_addr: String, msg: Binary) -> Result<WasmMsg, SkipError> {
        match self {
            Asset::Native(coin) => Ok(WasmMsg::Execute {
                contract_addr,
                msg,
                funds: vec![coin],
            }),
            Asset::Cw20(coin) => Ok(WasmMsg::Execute {
                contract_addr: coin.address,
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: contract_addr,
                    amount: coin.amount,
                    msg,
                })?,
                funds: vec![],
            }),
        }
    }

    pub fn into_astroport_asset(&self, api: &dyn Api) -> Result<AstroportAsset, SkipError> {
        match self {
            Asset::Native(coin) => Ok(AstroportAsset {
                info: AssetInfo::NativeToken {
                    denom: coin.denom.clone(),
                },
                amount: coin.amount,
            }),
            Asset::Cw20(cw20_coin) => Ok(AstroportAsset {
                info: AssetInfo::Token {
                    contract_addr: api.addr_validate(&cw20_coin.address)?,
                },
                amount: cw20_coin.amount,
            }),
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

                let cw20_contract = Cw20Contract(verified_cw20_coin_addr);

                let balance = cw20_contract.balance(&deps.querier, &env.contract.address)?;

                if coin.amount <= balance {
                    Ok(())
                } else {
                    Err(SkipError::InvalidCw20Coin)
                }
            }
        }
    }
}

pub fn get_current_asset_available(
    deps: &DepsMut,
    env: &Env,
    denom: &str,
) -> Result<Asset, SkipError> {
    match deps.api.addr_validate(denom) {
        Ok(addr) => {
            let cw20_contract = Cw20Contract(addr.clone());

            let amount = cw20_contract.balance(&deps.querier, &env.contract.address)?;

            Ok(Asset::Cw20(Cw20Coin {
                address: addr.to_string(),
                amount,
            }))
        }
        Err(_) => {
            let coin = deps.querier.query_balance(&env.contract.address, denom)?;

            Ok(Asset::Native(coin))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{
        testing::{mock_dependencies_with_balances, mock_env, mock_info},
        Addr, ContractResult, QuerierResult, SystemResult, WasmQuery,
    };
    use cw20::BalanceResponse;
    use cw_utils::PaymentError;

    #[test]
    fn test_new() {
        // TEST 1: Native asset
        let mut deps = mock_dependencies_with_balances(&[("entry_point", &[])]);

        let asset = Asset::new(deps.as_mut().api, "ua", Uint128::new(100));

        assert_eq!(
            asset,
            Asset::Native(Coin {
                denom: "ua".to_string(),
                amount: Uint128::new(100),
            })
        );

        // TEST 2: Cw20 asset
        let mut deps = mock_dependencies_with_balances(&[("entry_point", &[])]);

        let asset = Asset::new(deps.as_mut().api, "asset", Uint128::new(100));

        assert_eq!(
            asset,
            Asset::Cw20(Cw20Coin {
                address: "asset".to_string(),
                amount: Uint128::new(100),
            })
        );
    }

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
    fn test_add() {
        // TEST 1: Native asset
        let mut asset = Asset::Native(Coin {
            denom: "uatom".to_string(),
            amount: Uint128::new(100),
        });

        asset.add(Uint128::new(20)).unwrap();

        assert_eq!(asset.amount(), Uint128::new(120));

        // TEST 2: Cw20 asset
        let mut asset = Asset::Cw20(Cw20Coin {
            address: "asset".to_string(),
            amount: Uint128::new(100),
        });

        asset.add(Uint128::new(20)).unwrap();

        assert_eq!(asset.amount(), Uint128::new(120));
    }

    #[test]
    fn test_sub() {
        // TEST 1: Native asset
        let mut asset = Asset::Native(Coin {
            denom: "uatom".to_string(),
            amount: Uint128::new(100),
        });

        asset.sub(Uint128::new(20)).unwrap();

        assert_eq!(asset.amount(), Uint128::new(80));

        // TEST 2: Cw20 asset
        let mut asset = Asset::Cw20(Cw20Coin {
            address: "asset".to_string(),
            amount: Uint128::new(100),
        });

        asset.sub(Uint128::new(20)).unwrap();

        assert_eq!(asset.amount(), Uint128::new(80));
    }

    #[test]
    fn test_asset_transfer_native() {
        let asset = Asset::Native(Coin {
            denom: "uatom".to_string(),
            amount: Uint128::new(100),
        });

        let msg = asset.transfer("addr");

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
    fn test_asset_transfer_cw20() {
        let asset = Asset::Cw20(Cw20Coin {
            address: "asset".to_string(),
            amount: Uint128::new(100),
        });

        let msg = asset.transfer("addr");

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
    fn test_into_astroport_asset() {
        // TEST 1: Native asset
        let mut deps = mock_dependencies_with_balances(&[("entry_point", &[])]);

        let asset = Asset::Native(Coin {
            denom: "uatom".to_string(),
            amount: Uint128::new(100),
        });

        let astroport_asset = asset.into_astroport_asset(deps.as_mut().api).unwrap();

        assert_eq!(
            astroport_asset,
            AstroportAsset {
                info: AssetInfo::NativeToken {
                    denom: "uatom".to_string()
                },
                amount: Uint128::new(100),
            }
        );

        // TEST 2: Cw20 asset
        let mut deps = mock_dependencies_with_balances(&[("entry_point", &[])]);

        let asset = Asset::Cw20(Cw20Coin {
            address: "asset".to_string(),
            amount: Uint128::new(100),
        });

        let astroport_asset = asset.into_astroport_asset(deps.as_mut().api).unwrap();

        assert_eq!(
            astroport_asset,
            AstroportAsset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("asset")
                },
                amount: Uint128::new(100),
            }
        );
    }

    #[test]
    fn test_validate_native() {
        // TEST 1: Valid asset
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

        // TEST 2: Invalid asset due to less amount of denom sent
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
                amount: Uint128::new(50),
            }],
        );

        let res = asset.validate(&deps.as_mut(), &env, &info);

        assert_eq!(res, Err(SkipError::InvalidNativeCoin));

        // TEST 3: Invalid asset due to more than one coin sent
        let asset = Asset::Native(Coin {
            denom: "uatom".to_string(),
            amount: Uint128::new(100),
        });

        let mut deps = mock_dependencies_with_balances(&[("entry_point", &[])]);

        let env = mock_env();

        let info = mock_info(
            "sender",
            &[
                Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::new(100),
                },
                Coin {
                    denom: "uosmo".to_string(),
                    amount: Uint128::new(50),
                },
            ],
        );

        let res = asset.validate(&deps.as_mut(), &env, &info);

        assert_eq!(
            res,
            Err(SkipError::Payment(PaymentError::MultipleDenoms {}))
        );
    }

    #[test]
    fn test_validate_cw20() {
        // TEST 1: Valid asset
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

        // TEST 2: Invalid asset due to native coin sent in info
        let asset = Asset::Cw20(Cw20Coin {
            address: "asset".to_string(),
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

        let res = asset.validate(&deps.as_mut(), &env, &info);

        assert_eq!(res, Err(SkipError::Payment(PaymentError::NonPayable {})));

        // TEST 3: Invalid asset due to invalid cw20 balance
        let asset = Asset::Cw20(Cw20Coin {
            address: "asset".to_string(),
            amount: Uint128::new(100),
        });

        // Create mock wasm handler to handle the cw20 balance query
        let wasm_handler = |query: &WasmQuery| -> QuerierResult {
            match query {
                WasmQuery::Smart { .. } => SystemResult::Ok(ContractResult::Ok(
                    to_binary(&BalanceResponse {
                        balance: Uint128::from(50u128),
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

        let res = asset.validate(&deps.as_mut(), &env, &info);

        assert_eq!(res, Err(SkipError::InvalidCw20Coin));
    }

    #[test]
    fn test_get_current_asset_available() {
        // TEST 1: Native asset
        let mut deps = mock_dependencies_with_balances(&[("entry_point", &[Coin::new(100, "ua")])]);

        let mut env = mock_env();
        env.contract.address = Addr::unchecked("entry_point");

        let asset = get_current_asset_available(&deps.as_mut(), &env, "ua").unwrap();

        assert_eq!(
            asset,
            Asset::Native(Coin {
                denom: "ua".to_string(),
                amount: Uint128::new(100),
            })
        );

        // TEST 2: Cw20 asset
        let mut deps = mock_dependencies_with_balances(&[("entry_point", &[])]);

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

        deps.querier.update_wasm(wasm_handler);

        let env = mock_env();

        let asset = get_current_asset_available(&deps.as_mut(), &env, "asset").unwrap();

        assert_eq!(
            asset,
            Asset::Cw20(Cw20Coin {
                address: "asset".to_string(),
                amount: Uint128::new(100),
            })
        );
    }
}
