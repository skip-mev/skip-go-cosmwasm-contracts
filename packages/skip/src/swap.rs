use crate::{asset::Asset, error::SkipError};

use std::{convert::TryFrom, num::ParseIntError};

use astroport::{asset::AssetInfo, router::SwapOperation as AstroportSwapOperation};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_binary, Addr, Api, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response, WasmMsg,
};
use cw20::Cw20ReceiveMsg;
use cw20::{Cw20Contract, Cw20ExecuteMsg};
use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    SwapAmountInRoute as OsmosisSwapAmountInRoute, SwapAmountOutRoute as OsmosisSwapAmountOutRoute,
};

///////////////////
/// INSTANTIATE ///
///////////////////

// The OsmosisInstantiateMsg struct defines the initialization parameters for the
// Osmosis Poolmanager swap adapter contract.
#[cw_serde]
pub struct OsmosisInstantiateMsg {
    pub entry_point_contract_address: String,
}

// TODO: Change to AstroportInstantiateMsg as part of restructuring

// The NeutronInstantiateMsg struct defines the initialization parameters for the
// Neutron Astroport swap adapter contract.
#[cw_serde]
pub struct NeutronInstantiateMsg {
    pub entry_point_contract_address: String,
    pub router_contract_address: String,
}

/////////////////////////
///      EXECUTE      ///
/////////////////////////

// The ExecuteMsg enum defines the execution message that the swap adapter contracts can handle.
// Only the Swap message is callable by external users.
#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    Swap {
        sent_asset: Asset,
        operations: Vec<SwapOperation>,
    },
    TransferFundsBack {
        swapper: Addr,
        return_denom: String,
    },
}

#[cw_serde]
pub enum Cw20HookMsg {
    Swap {
        sent_asset: Asset,
        operations: Vec<SwapOperation>,
    },
}

/////////////////////////
///       QUERY       ///
/////////////////////////

// The QueryMsg enum defines the queries the swap adapter contracts provide.
// RouterContractAddress is only implemented for Astroport swap adapter contracts.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // RouterContractAddress returns the address of the router contract
    #[returns(Addr)]
    RouterContractAddress {},
    // SimulateSwapExactAmountOut returns the coin in necessary to receive the specified coin out
    #[returns(Asset)]
    SimulateSwapExactAssetOut {
        asset_out: Asset,
        swap_operations: Vec<SwapOperation>,
    },
    // SimulateSwapExactAmountIn returns the coin out received from the specified coin in
    #[returns(Asset)]
    SimulateSwapExactAssetIn {
        asset_in: Asset,
        swap_operations: Vec<SwapOperation>,
    },
}

////////////////////
/// COMMON TYPES ///
////////////////////

// Swap venue object that contains the name of the swap venue and adapter contract address.
#[cw_serde]
pub struct SwapVenue {
    pub name: String,
    pub adapter_contract_address: String,
}

// Standard swap operation type that contains the pool, denom in, and denom out
// for the swap operation. The type is converted into the respective swap venues
// expected format in each adapter contract.
#[cw_serde]
pub struct SwapOperation {
    pub pool: String,
    pub denom_in: String,
    pub denom_out: String,
}

// ASTROPORT CONVERSION

// Converts a swap operation to an astroport swap operation
impl SwapOperation {
    pub fn into_astroport_swap_operation(&self, api: &dyn Api) -> AstroportSwapOperation {
        let offer_asset_info = match api.addr_validate(&self.denom_in) {
            Ok(contract_addr) => AssetInfo::Token { contract_addr },
            Err(_) => AssetInfo::NativeToken {
                denom: self.denom_in.clone(),
            },
        };

        let ask_asset_info = match api.addr_validate(&self.denom_out) {
            Ok(contract_addr) => AssetInfo::Token { contract_addr },
            Err(_) => AssetInfo::NativeToken {
                denom: self.denom_out.clone(),
            },
        };

        AstroportSwapOperation::AstroSwap {
            offer_asset_info,
            ask_asset_info,
        }
    }
}

