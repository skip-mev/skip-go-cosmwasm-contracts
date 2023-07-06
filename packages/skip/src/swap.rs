use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin};
use std::{
    convert::{From, TryFrom},
    num::ParseIntError,
};

use astroport::{asset::AssetInfo, router::SwapOperation as AstroportSwapOperation};

use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    SwapAmountInRoute as OsmosisSwapAmountInRoute, SwapAmountOutRoute as OsmosisSwapAmountOutRoute,
};

///////////////////
/// INSTANTIATE ///
///////////////////

#[cw_serde]
pub struct OsmosisInstantiateMsg {}

#[cw_serde]
pub struct NeutronInstantiateMsg {
    pub router_contract_address: String,
}

/////////////////////////
///      EXECUTE      ///
/////////////////////////

#[cw_serde]
pub enum ExecuteMsg {
    Swap { operations: Vec<SwapOperation> },
    TransferFundsBack { swapper: Addr },
}

impl From<SwapExactCoinIn> for ExecuteMsg {
    fn from(swap: SwapExactCoinIn) -> Self {
        ExecuteMsg::Swap {
            operations: swap.operations,
        }
    }
}

impl From<SwapExactCoinOut> for ExecuteMsg {
    fn from(swap: SwapExactCoinOut) -> Self {
        ExecuteMsg::Swap {
            operations: swap.operations,
        }
    }
}

/////////////////////////
///       QUERY       ///
/////////////////////////

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // RouterContractAddress returns the address of the router contract
    #[returns(Addr)]
    RouterContractAddress {},
    // SimulateSwapExactAmountOut returns the coin in necessary to receive the specified coin out
    #[returns(Coin)]
    SimulateSwapExactCoinOut {
        coin_out: Coin,
        swap_operations: Vec<SwapOperation>,
    },
    // SimulateSwapExactAmountIn returns the coin out received from the specified coin in
    #[returns(Coin)]
    SimulateSwapExactCoinIn {
        coin_in: Coin,
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

// ASTROPORT CONVERSIONS

// Converts a skip swap operation to an astroport swap operation
impl From<SwapOperation> for AstroportSwapOperation {
    fn from(swap_operation: SwapOperation) -> Self {
        // Convert the swap operation to an astroport swap operation and return it
        AstroportSwapOperation::AstroSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: swap_operation.denom_in,
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: swap_operation.denom_out,
            },
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
    pub coin_out: Coin,
    pub operations: Vec<SwapOperation>,
}

// Swap object that swaps the given coin in when present. When not present,
// swaps the remaining coin recevied from the contract call minus fee swap (if present)
#[cw_serde]
pub struct SwapExactCoinIn {
    pub swap_venue_name: String,
    pub coin_in: Option<Coin>,
    pub operations: Vec<SwapOperation>,
}
