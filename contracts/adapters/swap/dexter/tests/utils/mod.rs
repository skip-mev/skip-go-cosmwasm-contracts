use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{Addr, Coin, Timestamp, Uint128};

use cw_multi_test::addons::{MockAddressGenerator, MockApiBech32};
use cw_multi_test::{App, AppBuilder, BankKeeper, ContractWrapper, Executor, WasmKeeper};

use dexter::vault::{
    ExecuteMsg as VaultExecuteMsg, FeeInfo, InstantiateMsg as VaultInstantiateMsg, PauseInfo,
    PoolCreationFee, PoolInfo, PoolType, PoolTypeConfig, QueryMsg as VaultQueryMsg,
};

use skip::swap::DexterAdapterInstantiateMsg;

pub const EPOCH_START: u64 = 1_000_000;

pub fn mock_app(owner: Addr, coins: Vec<Coin>) -> App<BankKeeper, MockApiBech32> {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(EPOCH_START);

    let mut app = AppBuilder::new()
        .with_api(MockApiBech32::new("persistence"))
        .with_wasm(WasmKeeper::new().with_address_generator(MockAddressGenerator))
        .build(|router, _, storage| {
            // initialization  moved to App construction
            router.bank.init_balance(storage, &owner, coins).unwrap();
        });

    app.set_block(env.block);
    app
}

pub fn store_vault_code(app: &mut App<BankKeeper, MockApiBech32>) -> u64 {
    let factory_contract = Box::new(
        ContractWrapper::new_with_empty(
            dexter_vault::contract::execute,
            dexter_vault::contract::instantiate,
            dexter_vault::contract::query,
        )
        .with_reply_empty(dexter_vault::contract::reply),
    );
    app.store_code(factory_contract)
}

pub fn store_router_code(app: &mut App<BankKeeper, MockApiBech32>) -> u64 {
    let router_contract = Box::new(ContractWrapper::new_with_empty(
        dexter_router::contract::execute,
        dexter_router::contract::instantiate,
        dexter_router::contract::query,
    ));
    app.store_code(router_contract)
}

pub fn store_stable_pool_code(app: &mut App<BankKeeper, MockApiBech32>) -> u64 {
    let pool_contract = Box::new(ContractWrapper::new_with_empty(
        dexter_stable_pool::contract::execute,
        dexter_stable_pool::contract::instantiate,
        dexter_stable_pool::contract::query,
    ));
    app.store_code(pool_contract)
}

pub fn store_weighted_pool_code(app: &mut App<BankKeeper, MockApiBech32>) -> u64 {
    let pool_contract = Box::new(ContractWrapper::new_with_empty(
        dexter_weighted_pool::contract::execute,
        dexter_weighted_pool::contract::instantiate,
        dexter_weighted_pool::contract::query,
    ));
    app.store_code(pool_contract)
}

pub fn store_token_code(app: &mut App<BankKeeper, MockApiBech32>) -> u64 {
    let token_contract = Box::new(ContractWrapper::new_with_empty(
        dexter_lp_token::contract::execute,
        dexter_lp_token::contract::instantiate,
        dexter_lp_token::contract::query,
    ));
    app.store_code(token_contract)
}

pub fn store_dexter_swap_adapter_code(app: &mut App<BankKeeper, MockApiBech32>) -> u64 {
    let dexter_swap_adapter_contract = Box::new(ContractWrapper::new_with_empty(
        skip_go_swap_adapter_dexter::contract::execute,
        skip_go_swap_adapter_dexter::contract::instantiate,
        skip_go_swap_adapter_dexter::contract::query,
    ));
    app.store_code(dexter_swap_adapter_contract)
}

pub struct DexterInstantiateResponse {
    pub dexter_vault_addr: Addr,
    pub dexter_router_addr: Addr,
    // expected that the order of the pool_info is the same as the order of the pool_instantiate_msgs
    pub pool_info: Vec<PoolInfo>,
}

