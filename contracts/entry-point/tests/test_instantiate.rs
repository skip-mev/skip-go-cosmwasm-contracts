use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr,
};
use skip::{entry_point::InstantiateMsg, swap::SwapVenue};
use skip_swap_entry_point::{error::ContractError, state::SWAP_VENUE_MAP};
use test_case::test_case;

/*
Test Cases:

Expect Response
    - Happy Path (tests the adapter contracts are stored correctly)

Expect Error
    - Duplicate Swap Venue Names
 */

// Define test parameters
struct Params {
    swap_venues: Vec<SwapVenue>,
    ibc_transfer_contract_address: String,
    expected_error: Option<ContractError>,
}

// Test instantiate
#[test_case(
    Params {
        swap_venues: vec![
            SwapVenue {
                name: "neutron-astroport".to_string(),
                adapter_contract_address: "neutron123".to_string(),
            },
            SwapVenue {
                name: "osmosis-poolmanager".to_string(),
                adapter_contract_address: "osmosis123".to_string(),
            },
        ],
        ibc_transfer_contract_address: "ibc_transfer_adapter".to_string(),
        expected_error: None,
    };
    "Happy Path")]
#[test_case(
    Params {
        swap_venues: vec![
            SwapVenue {
                name: "neutron-astroport".to_string(),
                adapter_contract_address: "neutron123".to_string(),
            },
            SwapVenue {
                name: "neutron-astroport".to_string(),
                adapter_contract_address: "neutron456".to_string(),
            },
        ],
        ibc_transfer_contract_address: "ibc_transfer_adapter".to_string(),
        expected_error: Some(ContractError::DuplicateSwapVenueName),
    };
    "Duplicate Swap Venue Names")]
fn test_instantiate(params: Params) {
    // Create mock dependencies
    let mut deps = mock_dependencies();

    // Create mock info
    let info = mock_info("creator", &[]);

    // Call instantiate with the given test parameters
    let res = skip_swap_entry_point::contract::instantiate(
        deps.as_mut(),
        mock_env(),
        info,
        InstantiateMsg {
            swap_venues: params.swap_venues.clone(),
            ibc_transfer_contract_address: params.ibc_transfer_contract_address,
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

            params.swap_venues.into_iter().for_each(|swap_venue| {
                // Assert the swap venue name exists in the map and that
                // the adapter contract address stored is correct
                assert_eq!(
                    SWAP_VENUE_MAP
                        .may_load(deps.as_ref().storage, &swap_venue.name)
                        .unwrap()
                        .unwrap(),
                    Addr::unchecked(swap_venue.adapter_contract_address)
                );
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
