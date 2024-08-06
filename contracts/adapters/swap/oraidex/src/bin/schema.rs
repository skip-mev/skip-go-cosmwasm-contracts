use cosmwasm_schema::write_api;
use oraiswap::mixed_router::{ExecuteMsg, QueryMsg};
use skip::swap::OraidexInstantiateMsg as InstantiateMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg
    }
}
