use crate::{
    asset::Asset,
    ibc::IbcInfo,
    swap::{Swap, SwapExactAssetOut, SwapVenue},
};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, StdResult, Uint128};
use cw20::Cw20ReceiveMsg;
use cw20_ics20_msg::msg::TransferBackMsg;
use oraiswap::universal_swap_memo::memo::PostAction;

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
    pub swap_venues: Option<Vec<SwapVenue>>,
    pub ibc_transfer_contract_address: Option<String>,
    pub ibc_wasm_contract_address: Option<String>,
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
    UpdateConfig {
        owner: Option<Addr>,
        swap_venues: Option<Vec<SwapVenue>>,
        ibc_transfer_contract_address: Option<String>,
        ibc_wasm_contract_address: Option<String>,
    },
    UniversalSwap {
        memo: String,
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
    UniversalSwap {
        memo: String,
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
    IbcWasmTransfer {
        ibc_wasm_info: TransferBackMsg,
        fee_swap: Option<SwapExactAssetOut>,
    },
}

// convert PostAction of universal swap to Action
impl Action {
    pub fn try_from(post_swap_action: PostAction, timeout_timestamp: u64) -> StdResult<Self> {
        if let Some(ibc_transfer) = post_swap_action.ibc_transfer_msg {
            return Ok(Action::IbcTransfer {
                ibc_info: IbcInfo::from(ibc_transfer),
                fee_swap: None,
            });
        }
        if let Some(contract_call) = post_swap_action.contract_call {
            return Ok(Action::ContractCall {
                contract_address: contract_call.contract_address,
                msg: Binary::from_base64(&contract_call.msg)?,
            });
        }
        if let Some(ibc_wasm_transfer) = post_swap_action.ibc_wasm_transfer_msg {
            return Ok(Action::IbcWasmTransfer {
                ibc_wasm_info: TransferBackMsg {
                    local_channel_id: ibc_wasm_transfer.local_channel_id,
                    remote_address: ibc_wasm_transfer.remote_address,
                    remote_denom: ibc_wasm_transfer.remote_denom,
                    timeout: Some(timeout_timestamp),
                    memo: ibc_wasm_transfer.memo,
                },
                fee_swap: None,
            });
        }
        Err(cosmwasm_std::StdError::GenericErr {
            msg: "No post swap action found".to_string(),
        })
    }
}

// The Affiliate struct is used to specify an affiliate address and BPS fee taken
// from the min_asset to send to that address.
#[cw_serde]
pub struct Affiliate {
    pub basis_points_fee: Uint128,
    pub address: String,
}
