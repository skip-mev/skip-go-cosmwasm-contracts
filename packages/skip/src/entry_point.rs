use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Coin, Uint128};

use crate::{
    ibc::IbcInfo,
    swap::{SwapExactCoinIn, SwapExactCoinOut, SwapVenue},
};

///////////////////
/// INSTANTIATE ///
///////////////////

#[cw_serde]
pub struct InstantiateMsg {
    pub swap_venues: Vec<SwapVenue>,
    pub ibc_transfer_contract_address: String,
}

///////////////
/// EXECUTE ///
///////////////

#[cw_serde]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteMsg {
    SwapAndAction {
        fee_swap: Option<SwapExactCoinOut>,
        user_swap: SwapExactCoinIn,
        min_coin: Coin,
        timeout_timestamp: u64,
        post_swap_action: Action,
        affiliates: Vec<Affiliate>,
    },
    PostSwapAction {
        min_coin: Coin,
        timeout_timestamp: u64,
        post_swap_action: Action,
        affiliates: Vec<Affiliate>,
    },
}

/////////////
/// QUERY ///
/////////////

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

#[cw_serde]
pub enum Action {
    BankSend {
        to_address: String,
    },
    IbcTransfer {
        ibc_info: IbcInfo,
    },
    ContractCall {
        contract_address: String,
        msg: Binary,
    },
}

#[cw_serde]
pub struct Affiliate {
    pub basis_points_fee: Uint128,
    pub address: String,
}
