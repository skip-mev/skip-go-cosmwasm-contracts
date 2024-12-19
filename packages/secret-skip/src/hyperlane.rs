use cosmwasm_schema::cw_serde;
use cosmwasm_std::HexBinary;

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

// The InstantiateMsg struct defines the initialization parameters for the IBC Transfer Adapter contracts.
#[cw_serde]
pub struct InstantiateMsg {
    pub entry_point_contract_address: String,
}

///////////////
/// EXECUTE ///
///////////////

// The ExecuteMsg enum defines the execution message that the IBC Transfer Adapter contracts can handle.
#[cw_serde]
pub enum ExecuteMsg {
    HplTransfer {
        dest_domain: u32,
        recipient: HexBinary,
        hook: Option<String>,
        metadata: Option<HexBinary>,
        warp_address: String,
    },
}
