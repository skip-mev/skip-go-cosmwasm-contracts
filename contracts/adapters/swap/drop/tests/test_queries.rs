use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    to_json_binary, Addr, Binary, Coin, ContractResult as SystemContractResult, Decimal,
    QuerierResult, SystemResult, WasmQuery,
};
// use lido_satellite::msg::ExecuteMsg as LidoSatelliteExecuteMsg;
use skip::{asset::Asset, swap::QueryMsg};
use skip_go_swap_adapter_drop::{
    error::ContractResult, error::ContractError,
    state::{DROP_CORE_CONTRACT_ADDRESS, FACTORY_BONDED_DENOM, IBC_REMOTE_DENOM},
};
use test_case::test_case;

// Define test parameters
struct Params {
    query: QueryMsg,
    response: Binary,
    exchange_rate: Decimal,
    expected_error: Option<ContractError>,
}

// Test queries
#[test_case(
    Params {
        query: QueryMsg::SimulateSwapExactAssetIn {
            swap_operations: vec![],
            asset_in: Asset::Native(Coin::new(100, "ibc/uatom")),
        },
        response: to_json_binary(&Asset::Native(Coin::new(
            100,
            "factory/uatom",
        ))).unwrap(),
        exchange_rate: Decimal::one(),
        expected_error: None,
    };
    "SimulateSwapExactAssetIn Query")]
#[test_case(
    Params {
        query: QueryMsg::SimulateSwapExactAssetIn {
            swap_operations: vec![],
            asset_in: Asset::Native(Coin::new(100, "ibc/uatom")),
        },
        response: to_json_binary(&Asset::Native(Coin::new(
            50,
            "factory/uatom",
        ))).unwrap(),
        exchange_rate: Decimal::from_atomics(cosmwasm_std::Uint128::new(5), 1).unwrap(),
        expected_error: None,
    };
    "SimulateSwapExactAssetIn Query half exchange rate")]
#[test_case(
    Params {
        query: QueryMsg::SimulateSwapExactAssetOut {
            swap_operations: vec![],
            asset_out: Asset::Native(Coin::new(100, "ibc/uatom")),
        },
        response: Binary([].to_vec()),
        exchange_rate: Decimal::one(),
        expected_error: Some(ContractError::UnsupportedDenom),
    };
    "SimulateSwapExactAssetOut Query - want out remote denom")]
#[test_case(
    Params {
        query: QueryMsg::SimulateSwapExactAssetOut {
            swap_operations: vec![],
            asset_out: Asset::Native(Coin::new(100, "factory/uatom")),
        },
        response: to_json_binary(&Asset::Native(Coin::new(
            100,
            "ibc/uatom",
        ))).unwrap(),
        exchange_rate: Decimal::one(),
        expected_error: None,
    };
    "SimulateSwapExactAssetOut Query - want out factory denom")]
#[test_case(
    Params {
        query: QueryMsg::SimulateSwapExactAssetOut {
            swap_operations: vec![],
            asset_out: Asset::Native(Coin::new(100, "factory/uatom")),
        },
        response: to_json_binary(&Asset::Native(Coin::new(
            200,
            "ibc/uatom",
        ))).unwrap(),
        exchange_rate: Decimal::from_atomics(cosmwasm_std::Uint128::new(5), 1).unwrap(),
        expected_error: None,
    };
    "SimulateSwapExactAssetOut Query half exchange rate - want out factory denom")]
#[test_case(
    Params {
        query: QueryMsg::SimulateSwapExactAssetOut {
            swap_operations: vec![],
            asset_out: Asset::Native(Coin::new(100, "ibc/uatom")),
        },
        response: Binary([].to_vec()),
        exchange_rate: Decimal::from_atomics(cosmwasm_std::Uint128::new(5), 1).unwrap(),
        expected_error: Some(ContractError::UnsupportedDenom),
    };
    "SimulateSwapExactAssetOut Query half exchange rate - want out remote denom")]
#[test_case(
    Params {
        query: QueryMsg::SimulateSwapExactAssetInWithMetadata {
            swap_operations: vec![],
            asset_in: Asset::Native(Coin::new(100, "ibc/uatom")),
            include_spot_price: false,
        },
        response: to_json_binary(&skip::swap::SimulateSwapExactAssetInResponse{
            asset_out: Asset::Native(Coin::new(
                100,
                "factory/uatom",
            )),
            spot_price: None
        }).unwrap(),
        exchange_rate: Decimal::one(),
        expected_error: None,
    };
    "SimulateSwapExactAssetInWithMetadata Query")]
#[test_case(
    Params {
        query: QueryMsg::SimulateSwapExactAssetInWithMetadata {
            swap_operations: vec![],
            asset_in: Asset::Native(Coin::new(100, "ibc/uatom")),
            include_spot_price: true,
        },
        response: to_json_binary(&skip::swap::SimulateSwapExactAssetInResponse{
            asset_out: Asset::Native(Coin::new(
                50,
                "factory/uatom",
            )),
            spot_price: Some(Decimal::from_atomics(cosmwasm_std::Uint128::new(5), 1).unwrap())
        }).unwrap(),
        exchange_rate: Decimal::from_atomics(cosmwasm_std::Uint128::new(5), 1).unwrap(),
        expected_error: None,
    };
    "SimulateSwapExactAssetInWithMetadata Query include spot price")]
