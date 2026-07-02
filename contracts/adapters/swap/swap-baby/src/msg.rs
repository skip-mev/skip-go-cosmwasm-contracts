use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct InstantiateMsg {
    pub entry_point_contract_address: String,
    pub router_contract_address: String,
}
