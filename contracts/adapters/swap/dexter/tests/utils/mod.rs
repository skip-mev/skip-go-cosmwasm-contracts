use std::collections::HashMap;

use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{attr, to_json_binary, Addr, Coin, Decimal, Decimal256, Timestamp, Uint128};
use cw20::{BalanceResponse, Cw20QueryMsg, MinterResponse};
use cw_multi_test::{App, ContractWrapper, Executor};

use dexter::asset::{Asset, AssetExchangeRate, AssetInfo};
use dexter::lp_token::InstantiateMsg as TokenInstantiateMsg;
use dexter::pool::{
    AfterExitResponse, AfterJoinResponse, ConfigResponse, CumulativePricesResponse, ExitType,
    FeeStructs, QueryMsg, SwapResponse,
};
use dexter::vault::{
    Cw20HookMsg, ExecuteMsg as VaultExecuteMsg, FeeInfo, InstantiateMsg as VaultInstantiateMsg,
    NativeAssetPrecisionInfo, PauseInfo, PoolCreationFee, PoolInfo, PoolType, PoolTypeConfig,
    QueryMsg as VaultQueryMsg, SingleSwapRequest, SwapType,
};

use cw20::Cw20ExecuteMsg;

use dexter::pool::ExitType::ExactLpBurn;
use dexter::vault;
use dexter_stable_pool::state::{AssetScalingFactor, MathConfig, StablePoolParams};
use skip::swap::{DexterAdapterInstantiateMsg, InstantiateMsg};

pub const EPOCH_START: u64 = 1_000_000;

pub fn mock_app(owner: Addr, coins: Vec<Coin>) -> App {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(EPOCH_START);

    let mut app = App::new(|router, _, storage| {
        // initialization  moved to App construction
        router.bank.init_balance(storage, &owner, coins).unwrap();
    });
    app.set_block(env.block);
    app
}

pub fn store_vault_code(app: &mut App) -> u64 {
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

pub fn store_router_code(app: &mut App) -> u64 {
    let router_contract = Box::new(ContractWrapper::new_with_empty(
        dexter_router::contract::execute,
        dexter_router::contract::instantiate,
        dexter_router::contract::query,
    ));
    app.store_code(router_contract)
}

pub fn store_stable_pool_code(app: &mut App) -> u64 {
    let pool_contract = Box::new(ContractWrapper::new_with_empty(
        dexter_stable_pool::contract::execute,
        dexter_stable_pool::contract::instantiate,
        dexter_stable_pool::contract::query,
    ));
    app.store_code(pool_contract)
}

pub fn store_weighted_pool_code(app: &mut App) -> u64 {
    let pool_contract = Box::new(ContractWrapper::new_with_empty(
        dexter_weighted_pool::contract::execute,
        dexter_weighted_pool::contract::instantiate,
        dexter_weighted_pool::contract::query,
    ));
    app.store_code(pool_contract)
}

pub fn store_token_code(app: &mut App) -> u64 {
    let token_contract = Box::new(ContractWrapper::new_with_empty(
        dexter_lp_token::contract::execute,
        dexter_lp_token::contract::instantiate,
        dexter_lp_token::contract::query,
    ));
    app.store_code(token_contract)
}

pub fn store_dexter_swap_adapter_code(app: &mut App) -> u64 {
    let dexter_swap_adapter_contract = Box::new(ContractWrapper::new_with_empty(
        skip_api_swap_adapter_dexter::contract::execute,
        skip_api_swap_adapter_dexter::contract::instantiate,
        skip_api_swap_adapter_dexter::contract::query,
    ));
    app.store_code(dexter_swap_adapter_contract)
}

// Mints some Tokens to "to" recipient
pub fn mint_some_tokens(
    app: &mut App,
    owner: Addr,
    token_instance: Addr,
    amount: Uint128,
    to: String,
) {
    let msg = cw20::Cw20ExecuteMsg::Mint {
        recipient: to.clone(),
        amount: amount,
    };
    let res = app
        .execute_contract(owner.clone(), token_instance.clone(), &msg, &[])
        .unwrap();
    assert_eq!(res.events[1].attributes[1], attr("action", "mint"));
    assert_eq!(res.events[1].attributes[2], attr("to", to));
    assert_eq!(res.events[1].attributes[3], attr("amount", amount));
}

pub struct DexterInstantiateResponse {
    pub dexter_vault_addr: Addr,
    pub dexter_router_addr: Addr,
    // expected that the order of the pool_info is the same as the order of the pool_instantiate_msgs
    pub pool_info: Vec<PoolInfo>
}


pub fn instantiate_dexter_swap_adapter_contract(
    app: &mut App,
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

    let address = app.instantiate_contract(
        dexter_swap_adapter_code_id,
        owner.to_owned(),
        &dexter_swap_adapter_init_msg,
        &[],
        "skip_swap_adapter:dexter",
        None,
    ).unwrap();

    address
}

pub fn instantiate_dexter_contracts_and_pools(
    app: &mut App,
    owner: &Addr,
    fee_info: FeeInfo,
    pool_instantiate_msgs: Vec<VaultExecuteMsg>
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
        }
    ];

    let vault_init_msg = VaultInstantiateMsg {
        pool_configs: pool_configs.clone(),
        lp_token_code_id: Some(token_code_id),
        fee_collector: Some("fee_collector".to_string()),
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

    let dexter_router_instance = app.
        instantiate_contract(
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

        println!("Pool instantiate response: {:?}", res);

        // get event by type
        let event = res.events.iter().find(|e| e.ty == "wasm-dexter-vault::reply::pool_init").unwrap();
        let attribute = event.attributes.iter().find(|a| a.key == "pool_id").unwrap();

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
        pool_info: pool_infos
    }
}

