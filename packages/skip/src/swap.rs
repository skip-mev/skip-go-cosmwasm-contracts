use crate::{asset::Asset, error::SkipError};

use std::{convert::TryFrom, num::ParseIntError};

use astroport::{asset::AssetInfo, router::SwapOperation as AstroportSwapOperation};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    Addr, Api, BankMsg, Binary, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, Uint128,
};
use cw20::Cw20Contract;
use cw20::Cw20ReceiveMsg;
use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    SwapAmountInRoute as OsmosisSwapAmountInRoute, SwapAmountOutRoute as OsmosisSwapAmountOutRoute,
};

///////////////
/// MIGRATE ///
///////////////

// The MigrateMsg struct defines the migration parameters used.
#[cw_serde]
pub struct MigrateMsg {
    pub entry_point_contract_address: String,
}

///////////////////
/// INSTANTIATE ///
///////////////////

// The InstantiateMsg struct defines the initialization parameters for the
// Osmosis Poolmanager and Astroport swap adapter contracts.
#[cw_serde]
pub struct InstantiateMsg {
    pub entry_point_contract_address: String,
}

#[cw_serde]
pub struct DexterAdapterInstantiateMsg {
    pub entry_point_contract_address: String,
    pub dexter_vault_contract_address: String,
    pub dexter_router_contract_address: String,
}

#[cw_serde]
pub struct DropBondInstantiateMsg {
    pub entry_point_contract_address: String,
    pub drop_factory_contract_address: String,
}

#[cw_serde]
pub struct LidoSatelliteInstantiateMsg {
    pub entry_point_contract_address: String,
    pub lido_satellite_contract_address: String,
}

#[cw_serde]
pub struct HallswapInstantiateMsg {
    pub entry_point_contract_address: String,
    pub hallswap_contract_address: String,
}

/////////////////////////
///      EXECUTE      ///
/////////////////////////

// The ExecuteMsg enum defines the execution message that the swap adapter contracts can handle.
// Only the Swap message is callable by external users.
#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    Swap { operations: Vec<SwapOperation> },
    TransferFundsBack { swapper: Addr, return_denom: String },
    AstroportPoolSwap { operation: SwapOperation }, // Only used for the astroport swap adapter contract
    WhiteWhalePoolSwap { operation: SwapOperation }, // Only used for the white whale swap adapter contract
}

#[cw_serde]
pub enum Cw20HookMsg {
    Swap { operations: Vec<SwapOperation> },
}

/////////////////////////
///       QUERY       ///
/////////////////////////

// The QueryMsg enum defines the queries the swap adapter contracts provide.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // SimulateSwapExactAssetOut returns the asset in necessary to receive the specified asset out
    #[returns(Asset)]
    SimulateSwapExactAssetOut {
        asset_out: Asset,
        swap_operations: Vec<SwapOperation>,
    },
    // SimulateSwapExactAssetIn returns the asset out received from the specified asset in
    #[returns(Asset)]
    SimulateSwapExactAssetIn {
        asset_in: Asset,
        swap_operations: Vec<SwapOperation>,
    },
    // SimulateSwapExactAssetOutWithSpotPrice returns the asset in necessary to receive the specified asset out with metadata
    #[returns(SimulateSwapExactAssetOutResponse)]
    SimulateSwapExactAssetOutWithMetadata {
        asset_out: Asset,
        swap_operations: Vec<SwapOperation>,
        include_spot_price: bool,
    },
    // SimulateSwapExactAssetInWithSpotPrice returns the asset out received from the specified asset in with metadata
    #[returns(SimulateSwapExactAssetInResponse)]
    SimulateSwapExactAssetInWithMetadata {
        asset_in: Asset,
        swap_operations: Vec<SwapOperation>,
        include_spot_price: bool,
    },
    // SimulateSmartSwapExactAssetIn returns the asset out received from the specified asset in over multiple routes
    #[returns(Asset)]
    SimulateSmartSwapExactAssetIn { asset_in: Asset, routes: Vec<Route> },
    // SimulateSmartSwapExactAssetInWithMetadata returns the asset out received from the specified asset in over multiple routes with metadata
    #[returns(SimulateSmartSwapExactAssetInResponse)]
    SimulateSmartSwapExactAssetInWithMetadata {
        asset_in: Asset,
        routes: Vec<Route>,
        include_spot_price: bool,
    },
}

// The SimulateSwapExactAssetInResponse struct defines the response for the
// SimulateSwapExactAssetIn query.
#[cw_serde]
pub struct SimulateSwapExactAssetInResponse {
    pub asset_out: Asset,
    pub spot_price: Option<Decimal>,
}

// The SimulateSwapExactAssetOutResponse struct defines the response for the
// SimulateSwapExactAssetOut query.
#[cw_serde]
pub struct SimulateSwapExactAssetOutResponse {
    pub asset_in: Asset,
    pub spot_price: Option<Decimal>,
}

