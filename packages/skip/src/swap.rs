use crate::error::SkipError;

use std::{
    convert::{From, TryFrom},
    num::ParseIntError,
};

use astroport::{asset::AssetInfo, router::SwapOperation as AstroportSwapOperation};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, BankMsg, Coin, DepsMut, Env, MessageInfo, Response};
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
    Swap { operations: Vec<SwapOperation> },
    TransferFundsBack { swapper: Addr },
}

// Converts a SwapExactCoinIn used in the entry point contract
// to a swap adapter Swap execute message
impl From<SwapExactCoinIn> for ExecuteMsg {
    fn from(swap: SwapExactCoinIn) -> Self {
        ExecuteMsg::Swap {
            operations: swap.operations,
        }
    }
}

// Converts a SwapExactCoinOut used in the entry point contract
// to a swap adapter Swap execute message
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

// The QueryMsg enum defines the queries the swap adapter contracts provide.
// RouterContractAddress is only implemented for Astroport swap adapter contracts.
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

#[cw_serde]
pub enum Swap {
    SwapExactCoinIn(SwapExactCoinIn),
    SwapExactCoinOut(SwapExactCoinOut),
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
) -> Result<Response, SkipError> {
    // Ensure the caller is the contract itself
    if info.sender != env.contract.address {
        return Err(SkipError::Unauthorized);
    }

    // Create the bank message send to transfer the contract funds back to the caller
    let transfer_funds_back_msg = BankMsg::Send {
        to_address: swapper.to_string(),
        amount: deps.querier.query_all_balances(env.contract.address)?,
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
