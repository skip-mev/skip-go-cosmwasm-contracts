use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Uint128};

#[cw_serde]
pub enum HallswapExecuteMsg {
    ExecuteRoutesV2 {
        routes: Vec<HallswapRouteInfo>,
        minimum_receive: Uint128,
        to: Option<Addr>,
    },
}

#[cw_serde]
pub struct HallswapRouteInfo {
    pub route: Vec<HallswapSwapOperation>,
    pub offer_amount: Uint128,
}

#[cw_serde]
pub struct HallswapSwapOperation {
    pub contract_addr: Addr,
    pub offer_asset: AssetInfo,
    pub return_asset: AssetInfo,
    pub interface: Option<HallswapInterface>,
}

#[cw_serde]
pub enum HallswapInterface {
    Binary(Binary),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum HallswapQueryMsg {
    #[returns(HallswapQuerySimulationResult)]
    Simulation { routes: Vec<HallswapRouteInfo> },
}

#[cw_serde]
pub struct HallswapQuerySimulationResult {
    pub return_asset: Asset,
    pub fee_asset: Option<Asset>,
}