// OSMOSIS CONVERSIONS

// Converts a skip swap operation to an osmosis swap amount in route
// Error if the given String for pool in the swap operation is not a valid u64.
impl TryFrom<SwapOperation> for OsmosisSwapAmountInRoute {
    type Error = ParseIntError;

    fn try_from(swap_operation: SwapOperation) -> Result<Self, Self::Error> {
        Ok(OsmosisSwapAmountInRoute {
            pool_id: swap_operation.pool.parse()?,
            token_out_denom: swap_operation.denom_out,
        })
    }
}

// Converts a skip swap operation to an osmosis swap amount out route
// Error if the given String for pool in the swap operation is not a valid u64.
impl TryFrom<SwapOperation> for OsmosisSwapAmountOutRoute {
    type Error = ParseIntError;

    fn try_from(swap_operation: SwapOperation) -> Result<Self, Self::Error> {
        Ok(OsmosisSwapAmountOutRoute {
            pool_id: swap_operation.pool.parse()?,
            token_in_denom: swap_operation.denom_in,
        })
    }
}

// Converts a vector of skip swap operation to vector of osmosis swap
// amount in/out routes, returning an error if any of the swap operations
// fail to convert. This only happens if the given String for pool in the
// swap operation is not a valid u64, which is the pool_id type for Osmosis.
pub fn convert_swap_operations<T>(
    swap_operations: Vec<SwapOperation>,
) -> Result<Vec<T>, ParseIntError>
where
    T: TryFrom<SwapOperation, Error = ParseIntError>,
{
    swap_operations.into_iter().map(T::try_from).collect()
}

// Swap object to get the exact amount of a given coin with the given vector of swap operations
#[cw_serde]
pub struct SwapExactCoinOut {
    pub swap_venue_name: String,
    pub operations: Vec<SwapOperation>,
    pub refund_address: Option<String>,
}

// Swap object that swaps the remaining coin recevied
// from the contract call minus fee swap (if present)
#[cw_serde]
pub struct SwapExactCoinIn {
    pub swap_venue_name: String,
    pub operations: Vec<SwapOperation>,
}

// Converts a SwapExactCoinOut used in the entry point contract
// to a swap adapter Swap execute message
impl SwapExactCoinOut {
    pub fn into_execute_msg(self, sent_asset: &Asset) -> ExecuteMsg {
        ExecuteMsg::Swap {
            sent_asset: sent_asset.clone(),
            operations: self.operations,
        }
    }
}

// Converts a SwapExactCoinIn used in the entry point contract
// to a swap adapter Swap execute message
impl SwapExactCoinIn {
    pub fn into_execute_msg(self, sent_asset: &Asset) -> ExecuteMsg {
        ExecuteMsg::Swap {
            sent_asset: sent_asset.clone(),
            operations: self.operations,
        }
    }
}

#[cw_serde]
pub enum Swap {
    SwapExactCoinIn(SwapExactCoinIn),
    SwapExactCoinOut(SwapExactCoinOut),
}

////////////////////////
/// COMMON FUNCTIONS ///
////////////////////////

// Query the contract's balance and transfer the funds back to the swapper
// @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
pub fn execute_transfer_funds_back(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    swapper: Addr,
    return_denom: String,
) -> Result<Response, SkipError> {
    // Ensure the caller is the contract itself
    if info.sender != env.contract.address {
        return Err(SkipError::Unauthorized);
    }

    // Create the transfer funds back message
    let transfer_funds_back_msg: CosmosMsg = match deps.api.addr_validate(&return_denom) {
        Ok(contract_addr) => {
            let cw20_contract = Cw20Contract(contract_addr.clone());

            let balance = cw20_contract.balance(&deps.querier, &env.contract.address)?;

            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: swapper.to_string(),
                    amount: balance,
                })?,
                funds: vec![],
            })
        }
        Err(_) => CosmosMsg::Bank(BankMsg::Send {
            to_address: swapper.to_string(),
            amount: deps.querier.query_all_balances(env.contract.address)?,
        }),
    };

    Ok(Response::new()
        .add_message(transfer_funds_back_msg)
        .add_attribute("action", "dispatch_transfer_funds_back_bank_send"))
}

