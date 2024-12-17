use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, ContractInfo};
use secret_skip::{
    asset::{Asset, Snip20ReceiveMsg},
    swap::SwapOperation,
};

#[cw_serde]
pub struct InstantiateMsg {
    pub entry_point_contract: ContractInfo,
    pub shade_router_contract: ContractInfo,
    pub shade_pool_code_hash: String,
    pub viewing_key: String,
}

#[cw_serde]
pub struct MigrateMsg {
    pub entry_point_contract: ContractInfo,
    pub shade_router_contract: ContractInfo,
    pub shade_pool_code_hash: String,
    pub viewing_key: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Snip20ReceiveMsg),
    TransferFundsBack { swapper: Addr, return_denom: String },
    RegisterTokens { contracts: Vec<ContractInfo> },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // SimulateSwapExactAssetIn returns the asset out received from the specified asset in
    #[returns(Asset)]
    SimulateSwapExactAssetIn {
        asset_in: Asset,
        swap_operations: Vec<SwapOperation>,
    },
}

#[cw_serde]
pub enum Cw20HookMsg {
    Swap { operations: Vec<SwapOperation> },
}

/*
#[cw_serde]
pub struct SwapOperation {
    pub pool: String,
    pub denom_in: String,
    pub denom_out: String,
    pub interface: Option<Binary>,
}
*/