// The SimulateSmartSwapExactAssetInResponse struct defines the response for the
// SimulateSmartSwapExactAssetIn query.
#[cw_serde]
pub struct SimulateSmartSwapExactAssetInResponse {
    pub asset_out: Asset,
    pub spot_price: Option<Decimal>,
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

#[cw_serde]
pub struct Route {
    pub offer_asset: Asset,
    pub operations: Vec<SwapOperation>,
}

impl Route {
    pub fn ask_denom(&self) -> Result<String, SkipError> {
        match self.operations.last() {
            Some(op) => Ok(op.denom_out.clone()),
            None => Err(SkipError::SwapOperationsEmpty),
        }
    }
}

pub fn get_ask_denom_for_routes(routes: &[Route]) -> Result<String, SkipError> {
    match routes.last() {
        Some(route) => route.ask_denom(),
        None => Err(SkipError::RoutesEmpty),
    }
}

// Standard swap operation type that contains the pool, denom in, and denom out
// for the swap operation. The type is converted into the respective swap venues
// expected format in each adapter contract.
#[cw_serde]
pub struct SwapOperation {
    pub pool: String,
    pub denom_in: String,
    pub denom_out: String,
    pub interface: Option<Binary>,
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

// Swap object to get the exact amount of a given asset with the given vector of swap operations
#[cw_serde]
pub struct SwapExactAssetOut {
    pub swap_venue_name: String,
    pub operations: Vec<SwapOperation>,
    pub refund_address: Option<String>,
}

// Swap object that swaps the remaining asset recevied
// from the contract call minus fee swap (if present)
#[cw_serde]
pub struct SwapExactAssetIn {
    pub swap_venue_name: String,
    pub operations: Vec<SwapOperation>,
}

// Swap object that swaps the remaining asset recevied
// over multiple routes from the contract call minus fee swap (if present)
#[cw_serde]
pub struct SmartSwapExactAssetIn {
    pub swap_venue_name: String,
    pub routes: Vec<Route>,
}

impl SmartSwapExactAssetIn {
    pub fn amount(&self) -> Uint128 {
        self.routes
            .iter()
            .map(|route| route.offer_asset.amount())
            .sum()
    }

    pub fn ask_denom(&self) -> Result<String, SkipError> {
        match self.routes.last() {
            Some(route) => route.ask_denom(),
            None => Err(SkipError::RoutesEmpty),
        }
    }

    pub fn largest_route_index(&self) -> Result<usize, SkipError> {
        match self
            .routes
            .iter()
            .enumerate()
            .max_by_key(|(_, route)| route.offer_asset.amount())
            .map(|(index, _)| index)
        {
            Some(idx) => Ok(idx),
            None => Err(SkipError::RoutesEmpty),
        }
    }
}

// Converts a SwapExactAssetOut used in the entry point contract
// to a swap adapter Swap execute message
impl From<SwapExactAssetOut> for ExecuteMsg {
    fn from(swap: SwapExactAssetOut) -> Self {
        ExecuteMsg::Swap {
            operations: swap.operations,
        }
    }
}

// Converts a SwapExactAssetIn used in the entry point contract
// to a swap adapter Swap execute message
impl From<SwapExactAssetIn> for ExecuteMsg {
    fn from(swap: SwapExactAssetIn) -> Self {
        ExecuteMsg::Swap {
            operations: swap.operations,
        }
    }
}

#[cw_serde]
pub enum Swap {
    SwapExactAssetIn(SwapExactAssetIn),
    SwapExactAssetOut(SwapExactAssetOut),
    SmartSwapExactAssetIn(SmartSwapExactAssetIn),
}

////////////////////////
/// COMMON FUNCTIONS ///
////////////////////////

// Query the contract's balance and transfer the funds back to the swapper
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
        Ok(contract_addr) => Asset::new(
            deps.api,
            contract_addr.as_str(),
            Cw20Contract(contract_addr.clone()).balance(&deps.querier, &env.contract.address)?,
        )
        .transfer(swapper.as_str()),
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
    asset_in_denom: &str,
    asset_out_denom: &str,
) -> Result<(), SkipError> {
    // Verify the swap operations are not empty
    let (Some(first_op), Some(last_op)) = (swap_operations.first(), swap_operations.last()) else {
        return Err(SkipError::SwapOperationsEmpty);
    };

    // Verify the first swap operation denom in is the same as the asset in denom
    if first_op.denom_in != asset_in_denom {
        return Err(SkipError::SwapOperationsAssetInDenomMismatch);
    }

    // Verify the last swap operation denom out is the same as the asset out denom
    if last_op.denom_out != asset_out_denom {
        return Err(SkipError::SwapOperationsAssetOutDenomMismatch);
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
            interface: None,
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
            interface: None,
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
            interface: None,
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
            interface: None,
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
            interface: None,
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
            interface: None,
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
                interface: None,
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "uosmo".to_string(),
                denom_out: "untrn".to_string(),
                interface: None,
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
                interface: None,
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "uosmo".to_string(),
                denom_out: "untrn".to_string(),
                interface: None,
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
                interface: None,
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "uosmo".to_string(),
                denom_out: "untrn".to_string(),
                interface: None,
            },
        ];

        let asset_in_denom = "uatom";
        let asset_out_denom = "untrn";

        let result = validate_swap_operations(&swap_operations, asset_in_denom, asset_out_denom);

        assert!(result.is_ok());

        // TEST CASE 2: Empty Swap Operations
        let swap_operations: Vec<SwapOperation> = vec![];

        let asset_in_denom = "uatom";
        let asset_out_denom = "untrn";

        let result = validate_swap_operations(&swap_operations, asset_in_denom, asset_out_denom);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SkipError::SwapOperationsEmpty);

        // TEST CASE 3: First Swap Operation Denom In Mismatch
        let swap_operations = vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "uosmo".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "uatom".to_string(),
                denom_out: "untrn".to_string(),
                interface: None,
            },
        ];

        let asset_in_denom = "uatom";
        let asset_out_denom = "untrn";

        let result = validate_swap_operations(&swap_operations, asset_in_denom, asset_out_denom);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            SkipError::SwapOperationsAssetInDenomMismatch
        );

        // TEST CASE 4: Last Swap Operation Denom Out Mismatch
        let swap_operations = vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "uatom".to_string(),
                denom_out: "uosmo".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "uosmo".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            },
        ];

        let asset_in_denom = "uatom";
        let asset_out_denom = "untrn";

        let result = validate_swap_operations(&swap_operations, asset_in_denom, asset_out_denom);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            SkipError::SwapOperationsAssetOutDenomMismatch
        );
    }
}