pub fn add_liquidity_to_pool(
    app: &mut App,
    owner: &Addr,
    user: &Addr,
    vault_addr: Addr,
    pool_id: Uint128,
    pool_addr: Addr,
    amount_to_add: Vec<Asset>,
) -> Uint128 {
    // Find CW20 assets from the bootstrapping amount and mint token to the user
    let cw20_assets: Vec<AssetInfo> = amount_to_add
        .iter()
        .filter(|a| !a.info.is_native_token())
        .map(|a| a.info.clone())
        .collect();

    // Step 1: Mint CW20 tokens to the user
    for asset in &cw20_assets {
        let mint_msg = Cw20ExecuteMsg::Mint {
            recipient: user.to_string(),
            amount: Uint128::from(1_000_000_000_000_000_000u128),
        };
        let contract_address = asset.to_string();
        app.execute_contract(
            owner.clone(),
            Addr::unchecked(contract_address),
            &mint_msg,
            &[],
        )
        .unwrap();
    }

    // Step 2: Add allowance for the pool to spend the user's tokens
    for asset in &cw20_assets {
        let allowance_msg = Cw20ExecuteMsg::IncreaseAllowance {
            spender: vault_addr.to_string(),
            amount: Uint128::from(1_000_000_000_000_000_000u128),
            expires: None,
        };
        let contract_address = asset.to_string();
        app.execute_contract(
            user.clone(),
            Addr::unchecked(contract_address),
            &allowance_msg,
            &[],
        )
        .unwrap();
    }

    // Step 3: Create coins vec for native tokens to be sent for joining pool
    let native_token: Vec<&Asset> = amount_to_add
        .iter()
        .filter(|a| a.info.is_native_token())
        .collect();

    let mut coins = vec![];
    for asset in native_token {
        let denom = asset.info.to_string();
        coins.push(Coin {
            denom,
            amount: asset.amount,
        });
    }

    // Step 4: Do the query to get to join pool once
    let query_msg = QueryMsg::OnJoinPool {
        assets_in: Some(amount_to_add.clone()),
        mint_amount: None,
    };

    let res: AfterJoinResponse = app
        .wrap()
        .query_wasm_smart(pool_addr.as_str(), &query_msg)
        .unwrap();

    // Step 4: Execute join pool
    let msg = VaultExecuteMsg::JoinPool {
        pool_id,
        recipient: None,
        auto_stake: None,
        assets: Some(amount_to_add),
        min_lp_to_receive: None,
    };

    app.execute_contract(user.clone(), vault_addr.clone(), &msg, &coins)
        .unwrap();

    res.new_shares
}

pub fn query_cw20_balance(app: &mut App, user: &Addr, contract_addr: Addr) -> Uint128 {
    let query_msg = Cw20QueryMsg::Balance {
        address: user.to_string(),
    };
    let res: BalanceResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.as_str(), &query_msg)
        .unwrap();

    res.balance
}

pub fn query_bank_balance(app: &mut App, user: &Addr, denom: String) -> Uint128 {
    let res: Coin = app.wrap().query_balance(user.clone(), denom).unwrap();

    res.amount
}

pub fn create_cw20_asset(
    app: &mut App,
    owner: &Addr,
    token_code_id: u64,
    name: String,
    symbol: String,
    decimals: u8,
) -> Addr {
    // Create Token X
    let init_msg = TokenInstantiateMsg {
        name: name.clone(),
        symbol,
        decimals,
        initial_balances: vec![],
        mint: Some(MinterResponse {
            minter: owner.to_string(),
            cap: None,
        }),
        marketing: None,
    };
    let token_instance0 = app
        .instantiate_contract(
            token_code_id,
            Addr::unchecked(owner.clone()),
            &init_msg,
            &[],
            name,
            None,
        )
        .unwrap();

    return token_instance0;
}


pub fn log_pool_info(app: &mut App, pool_addr: &Addr) {
    let pool_info_query = QueryMsg::Config {};
    let pool_info_response: ConfigResponse = app
        .wrap()
        .query_wasm_smart(pool_addr.clone(), &pool_info_query)
        .unwrap();

    println!("Pool Info: {:?}", pool_info_response);
}