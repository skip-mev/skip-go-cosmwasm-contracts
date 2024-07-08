use skip_api_swap_adapter_pryzm::swap::SwapExecutionStep;

#[test]
fn test_execution_step_return_denom() {
    let step = SwapExecutionStep::Swap { swap_steps: vec![] };
    assert_eq!(true, step.get_return_denom().is_err())
}
