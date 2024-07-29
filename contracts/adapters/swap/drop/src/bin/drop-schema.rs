use cosmwasm_schema::write_api;
use skip::swap::{DropBondInstantiateMsg as InstantiateMsg, ExecuteMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg
    }
}