// Validates the swap operations
pub fn validate_swap_operations(
    swap_operations: &[SwapOperation],
    coin_in_denom: &str,
    coin_out_denom: &str,
) -> Result<(), SkipError> {
    // Verify the swap operations are not empty
    let (Some(first_op), Some(last_op)) = (swap_operations.first(), swap_operations.last()) else {
        return Err(SkipError::SwapOperationsEmpty);
    };

    // Verify the first swap operation denom in is the same as the coin in denom
    if first_op.denom_in != coin_in_denom {
        return Err(SkipError::SwapOperationsCoinInDenomMismatch);
    }

    // Verify the last swap operation denom out is the same as the coin out denom
    if last_op.denom_out != coin_out_denom {
        return Err(SkipError::SwapOperationsCoinOutDenomMismatch);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn test_from_swap_operation_to_astropot_swap_operation() {
        // TEST CASE 1: Native Swap Operation
        let swap_operation = SwapOperation {
            pool: "1".to_string(),
            denom_in: "ua".to_string(),
            denom_out: "uo".to_string(),
        };

        let deps = mock_dependencies();

        let astroport_swap_operation: AstroportSwapOperation =
            swap_operation.into_astroport_swap_operation(&deps.api);

        assert_eq!(
            astroport_swap_operation,
            AstroportSwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "ua".to_string()
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uo".to_string()
                }
            }
        );

        // TEST CASE 2: CW20 Swap Operation
        let swap_operation = SwapOperation {
            pool: "1".to_string(),
            denom_in: "cwabc".to_string(),
            denom_out: "cw123".to_string(),
        };

        let deps = mock_dependencies();

        let astroport_swap_operation: AstroportSwapOperation =
            swap_operation.into_astroport_swap_operation(&deps.api);

        assert_eq!(
            astroport_swap_operation,
            AstroportSwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("cwabc")
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("cw123")
                }
            }
        );
    }

    #[test]
    fn test_from_swap_operation_to_osmosis_swap_amount_in_route() {
        // TEST CASE 1: Valid Swap Operation
        let swap_operation = SwapOperation {
            pool: "1".to_string(),
            denom_in: "uatom".to_string(),
            denom_out: "uosmo".to_string(),
        };

        let osmosis_swap_amount_in_route: OsmosisSwapAmountInRoute =
            swap_operation.try_into().unwrap();

        assert_eq!(
            osmosis_swap_amount_in_route,
            OsmosisSwapAmountInRoute {
                pool_id: 1,
                token_out_denom: "uosmo".to_string()
            }
        );

        // TEST CASE 2: Invalid Pool ID
        let swap_operation = SwapOperation {
            pool: "invalid".to_string(),
            denom_in: "uatom".to_string(),
            denom_out: "uosmo".to_string(),
        };

        let result: Result<OsmosisSwapAmountInRoute, ParseIntError> = swap_operation.try_into();

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "invalid digit found in string"
        );
    }

    #[test]
    fn test_from_swap_operation_to_osmosis_swap_amount_out_route() {
        // TEST CASE 1: Valid Swap Operation
        let swap_operation = SwapOperation {
            pool: "1".to_string(),
            denom_in: "uatom".to_string(),
            denom_out: "uosmo".to_string(),
        };

        let osmosis_swap_amount_out_route: OsmosisSwapAmountOutRoute =
            swap_operation.try_into().unwrap();

        assert_eq!(
            osmosis_swap_amount_out_route,
            OsmosisSwapAmountOutRoute {
                pool_id: 1,
                token_in_denom: "uatom".to_string()
            }
        );

        // TEST CASE 2: Invalid Pool ID
        let swap_operation = SwapOperation {
            pool: "invalid".to_string(),
            denom_in: "uatom".to_string(),
            denom_out: "uosmo".to_string(),
        };

        let result: Result<OsmosisSwapAmountOutRoute, ParseIntError> = swap_operation.try_into();

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "invalid digit found in string"
        );
    }

    #[test]
    fn test_convert_swap_operations() {
        // TEST CASE 1: Valid Swap Operations
        let swap_operations = vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "uatom".to_string(),
                denom_out: "uosmo".to_string(),
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "uosmo".to_string(),
                denom_out: "untrn".to_string(),
            },
        ];

        let result: Result<Vec<OsmosisSwapAmountInRoute>, ParseIntError> =
            convert_swap_operations(swap_operations.clone());

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            vec![
                OsmosisSwapAmountInRoute {
                    pool_id: 1,
                    token_out_denom: "uosmo".to_string()
                },
                OsmosisSwapAmountInRoute {
                    pool_id: 2,
                    token_out_denom: "untrn".to_string()
                }
            ]
        );

        let result: Result<Vec<OsmosisSwapAmountOutRoute>, ParseIntError> =
            convert_swap_operations(swap_operations);

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            vec![
                OsmosisSwapAmountOutRoute {
                    pool_id: 1,
                    token_in_denom: "uatom".to_string()
                },
                OsmosisSwapAmountOutRoute {
                    pool_id: 2,
                    token_in_denom: "uosmo".to_string()
                }
            ]
        );

        // TEST CASE 2: Invalid Pool ID
        let swap_operations = vec![
            SwapOperation {
                pool: "invalid".to_string(),
                denom_in: "uatom".to_string(),
                denom_out: "uosmo".to_string(),
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "uosmo".to_string(),
                denom_out: "untrn".to_string(),
            },
        ];

        let result: Result<Vec<OsmosisSwapAmountInRoute>, ParseIntError> =
            convert_swap_operations(swap_operations.clone());

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "invalid digit found in string"
        );

        let result: Result<Vec<OsmosisSwapAmountOutRoute>, ParseIntError> =
            convert_swap_operations(swap_operations);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "invalid digit found in string"
        );
    }

    #[test]
    fn test_validate_swap_operations() {
        // TEST CASE 1: Valid Swap Operations
        let swap_operations = vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "uatom".to_string(),
                denom_out: "uosmo".to_string(),
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "uosmo".to_string(),
                denom_out: "untrn".to_string(),
            },
        ];

        let coin_in_denom = "uatom";
        let coin_out_denom = "untrn";

        let result = validate_swap_operations(&swap_operations, coin_in_denom, coin_out_denom);

        assert!(result.is_ok());

        // TEST CASE 2: Empty Swap Operations
        let swap_operations: Vec<SwapOperation> = vec![];

        let coin_in_denom = "uatom";
        let coin_out_denom = "untrn";

        let result = validate_swap_operations(&swap_operations, coin_in_denom, coin_out_denom);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SkipError::SwapOperationsEmpty);

        // TEST CASE 3: First Swap Operation Denom In Mismatch
        let swap_operations = vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "uosmo".to_string(),
                denom_out: "uatom".to_string(),
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "uatom".to_string(),
                denom_out: "untrn".to_string(),
            },
        ];

        let coin_in_denom = "uatom";
        let coin_out_denom = "untrn";

        let result = validate_swap_operations(&swap_operations, coin_in_denom, coin_out_denom);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            SkipError::SwapOperationsCoinInDenomMismatch
        );

        // TEST CASE 4: Last Swap Operation Denom Out Mismatch
        let swap_operations = vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "uatom".to_string(),
                denom_out: "uosmo".to_string(),
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "uosmo".to_string(),
                denom_out: "uatom".to_string(),
            },
        ];

        let coin_in_denom = "uatom";
        let coin_out_denom = "untrn";

        let result = validate_swap_operations(&swap_operations, coin_in_denom, coin_out_denom);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            SkipError::SwapOperationsCoinOutDenomMismatch
        );
    }
}
