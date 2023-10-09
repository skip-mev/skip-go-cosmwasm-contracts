// use anyhow::Result as AnyResult;
// use cosmwasm_schema::cw_serde;
// use cosmwasm_std::{Addr, Coin, Empty, StdResult};
// use cw_multi_test::{
//     App, AppResponse, BankSudo, BasicApp, Contract, ContractWrapper, Executor, SudoMsg,
// };
// use skip::entry_point::{Action, Affiliate, ExecuteMsg, InstantiateMsg, QueryMsg};
// use skip::ibc::{
//     ExecuteMsg as IbcExecuteMsg, InstantiateMsg as IbcInstantiateMsg, QueryMsg as IbcQueryMsg,
// };
// use skip::swap::{Swap, SwapVenue};
// use std::mem::take;
//
// // Set up contracts:
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
//     )
//     .with_reply(skip_api_entry_point::contract::reply);
//     Box::new(contract)
// }
//
// // FIXME:: IBC transfer and swap venue contract set ups
// pub fn mock_ibc_transfer_contract() -> Box<dyn Contract<Empty>> {
//     let contract = ContractWrapper::new(
//         skip::ibc::IbcExecuteMsg,
//         skip::ibc::IbcInstantiateMsg,
//         skip::ibc::IbcQueryMsg,
//     )
//     .with_reply(skip_api_entry_point::contract::reply);
//     Box::new(contract)
// }
//
// pub fn mock_swap_venue_contract() -> Box<dyn Contract<Empty>> {
//     let contract = ContractWrapper::new(
//         skip::swap::ExecuteMsg,
//         skip::swap::NeutronInstantiateMsg,
//         skip::swap::QueryMsg,
//     )
//     .with_reply(skip_api_entry_point::contract::reply);
//     Box::new(contract)
// }
//
// // Set up mock environment
// pub struct MockEnv {
//     pub app: BasicApp,
//     pub entry_point: Addr,
//     pub ibc_transfer_contract_address: String,
//     pub swap_venues_contract: Vec<SwapVenue>,
// }
//
// pub struct MockEnvBuilder {
//     pub app: BasicApp,
//     pub owner: Option<Addr>,
//     pub swap_venues: Vec<SwapVenue>,
//     pub entry_point_contract_address: String,
//     pub accounts_to_fund: Vec<AccountToFund>,
//     pub ibc_transfer_contract_address: String,
// }
//
// #[cw_serde]
// pub struct AccountToFund {
//     pub addr: Addr,
//     pub funds: Vec<Coin>,
// }
//
// #[allow(clippy::new_ret_no_self)]
// impl MockEnv {
//     pub fn new() -> MockEnvBuilder {
//         MockEnvBuilder {
//             app: App::default(),
//             owner: None,
//             swap_venues: vec![],
//             entry_point_contract_address: "".to_string(),
//             accounts_to_fund: vec![],
//             ibc_transfer_contract_address: "".to_string(),
//         }
//     }
//
//     // Execute Messages:
//
//     pub fn swap_and_send_action(
//         &mut self,
//         user_swap: Swap,
//         min_coin: Coin,
//         timeout_timestamp: u64,
//         post_swap_action: Action,
//         affiliates: Vec<Affiliate>,
//         sender: &Addr,
//         send_funds: &[Coin],
//     ) -> AnyResult<AppResponse> {
//         self.app.execute_contract(
//             sender.clone(),
//             self.entry_point.clone(),
//             &ExecuteMsg::SwapAndAction {
//                 user_swap,
//                 min_coin,
//                 timeout_timestamp,
//                 post_swap_action,
//                 affiliates,
//             },
//             send_funds,
//         )
//     }
//
//     pub fn execute_user_swap(
//         &mut self,
//         swap: Swap,
//         min_coin: Coin,
//         remaining_coin: Coin,
//         affiliates: Vec<Affiliate>,
//         sender: &Addr,
//         send_funds: &[Coin],
//     ) -> AnyResult<AppResponse> {
//         self.app.execute_contract(
//             sender.clone(),
//             self.entry_point.clone(),
//             &ExecuteMsg::UserSwap {
//                 swap,
//                 min_coin,
//                 remaining_coin,
//                 affiliates,
//             },
//             send_funds,
//         )
//     }
//
//     pub fn execute_post_swap_action(
//         &mut self,
//         min_coin: Coin,
//         timeout_timestamp: u64,
//         post_swap_action: Action,
//         exact_out: bool,
//         sender: &Addr,
//         send_funds: &[Coin],
//     ) -> AnyResult<AppResponse> {
//         self.app.execute_contract(
//             sender.clone(),
//             self.entry_point.clone(),
//             &ExecuteMsg::PostSwapAction {
//                 min_coin,
//                 timeout_timestamp,
//                 post_swap_action,
//                 exact_out,
//             },
//             send_funds,
//         )
//     }
//
//     // Query Messages:
//     pub fn query_swap_venue(&self, name: String) -> StdResult<Addr> {
//         self.app
//             .wrap()
//             .query_wasm_smart(
//                 self.entry_point.clone(),
//                 &QueryMsg::SwapVenueAdapterContract { name },
//             )
//             .unwrap()
//     }
//
//     pub fn query_ibc_transfer(&self) -> StdResult<Addr> {
//         self.app
//             .wrap()
//             .query_wasm_smart(
//                 self.entry_point.clone(),
//                 &QueryMsg::IbcTransferAdapterContract {},
//             )
//             .unwrap()
//     }
// }
//
// impl MockEnvBuilder {
//     pub fn build(&mut self) -> AnyResult<MockEnv> {
//         let entry_point = self.get_entry_point()?;
//
//         let ibc_transfer_contract_address = self.get_ibc_transfer_addr();
//         let swap_venues_contract = self.get_swap_venues();
//
//         self.fund_users();
//
//         self.deploy_vaults();
//
//         Ok(MockEnv {
//             app: take(&mut self.app),
//             entry_point,
//             ibc_transfer_contract_address,
//             swap_venues_contract,
//         })
//     }
//
//     // Set up functions:
//
//     pub fn fund_users(&mut self) {
//         for account in &self.accounts_to_fund {
//             self.app
//                 .sudo(SudoMsg::Bank(BankSudo::Mint {
//                     to_address: account.addr.to_string(),
//                     amount: account.funds.clone(),
//                 }))
//                 .unwrap();
//         }
//     }
//
//     pub fn get_entry_point(&mut self) -> AnyResult<Addr> {
//         let code_id = self.app.store_code(mock_entry_point_contract());
//         let ibc_transfer_contract_address = self.deploy_ibc_transfer_contract();
//         let swap_venues = self.get_swap_venues();
//
//         let addr = self
//             .app
//             .instantiate_contract(
//                 code_id,
//                 self.get_owner(),
//                 &InstantiateMsg {
//                     swap_venues,
//                     ibc_transfer_contract_address: ibc_transfer_contract_address.to_string(),
//                 },
//                 &[],
//                 "mock-entry-point-contract",
//                 None,
//             )
//             .unwrap();
//
//         Ok(addr)
//     }
//
//     // Fixme:: Add Swap venues
//     pub fn get_swap_venues(&mut self) -> Vec<SwapVenue> {
//         vec![SwapVenue {
//             name: "".to_string(),
//             adapter_contract_address: "".to_string(),
//         }]
//     }
//
//     pub fn deploy_ibc_transfer_contract(&mut self) -> Addr {
//         let contract_code_id = self.app.store_code(mock_ibc_transfer_contract());
//         let owner = self.get_owner();
//
//         self.app
//             .instantiate_contract(
//                 contract_code_id,
//                 owner.clone(),
//                 &IbcInstantiateMsg {
//                     entry_point_contract_address: "".to_string(),
//                 },
//                 &[],
//                 "mock-address-provider",
//                 None,
//             )
//             .unwrap()
//     }
//     pub fn get_ibc_transfer_addr(&mut self) -> String {
//         if self.ibc_transfer_contract_address.is_none() {
//             let addr = self.deploy_ibc_transfer_contract();
//             self.ibc_transfer_contract_address = addr.to_string();
//         }
//         self.ibc_transfer_contract_address.clone().unwrap()
//     }
//
//     pub fn deploy_swap_venue_contract(&mut self) -> Addr {
//         let contract_code_id = self.app.store_code(mock_swap_venue_contract());
//         let owner = self.get_owner();
//
//         // FIXME: Needs to be swap instantiate msg
//         self.app
//             .instantiate_contract(
//                 contract_code_id,
//                 owner.clone(),
//                 &InstantiateMsg {
//                     swap_venues: vec![],
//                     ibc_transfer_contract_address: "".to_string(),
//                 },
//                 &[],
//                 "mock-address-provider",
//                 None,
//             )
//             .unwrap()
//     }
//
//     pub fn get_swap_venue_addr(&mut self) -> String {
//         if self.ibc_transfer_contract_address.is_none() {
//             let addr = self.deploy_ibc_transfer_contract();
//             self.ibc_transfer_contract_address = addr.to_string();
//         }
//         self.ibc_transfer_contract_address.clone().unwrap()
//     }
// }