pub fn instantiate_dexter_swap_adapter_contract(
    app: &mut App<BankKeeper, MockApiBech32>,
    owner: &Addr,
    entry_point_contract_address_dummy: &Addr,
    dexter_vault_addr: &Addr,
    dexter_router_addr: &Addr,
) -> Addr {
    let dexter_swap_adapter_code_id = store_dexter_swap_adapter_code(app);

    let dexter_swap_adapter_init_msg = DexterAdapterInstantiateMsg {
        entry_point_contract_address: entry_point_contract_address_dummy.to_string(),
        dexter_vault_contract_address: dexter_vault_addr.to_string(),
        dexter_router_contract_address: dexter_router_addr.to_string(),
    };

    app.instantiate_contract(
        dexter_swap_adapter_code_id,
        owner.to_owned(),
        &dexter_swap_adapter_init_msg,
        &[],
        "skip_swap_adapter:dexter",
        None,
    )
    .unwrap()
}

pub fn instantiate_dexter_contracts_and_pools(
    app: &mut App<BankKeeper, MockApiBech32>,
    owner: &Addr,
    fee_info: FeeInfo,
    pool_instantiate_msgs: Vec<VaultExecuteMsg>,
) -> DexterInstantiateResponse {
    let stable5pool_code_id = store_stable_pool_code(app);
    let weighted_pool_code_id = store_weighted_pool_code(app);
    let vault_code_id = store_vault_code(app);
    let token_code_id = store_token_code(app);
    let router_code_id = store_router_code(app);

    let pool_configs = vec![
        PoolTypeConfig {
            code_id: weighted_pool_code_id,
            pool_type: PoolType::Weighted {},
            default_fee_info: fee_info.clone(),
            allow_instantiation: dexter::vault::AllowPoolInstantiation::Everyone,
            paused: PauseInfo::default(),
        },
        PoolTypeConfig {
            code_id: stable5pool_code_id,
            pool_type: PoolType::StableSwap {},
            default_fee_info: fee_info.clone(),
            allow_instantiation: dexter::vault::AllowPoolInstantiation::Everyone,
            paused: PauseInfo::default(),
        },
    ];

    let vault_init_msg = VaultInstantiateMsg {
        pool_configs: pool_configs.clone(),
        lp_token_code_id: Some(token_code_id),
        fee_collector: Some(app.api().addr_make("fee_collector").to_string()),
        owner: owner.to_string(),
        pool_creation_fee: PoolCreationFee::default(),
        auto_stake_impl: dexter::vault::AutoStakeImpl::None,
    };

    // Initialize Vault contract instance
    let vault_instance = app
        .instantiate_contract(
            vault_code_id,
            owner.to_owned(),
            &vault_init_msg,
            &[],
            "vault",
            None,
        )
        .unwrap();

    let router_instantiate_msg = dexter::router::InstantiateMsg {
        dexter_vault: vault_instance.to_string(),
    };

    let dexter_router_instance = app
        .instantiate_contract(
            router_code_id,
            owner.to_owned(),
            &router_instantiate_msg,
            &[],
            "dexter_router",
            None,
        )
        .unwrap();

    let mut pool_infos = vec![];
    for msg in pool_instantiate_msgs {
        let res = app
            .execute_contract(owner.clone(), vault_instance.clone(), &msg, &[])
            .unwrap();

        // get event by type
        let event = res
            .events
            .iter()
            .find(|e| e.ty == "wasm-dexter-vault::reply::pool_init")
            .unwrap();
        let attribute = event
            .attributes
            .iter()
            .find(|a| a.key == "pool_id")
            .unwrap();

        // get pool id from the event
        let pool_id = attribute.value.parse::<u64>().unwrap();

        let pool_res: PoolInfo = app
            .wrap()
            .query_wasm_smart(
                vault_instance.clone(),
                &VaultQueryMsg::GetPoolById {
                    pool_id: Uint128::from(pool_id),
                },
            )
            .unwrap();

        pool_infos.push(pool_res);
    }

    DexterInstantiateResponse {
        dexter_vault_addr: vault_instance,
        dexter_router_addr: dexter_router_instance,
        pool_info: pool_infos,
    }
}
