// use crate::helpers::mock_env::MockEnv;
// use cosmwasm_std::testing::mock_env;
// use cosmwasm_std::{Addr, Coin};
// use cw_multi_test::{App, BankSudo, ContractWrapper, SudoMsg};
// use skip::entry_point::Action;
// use skip_api_entry_point::contract::execute;
//
// pub mod helpers;
//
// #[test]
// pub fn test_swap_err() {
//     let user = Addr::unchecked("user");
//
//     let mut mock = MockEnv::new().build().();
//
//     let min_coin = Coin::new(1_000_000, "osmo");
//     let timeout_timestamp = 101;
//     let post_swap_action = Action::BankSend {
//         to_address: "to_address".to_string(),
//     };
//     let exact_out = false;
//
//     mock.execute_post_swap_action(min_coin, timeout_timestamp, post_swap_action, exact_out, &user, &[Coin::new(1_000_000, "untrn")]).unwrap();
// }
