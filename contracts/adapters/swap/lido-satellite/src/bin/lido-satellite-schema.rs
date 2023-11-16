use cosmwasm_schema::write_api;
use skip::swap::{ExecuteMsg, LidoSatelliteInstantiateMsg as InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg
    }
}
