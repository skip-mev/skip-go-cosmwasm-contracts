use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, ContractInfo,
};
use skip_go_secret_entry_point::{
    error::ContractError,
    msg::{InstantiateMsg, SwapVenue},
    state::{BLOCKED_CONTRACT_ADDRESSES, IBC_TRANSFER_CONTRACT, SWAP_VENUE_MAP},
};
use test_case::test_case;

/*
Test Cases:

Expect Response
    - Happy Path (tests the adapter and blocked contract addresses are stored correctly)

Expect Error
    - Duplicate Swap Venue Names
 */

// Define test parameters
struct Params {
    swap_venues: Vec<SwapVenue>,
    ibc_transfer_contract: ContractInfo,
    viewing_key: String,
    expected_error: Option<ContractError>,
}

// Test instantiate
#[test_case(
    Params {
        swap_venues: vec![
            SwapVenue {
                name: "shade-swap".to_string(),
                adapter_contract: ContractInfo {
                    address: Addr::unchecked("secret123".to_string()),
                    code_hash: "code_hash".to_string(),
                },
            },
        ],
        ibc_transfer_contract: ContractInfo {
            address: Addr::unchecked("ibc_transfer_adapter".to_string()),
            code_hash: "code_hash".to_string(),
        },
        viewing_key: "viewing_key".to_string(),
        expected_error: None,
    };
    "Happy Path")]
#[test_case(
    Params {
        swap_venues: vec![
            SwapVenue {
                name: "shade-swap".to_string(),
                adapter_contract: ContractInfo {
                    address: Addr::unchecked("secret123".to_string()),
                    code_hash: "code_hash".to_string(),
                },
            },
            SwapVenue {
                name: "shade-swap".to_string(),
                adapter_contract: ContractInfo {
                    address: Addr::unchecked("secret123".to_string()),
                    code_hash: "code_hash".to_string(),
                },
            },
        ],
        ibc_transfer_contract: ContractInfo {
            address: Addr::unchecked("ibc_transfer_adapter".to_string()),
            code_hash: "code_hash".to_string(),
        },
        viewing_key: "viewing_key".to_string(),
        expected_error: Some(ContractError::DuplicateSwapVenueName),
    };
    "Duplicate Swap Venue Names")]
fn test_instantiate(params: Params) {
    // Create mock dependencies
    let mut deps = mock_dependencies();

    // Create mock info
    let info = mock_info("creator", &[]);

    // Create mock env with the entry point contract address
    let mut env = mock_env();
    env.contract.address = Addr::unchecked("entry_point");

    // Call instantiate with the given test parameters
    let res = skip_go_secret_entry_point::contract::instantiate(
        deps.as_mut(),
        env,
        info,
        InstantiateMsg {
            swap_venues: params.swap_venues.clone(),
            ibc_transfer_contract: params.ibc_transfer_contract,
            viewing_key: params.viewing_key,
        },
    );

    match res {
        Ok(_) => {
            // Assert the test did not expect an error
            assert!(
                params.expected_error.is_none(),
                "expected test to error with {:?}, but it succeeded",
                params.expected_error
            );

            // Assert the entry point contract address exists in the blocked contract addresses map
            assert!(BLOCKED_CONTRACT_ADDRESSES
                .has(deps.as_ref().storage, &Addr::unchecked("entry_point")));

            // Get stored ibc transfer adapter contract address
            let stored_ibc_transfer_contract =
                IBC_TRANSFER_CONTRACT.load(deps.as_ref().storage).unwrap();

            // Assert the ibc transfer adapter contract address exists in the blocked contract addresses map
            assert!(BLOCKED_CONTRACT_ADDRESSES
                .has(deps.as_ref().storage, &stored_ibc_transfer_contract.address));

            params.swap_venues.into_iter().for_each(|swap_venue| {
                // Get stored swap venue adapter contract address
                let stored_swap_venue_contract = SWAP_VENUE_MAP
                    .may_load(deps.as_ref().storage, &swap_venue.name)
                    .unwrap()
                    .unwrap();

                // Assert the swap venue name exists in the map and that
                // the adapter contract address stored is correct
                assert_eq!(&stored_swap_venue_contract, &swap_venue.adapter_contract);

                // Assert the swap adapter contract address exists in the blocked contract addresses map
                assert!(BLOCKED_CONTRACT_ADDRESSES
                    .has(deps.as_ref().storage, &stored_swap_venue_contract.address));
            });
        }
        Err(err) => {
            // Assert the test expected an error
            assert!(
                params.expected_error.is_some(),
                "expected test to succeed, but it errored with {:?}",
                err
            );

            // Assert the error is correct
            assert_eq!(err, params.expected_error.unwrap());
        }
    }
}
