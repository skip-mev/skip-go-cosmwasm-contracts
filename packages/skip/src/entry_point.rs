use crate::{
    asset::Asset,
    ibc::IbcInfo,
    swap::{Swap, SwapExactAssetOut, SwapVenue},
};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Uint128};
use cw20::Cw20ReceiveMsg;

///////////////
/// MIGRATE ///
///////////////

// The MigrateMsg struct defines the migration parameters for the entry point contract.
#[cw_serde]
pub struct MigrateMsg {}

///////////////////
/// INSTANTIATE ///
///////////////////

// The InstantiateMsg struct defines the initialization parameters for the entry point contract.
#[cw_serde]
pub struct InstantiateMsg {
    pub swap_venues: Vec<SwapVenue>,
    pub ibc_transfer_contract_address: String,
}

///////////////
/// EXECUTE ///
///////////////

// The ExecuteMsg enum defines the execution messages that the entry point contract can handle.
// Only the SwapAndAction message is callable by external users.
#[cw_serde]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    SwapAndActionWithRecover {
        sent_asset: Option<Asset>,
        user_swap: Swap,
        min_asset: Asset,
        timeout_timestamp: u64,
        post_swap_action: Action,
        affiliates: Vec<Affiliate>,
        recovery_addr: Addr,
    },
    SwapAndAction {
        sent_asset: Option<Asset>,
        user_swap: Swap,
        min_asset: Asset,
        timeout_timestamp: u64,
        post_swap_action: Action,
        affiliates: Vec<Affiliate>,
    },
    UserSwap {
        swap: Swap,
        min_asset: Asset,
        remaining_asset: Asset,
        affiliates: Vec<Affiliate>,
    },
    PostSwapAction {
        min_asset: Asset,
        timeout_timestamp: u64,
        post_swap_action: Action,
        exact_out: bool,
    },
    Action {
        sent_asset: Option<Asset>,
        timeout_timestamp: u64,
        action: Action,
        exact_out: bool,
        min_asset: Option<Asset>,
    },
    ActionWithRecover {
        sent_asset: Option<Asset>,
        timeout_timestamp: u64,
        action: Action,
        exact_out: bool,
        min_asset: Option<Asset>,
        recovery_addr: Addr,
    },
}

/// This structure describes a CW20 hook message.
#[cw_serde]
pub enum Cw20HookMsg {
    SwapAndActionWithRecover {
        user_swap: Swap,
        min_asset: Asset,
        timeout_timestamp: u64,
        post_swap_action: Action,
        affiliates: Vec<Affiliate>,
        recovery_addr: Addr,
    },
    SwapAndAction {
        user_swap: Swap,
        min_asset: Asset,
        timeout_timestamp: u64,
        post_swap_action: Action,
        affiliates: Vec<Affiliate>,
    },
    Action {
        timeout_timestamp: u64,
        action: Action,
        exact_out: bool,
        min_asset: Option<Asset>,
    },
    ActionWithRecover {
        timeout_timestamp: u64,
        action: Action,
        exact_out: bool,
        min_asset: Option<Asset>,
        recovery_addr: Addr,
    },
}

/////////////
/// QUERY ///
/////////////

// The QueryMsg enum defines the queries the entry point contract provides.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // SwapVenueAdapterContract returns the address of the swap
    // adapter contract for the given swap venue name.
    #[returns(cosmwasm_std::Addr)]
    SwapVenueAdapterContract { name: String },

    // IbcTransferAdapterContract returns the address of the IBC
    // transfer adapter contract.
    #[returns(cosmwasm_std::Addr)]
    IbcTransferAdapterContract {},
}

////////////////////
/// COMMON TYPES ///
////////////////////

// The Action enum is used to specify what action to take after a swap.
#[cw_serde]
pub enum Action {
    Transfer {
        to_address: String,
    },
    IbcTransfer {
        ibc_info: IbcInfo,
        fee_swap: Option<SwapExactAssetOut>,
    },
    ContractCall {
        contract_address: String,
        msg: Binary,
    },
}

// The Affiliate struct is used to specify an affiliate address and BPS fee taken
// from the min_asset to send to that address.
#[cw_serde]
pub struct Affiliate {
    pub basis_points_fee: Uint128,
    pub address: String,
}