#[test_case(
    Params {
        query: QueryMsg::SimulateSwapExactAssetOutWithMetadata {
            swap_operations: vec![],
            asset_out: Asset::Native(Coin::new(100, "factory/uatom")),
            include_spot_price: false,
        },
        response: to_json_binary(&skip::swap::SimulateSwapExactAssetOutResponse{
            asset_in: Asset::Native(Coin::new(
                100,
                "ibc/uatom",
            )),
            spot_price: None
        }).unwrap(),
        exchange_rate: Decimal::one(),
        expected_error: None,
    };
    "SimulateSwapExactAssetOutWithMetadata Query - want out factory denom")]
#[test_case(
    Params {
        query: QueryMsg::SimulateSwapExactAssetOutWithMetadata {
            swap_operations: vec![],
            asset_out: Asset::Native(Coin::new(100, "ibc/uatom")),
            include_spot_price: false,
        },
        response: Binary([].to_vec()),
        exchange_rate: Decimal::one(),
        expected_error: Some(ContractError::UnsupportedDenom),
    };
    "SimulateSwapExactAssetOutWithMetadata Query - want out remote denom")]
#[test_case(
    Params {
        query: QueryMsg::SimulateSwapExactAssetOutWithMetadata {
            swap_operations: vec![],
            asset_out: Asset::Native(Coin::new(100, "ibc/uatom")),
            include_spot_price: true,
        },
        response: Binary([].to_vec()),
        exchange_rate: Decimal::from_atomics(cosmwasm_std::Uint128::new(5), 1).unwrap(),
        expected_error: Some(ContractError::UnsupportedDenom),
    };
    "SimulateSwapExactAssetOutWithMetadata Query include spot price - want out remote denom")]
#[test_case(
    Params {
        query: QueryMsg::SimulateSwapExactAssetOutWithMetadata {
            swap_operations: vec![],
            asset_out: Asset::Native(Coin::new(100, "factory/uatom")),
            include_spot_price: true,
        },
        response: to_json_binary(&skip::swap::SimulateSwapExactAssetOutResponse{
            asset_in: Asset::Native(Coin::new(
                200,
                "ibc/uatom",
            )),
            spot_price: Some(Decimal::from_atomics(cosmwasm_std::Uint128::new(5), 1).unwrap())
        }).unwrap(),
        exchange_rate: Decimal::from_atomics(cosmwasm_std::Uint128::new(5), 1).unwrap(),
        expected_error: None,
    };
    "SimulateSwapExactAssetOutWithMetadata Query include spot price - want out factory denom")]

fn test_queries(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps = mock_dependencies();

    let exchange_rate = params.exchange_rate;

    // Create mock wasm handler to handle the swap adapter contract query
    let wasm_handler = move |query: &WasmQuery| -> QuerierResult {
        match query {
            WasmQuery::Smart { contract_addr, .. } => {
                if contract_addr == "drop_core_contract" {
                    SystemResult::Ok(SystemContractResult::Ok(
                        to_json_binary(&exchange_rate).unwrap(),
                    ))
                } else {
                    panic!("Unsupported contract: {:?}", query);
                }
            }
            _ => panic!("Unsupported query: {:?}", query),
        }
    };

    // Update querier with mock wasm handler
    deps.querier.update_wasm(wasm_handler);

    // Create mock env
    let mut env = mock_env();
    env.contract.address = Addr::unchecked("swap_contract_address");

    // Store the drop contract address
    DROP_CORE_CONTRACT_ADDRESS.save(
        deps.as_mut().storage,
        &Addr::unchecked("drop_core_contract"),
    )?;

    // Store drop remote and bonded denoms
    IBC_REMOTE_DENOM.save(deps.as_mut().storage, &String::from("ibc/uatom"))?;
    FACTORY_BONDED_DENOM.save(deps.as_mut().storage, &String::from("factory/uatom"))?;

    // Call execute_swap with the given test parameters
    let res = skip_go_swap_adapter_drop::contract::query(deps.as_ref(), env, params.query.clone());

    match res {
        Ok(res) => {
            // Assert that we did not want an error
            assert!(
                params.expected_error.is_none(),
                "expected test to error with {:?}, but it succeeded",
                params.expected_error
            );

            // Assert the response is correct
            assert_eq!(res, params.response);
        }
        Err(err) => {
            // Asser that we did in fact expect an error
            assert!(
                params.expected_error.is_some(),
                "expected test to succeed, but it errored with {:?}",
                err
            ); 
            
            // Assert the error is correct
            assert_eq!(err, params.expected_error.unwrap());
        }
    }

    Ok(())
}
