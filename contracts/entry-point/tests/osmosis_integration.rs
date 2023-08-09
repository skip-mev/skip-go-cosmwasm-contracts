use cosmwasm_std::Coin;
use osmosis_test_tube::{Module, OsmosisTestApp, Gamm, PoolManager};
use osmosis_std::types::osmosis::poolmanager::v1beta1::AllPoolsRequest;

#[test]
fn test_osmosis() {
    let app = OsmosisTestApp::default();
    let alice = app
        .init_account(&[
            Coin::new(1_000_000_000_000, "uatom"),
            Coin::new(1_000_000_000_000, "uosmo"),
        ])
        .unwrap();

    // create Gamm Module Wrapper
    let gamm = Gamm::new(&app);

    // create balancer pool with basic configuration
    let pool_liquidity = vec![Coin::new(100_000, "uatom"), Coin::new(1100_000_000, "uosmo")];
    let pool_id = gamm
        .create_basic_pool(&pool_liquidity, &alice)
        .unwrap()
        .data
        .pool_id;

    // query pool and assert if the pool is created successfully
    let pool = gamm.query_pool(pool_id).unwrap();
    println!("pool: {:?}", pool);

    // create pool manager
    let pool_manager = PoolManager::new(&app);
    let req = AllPoolsRequest{};
    let resp = pool_manager.query_all_pools(&req);
    println!("all pools: {:?}", resp);
}