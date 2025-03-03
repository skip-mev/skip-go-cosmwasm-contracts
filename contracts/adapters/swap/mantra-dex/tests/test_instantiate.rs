use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr,
};
use skip::swap::MantraDexInstantiateMsg;
use skip_go_swap_adapter_mantra_dex::state::{
    ENTRY_POINT_CONTRACT_ADDRESS, MANTRA_DEX_POOL_MANAGER_ADDRESS,
};

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let sender = Addr::unchecked("sender");

    // Create mock info with entry point contract address
    let info = mock_info(sender.as_ref(), &[]);

    // Call execute_swap with the given test parameters
    let res = skip_go_swap_adapter_mantra_dex::contract::instantiate(
        deps.as_mut(),
        env,
        info,
        MantraDexInstantiateMsg {
            entry_point_contract_address: "entry_point".to_string(),
            mantra_pool_manager_address: "pool_manager".to_string(),
        },
    );

    assert!(res.is_ok());

    let entry_point = ENTRY_POINT_CONTRACT_ADDRESS
        .load(deps.as_ref().storage)
        .unwrap();
    let pool_manager = MANTRA_DEX_POOL_MANAGER_ADDRESS
        .load(deps.as_ref().storage)
        .unwrap();

    assert_eq!(entry_point, Addr::unchecked("entry_point"));
    assert_eq!(pool_manager, Addr::unchecked("pool_manager"));
}
