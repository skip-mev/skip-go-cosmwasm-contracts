use pryzm_std::types::pryzm::amm::v1::SwapStep;
use skip_api_swap_adapter_pryzm::execution::SwapExecutionStep;

#[test]
fn test_execution_step_return_denom() {
    let step = SwapExecutionStep::Swap { swap_steps: vec![] };
    assert!(step.get_return_denom().is_err());

    let step = SwapExecutionStep::Swap { swap_steps: vec![
        SwapStep {
            pool_id: 1,
            token_in: "a".to_string(),
            token_out: "b".to_string(),
            amount: Some("1000".to_string())
        }
    ] };
    assert!(step.get_return_denom().is_ok());
    assert_eq!("b", step.get_return_denom().unwrap());

    let step = SwapExecutionStep::Swap { swap_steps: vec![
        SwapStep {
            pool_id: 1,
            token_in: "a".to_string(),
            token_out: "b".to_string(),
            amount: Some("1000".to_string())
        },
        SwapStep {
            pool_id: 3,
            token_in: "b".to_string(),
            token_out: "c".to_string(),
            amount: None
        },
        SwapStep {
            pool_id: 2,
            token_in: "c".to_string(),
            token_out: "d".to_string(),
            amount: None
        }
    ] };
    assert!(step.get_return_denom().is_ok());
    assert_eq!("d", step.get_return_denom().unwrap());

    let step = SwapExecutionStep::Stake {
        host_chain_id: "uatom".to_string(),
        transfer_channel: "channel-0".to_string(),
    };
    assert!(step.get_return_denom().is_ok());
    assert_eq!("c:uatom", step.get_return_denom().unwrap());
}
