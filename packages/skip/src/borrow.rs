use crate::{asset::Asset, error::SkipError};

use cosmwasm_std::Uint128;
use std::{convert::TryFrom, num::ParseIntError};

// use astroport::{asset::AssetInfo, router::SwapOperation as AstroportSwapOperation};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Api, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response};
use cw20::Cw20Contract;
use cw20::Cw20ReceiveMsg;

///////////////////
/// INSTANTIATE ///
///////////////////

// The MarsV1InstantiateMsg struct defines the initialization parameters for the
// Mars V1 rend bank adapter contract.
#[cw_serde]
pub struct MarsV1InstantiateMsg {
    pub entry_point_contract_address: String,
    pub red_bank_contract_address: String,
}

/////////////////////////
///      EXECUTE      ///
/////////////////////////

// The ExecuteMsg enum defines the execution message that the swap adapter contracts can handle.
// Only the Swap message is callable by external users.
#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    DepositAndBorrow { collateral_denom: String, borrow_denom: String, },
    TransferFundsBack { swapper: Addr, return_denom: String },
}

#[cw_serde]
pub enum Cw20HookMsg {
    DepositAndBorrow { collateral_denom: String, borrow_denom: String, },
}

/////////////////////////
///       QUERY       ///
/////////////////////////

// The QueryMsg enum defines the queries the lending adapter contracts provide.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // RedBankContractAddress returns the address of the red bank contract
    #[returns(Addr)]
    RedBankContractAddress {},
    // DepositAPR returns the rate of return on collateral deposit
    #[returns(Uint128)]
    DepositAPR {
        asset_denom: String,
    },
    // BorrowAPR returns the rate of interest on asset borrow
    #[returns(Uint128)]
    BorrowAPR {
        asset_denom: String,
    },
    // DepositAndBorrowAPR returns the net interest rate on collateral deposit and asset borrow
    #[returns(Uint128)]
    DepositAndBorrowAPR {
        deposit_asset_denom: String,
        borrow_asset_denom: String,
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

// Standard deposit and borrow operation type that contains the market, denom deposit,
// and denom borrow. The type is converted into the respective lending venues
// expected format in each adapter contract.
#[cw_serde]
pub struct DepositAndBorrowOperation {
    pub market: String,
    pub denom_deposit: String,
    pub denom_borrow: String,
}

// MARS V1 RED BANK CONVERSION

// Converts a deposit and borrow operation to a mars v1 deposit and borrow operation
impl DepositAndBorrowOperation {
    pub fn into_mars_v1_deposit_and_borrow_operation(
        &self,
        api: &dyn Api,
    ) -> MarsV1SwapOperation {
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

        MarsV1SwapOperation::AstroSwap {
            offer_asset_info,
            ask_asset_info,
        }
    }
}

// Converts a vector of skip swap operation to vector of osmosis swap
// amount in/out routes, returning an error if any of the swap operations
// fail to convert. This only happens if the given String for pool in the
// swap operation is not a valid u64, which is the pool_id type for Osmosis.
pub fn convert_swap_operations<T>(
    swap_operations: Vec<DepositAndBorrowOperation>,
) -> Result<Vec<T>, ParseIntError>
where
    T: TryFrom<DepositAndBorrowOperation, Error = ParseIntError>,
{
    swap_operations.into_iter().map(T::try_from).collect()
}

// Swap object to get the exact amount of a given asset with the given vector of swap operations
#[cw_serde]
pub struct SwapExactAssetOut {
    pub swap_venue_name: String,
    pub operations: Vec<DepositAndBorrowOperation>,
    pub refund_address: Option<String>,
}

// Swap object that swaps the remaining asset recevied
// from the contract call minus fee swap (if present)
#[cw_serde]
pub struct SwapExactAssetIn {
    pub swap_venue_name: String,
    pub operations: Vec<DepositAndBorrowOperation>,
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

// TODO: replace with validate_borrow_operations
// // Validates the swap operations
// pub fn validate_swap_operations(
//     swap_operations: &[SwapOperation],
//     asset_in_denom: &str,
//     asset_out_denom: &str,
// ) -> Result<(), SkipError> {
//     // Verify the swap operations are not empty
//     let (Some(first_op), Some(last_op)) = (swap_operations.first(), swap_operations.last()) else {
//         return Err(SkipError::SwapOperationsEmpty);
//     };

//     // Verify the first swap operation denom in is the same as the asset in denom
//     if first_op.denom_in != asset_in_denom {
//         return Err(SkipError::SwapOperationsAssetInDenomMismatch);
//     }

//     // Verify the last swap operation denom out is the same as the asset out denom
//     if last_op.denom_out != asset_out_denom {
//         return Err(SkipError::SwapOperationsAssetOutDenomMismatch);
//     }

//     Ok(())
// }

// TODO: add unit tests here
