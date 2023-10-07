// use cosmwasm_std::Empty;
// use cw_multi_test::{App, Contract, ContractWrapper};
//
// pub fn mock_app() -> App {
//     App::default()
// }
//
// pub fn mock_entry_point_contract() -> Box<dyn Contract<Empty>> {
//     let contract = ContractWrapper::new(
//         skip_api_entry_point::contract::execute,
//         skip_api_entry_point::contract::instantiate,
//         skip_api_entry_point::contract::query,
//     ).with_reply(skip_api_entry_point::contract::reply);
//     Box::new(contract)
// }
