use skip_go_swap_adapter_swap_baby::contract::{execute, instantiate};
use skip_go_swap_adapter_swap_baby::error::ContractError;
use skip_go_swap_adapter_swap_baby::msg::InstantiateMsg;
use skip_go_swap_adapter_swap_baby::state::{ENTRY_POINT_CONTRACT_ADDRESS, ROUTER_CONTRACT_ADDRESS};
use skip_go_swap_adapter_swap_baby::swap_baby::ExecuteMsg as SwapBabyExecuteMsg;
use skip_go_swap_adapter_swap_baby::swap_baby::Hop as SwapBabyHop;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{to_json_binary, to_json_string, Addr, Coin, CosmosMsg, WasmMsg};
use skip::swap::{ExecuteMsg, SwapOperation};

#[test]
fn test_instantiate() {
    // Setup
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("addr", &[]);
    let entry_point_addr = Addr::unchecked("entry_point");
    let router_addr = Addr::unchecked("router");

    // Instantiate the contract
    let msg = InstantiateMsg {
        entry_point_contract_address: entry_point_addr.to_string(),
        router_contract_address: router_addr.to_string(),
    };
    let result = instantiate(deps.as_mut(), env, info, msg);
    assert!(result.is_ok(), "Instantiation should succeed");

    // Verify state
    let stored_entry_point = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.as_ref().storage).unwrap();
    assert_eq!(entry_point_addr, stored_entry_point, "Incorrect entry point address stored");

    let stored_router = ROUTER_CONTRACT_ADDRESS.load(deps.as_ref().storage).unwrap();
    assert_eq!(router_addr, stored_router, "Incorrect router address stored");
}

#[test]
fn test_execute_unauthorized() {
    // Setup
    let mut deps = mock_dependencies();
    let env = mock_env();
    let instantiate_info = mock_info("addr", &[]);
    let entry_point_addr = Addr::unchecked("entry_point");
    let router_addr = Addr::unchecked("router");

    // Instantiate the contract
    let instantiate_msg = InstantiateMsg {
        entry_point_contract_address: entry_point_addr.to_string(),
        router_contract_address: router_addr.to_string(),
    };
    instantiate(deps.as_mut(), env.clone(), instantiate_info, instantiate_msg).unwrap();

    // Attempt unauthorized execution
    let unauthorized_info = mock_info("unauthorized", &[]);
    let execute_msg = ExecuteMsg::Swap {
        operations: vec![],
    };

    let result = execute(deps.as_mut(), env, unauthorized_info, execute_msg);

    // Verify the error
    match result {
        Err(ContractError::Unauthorized {}) => {}
        _ => panic!("Expected Unauthorized error, but got: {:?}", result),
    }
}

#[test]
fn test_execute_swap_success() {
    // Setup
    let mut deps = mock_dependencies();
    let env = mock_env();
    let entry_point_addr = Addr::unchecked("entry_point");
    let router_addr = Addr::unchecked("router");

    // Instantiate the contract
    let instantiate_msg = InstantiateMsg {
        entry_point_contract_address: entry_point_addr.to_string(),
        router_contract_address: router_addr.to_string(),
    };
    let info = mock_info("addr", &[]);
    instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();

    // Prepare swap operations
    let operations = vec![SwapOperation {
        pool: "cw-BTC-USDT".to_string(),
        denom_in: "BTC".to_string(),
        denom_out: "USDT".to_string(),
        interface: None,
    }];

    // Execute the swap
    let execute_msg = ExecuteMsg::Swap {
        operations: operations.clone(),
    };
    let funds = vec![Coin::new(1000, "BTC")];
    let info = mock_info(entry_point_addr.as_str(), &funds);

    let result = execute(deps.as_mut(), env, info, execute_msg).unwrap();

    // Assertions
    assert_eq!(result.messages.len(), 1, "Expected one response message");

    let actual_msg = &result.messages[0].msg;
    let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: router_addr.to_string(),
        msg: to_json_binary(&SwapBabyExecuteMsg::SwapExactAmountInWithHops {
            receiver: Some(entry_point_addr.to_string()),
            min_out: Default::default(),
            hops: operations
                .iter()
                .map(|op| SwapBabyHop {
                    pool: op.pool.clone(),
                    denom: op.denom_out.clone(),
                })
                .collect(),
        })
        .unwrap(),
        funds,
    });

    assert_eq!(actual_msg, &expected_msg, "Unexpected message content");
}
