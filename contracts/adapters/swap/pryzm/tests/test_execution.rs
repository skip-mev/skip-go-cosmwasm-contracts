use cosmwasm_std::{coin, CosmosMsg};
use pryzm_std::types::cosmos::base::v1beta1::Coin as CosmosCoin;
use pryzm_std::types::pryzm::{
    amm::v1::{MsgBatchSwap, SwapStep, SwapType},
    icstaking::v1::MsgStake,
};

use skip::swap::SwapOperation;
use skip_go_swap_adapter_pryzm::error::ContractError;
use skip_go_swap_adapter_pryzm::execution::{extract_execution_steps, SwapExecutionStep};

#[test]
fn test_execution_step_return_denom() {
    // empty swap steps
    let step = SwapExecutionStep::Swap { swap_steps: vec![] };
    assert!(step.get_return_denom().is_err());

    // single step Swap type step
    let step = SwapExecutionStep::Swap {
        swap_steps: vec![SwapStep {
            pool_id: 1,
            token_in: "a".to_string(),
            token_out: "b".to_string(),
            amount: Some("1000".to_string()),
        }],
    };
    assert!(step.get_return_denom().is_ok());
    assert_eq!("b", step.get_return_denom().unwrap());

    // multistep Swap type step
    let step = SwapExecutionStep::Swap {
        swap_steps: vec![
            SwapStep {
                pool_id: 1,
                token_in: "a".to_string(),
                token_out: "b".to_string(),
                amount: Some("1000".to_string()),
            },
            SwapStep {
                pool_id: 3,
                token_in: "b".to_string(),
                token_out: "c".to_string(),
                amount: None,
            },
            SwapStep {
                pool_id: 2,
                token_in: "c".to_string(),
                token_out: "d".to_string(),
                amount: None,
            },
        ],
    };
    assert!(step.get_return_denom().is_ok());
    assert_eq!("d", step.get_return_denom().unwrap());

    // stake step
    let step = SwapExecutionStep::Stake {
        host_chain_id: "uatom".to_string(),
        transfer_channel: "channel-0".to_string(),
    };
    assert!(step.get_return_denom().is_ok());
    assert_eq!("c:uatom", step.get_return_denom().unwrap());
}

#[test]
fn test_execution_step_cosmos_msg() {
    let address = "address";

    // empty swap steps
    let step = SwapExecutionStep::Swap { swap_steps: vec![] };
    assert!(step
        .to_cosmos_msg(address.to_string(), coin(1000, "a"))
        .is_err());

    // single step Swap type step
    let step = SwapExecutionStep::Swap {
        swap_steps: vec![SwapStep {
            pool_id: 1,
            token_in: "a".to_string(),
            token_out: "b".to_string(),
            amount: None,
        }],
    };
    let result = step.to_cosmos_msg(address.to_string(), coin(1000, "a"));
    assert!(result.is_ok());
    assert_eq!(
        <MsgBatchSwap as Into<CosmosMsg>>::into(MsgBatchSwap {
            creator: address.to_string(),
            swap_type: SwapType::GivenIn.into(),
            max_amounts_in: vec![CosmosCoin {
                amount: "1000".to_string(),
                denom: "a".to_string()
            }],
            min_amounts_out: vec![CosmosCoin {
                amount: "1".to_string(),
                denom: "b".to_string()
            }],
            steps: vec![SwapStep {
                pool_id: 1,
                token_in: "a".to_string(),
                token_out: "b".to_string(),
                amount: "1000".to_string().into()
            }],
        }),
        result.unwrap()
    );

    // multistep Swap type step
    let step = SwapExecutionStep::Swap {
        swap_steps: vec![
            SwapStep {
                pool_id: 1,
                token_in: "a".to_string(),
                token_out: "b".to_string(),
                amount: None,
            },
            SwapStep {
                pool_id: 3,
                token_in: "b".to_string(),
                token_out: "c".to_string(),
                amount: None,
            },
            SwapStep {
                pool_id: 2,
                token_in: "c".to_string(),
                token_out: "d".to_string(),
                amount: None,
            },
        ],
    };
    let result = step.to_cosmos_msg(address.to_string(), coin(1000, "a"));
    assert!(result.is_ok());
    assert_eq!(
        <MsgBatchSwap as Into<CosmosMsg>>::into(MsgBatchSwap {
            creator: address.to_string(),
            swap_type: SwapType::GivenIn.into(),
            max_amounts_in: vec![CosmosCoin {
                amount: "1000".to_string(),
                denom: "a".to_string()
            }],
            min_amounts_out: vec![CosmosCoin {
                amount: "1".to_string(),
                denom: "d".to_string()
            }],
            steps: vec![
                SwapStep {
                    pool_id: 1,
                    token_in: "a".to_string(),
                    token_out: "b".to_string(),
                    amount: "1000".to_string().into()
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
            ],
        }),
        result.unwrap()
    );

    // stake step
    let step = SwapExecutionStep::Stake {
        host_chain_id: "uatom".to_string(),
        transfer_channel: "channel-0".to_string(),
    };
    let result = step.to_cosmos_msg(address.to_string(), coin(1000, "uatom"));
    assert!(result.is_ok());
    assert_eq!(
        <MsgStake as Into<CosmosMsg>>::into(MsgStake {
            creator: address.to_string(),
            host_chain: "uatom".to_string(),
            transfer_channel: "channel-0".to_string(),
            amount: "1000".into(),
        }),
        result.unwrap()
    );
}

#[test]
fn test_extract_execution_step() {
    // empty swap operations
    let result = extract_execution_steps(vec![]);
    assert!(result.is_err());

    // single amm swap step
    let result = extract_execution_steps(vec![SwapOperation {
        pool: "amm:1".to_string(),
        denom_in: "a".to_string(),
        denom_out: "b".to_string(),
        interface: None,
    }]);
    assert!(result.is_ok());
    let vec = result.unwrap();
    assert_eq!(1, vec.len());
    assert_eq!(
        SwapExecutionStep::Swap {
            swap_steps: vec![SwapStep {
                pool_id: 1,
                token_in: "a".to_string(),
                token_out: "b".to_string(),
                amount: None
            }]
        },
        vec.front().unwrap().clone()
    );

    // multi step amm swap
    let result = extract_execution_steps(vec![
        SwapOperation {
            pool: "amm:1".to_string(),
            denom_in: "a".to_string(),
            denom_out: "b".to_string(),
            interface: None,
        },
        SwapOperation {
            pool: "amm:3".to_string(),
            denom_in: "b".to_string(),
            denom_out: "c".to_string(),
            interface: None,
        },
        SwapOperation {
            pool: "amm:2".to_string(),
            denom_in: "c".to_string(),
            denom_out: "d".to_string(),
            interface: None,
        },
    ]);
    assert!(result.is_ok());
    let vec = result.unwrap();
    assert_eq!(1, vec.len());
    assert_eq!(
        SwapExecutionStep::Swap {
            swap_steps: vec![
                SwapStep {
                    pool_id: 1,
                    token_in: "a".to_string(),
                    token_out: "b".to_string(),
                    amount: None
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
            ]
        },
        vec.front().unwrap().clone()
    );

    // single staking step
    let result = extract_execution_steps(vec![SwapOperation {
        pool: "icstaking:uatom:channel-0".to_string(),
        denom_in: "uatom".to_string(),
        denom_out: "c:uatom".to_string(),
        interface: None,
    }]);
    assert!(result.is_ok());
    let vec = result.unwrap();
    assert_eq!(1, vec.len());
    assert_eq!(
        SwapExecutionStep::Stake {
            host_chain_id: "uatom".to_string(),
            transfer_channel: "channel-0".to_string(),
        },
        vec.front().unwrap().clone()
    );

    // multiple steps including amm and icstaking
    let result = extract_execution_steps(vec![
        SwapOperation {
            pool: "amm:7".to_string(),
            denom_in: "uusdc".to_string(),
            denom_out: "uauuu".to_string(),
            interface: None,
        },
        SwapOperation {
            pool: "amm:12".to_string(),
            denom_in: "uauuu".to_string(),
            denom_out: "uatom".to_string(),
            interface: None,
        },
        SwapOperation {
            pool: "icstaking:uatom:channel-0".to_string(),
            denom_in: "uatom".to_string(),
            denom_out: "c:uatom".to_string(),
            interface: None,
        },
        SwapOperation {
            pool: "amm:1".to_string(),
            denom_in: "c:uatom".to_string(),
            denom_out: "y:uatom:30Sep2024".to_string(),
            interface: None,
        },
    ]);
    assert!(result.is_ok());
    let mut vec = result.unwrap();
    assert_eq!(3, vec.len());
    assert_eq!(
        SwapExecutionStep::Swap {
            swap_steps: vec![
                SwapStep {
                    pool_id: 7,
                    token_in: "uusdc".to_string(),
                    token_out: "uauuu".to_string(),
                    amount: None
                },
                SwapStep {
                    pool_id: 12,
                    token_in: "uauuu".to_string(),
                    token_out: "uatom".to_string(),
                    amount: None
                },
            ]
        },
        vec.pop_front().unwrap().clone()
    );
    assert_eq!(
        SwapExecutionStep::Stake {
            host_chain_id: "uatom".to_string(),
            transfer_channel: "channel-0".to_string(),
        },
        vec.pop_front().unwrap().clone()
    );
    assert_eq!(
        SwapExecutionStep::Swap {
            swap_steps: vec![SwapStep {
                pool_id: 1,
                token_in: "c:uatom".to_string(),
                token_out: "y:uatom:30Sep2024".to_string(),
                amount: None
            },]
        },
        vec.pop_front().unwrap().clone()
    );

    // invalid pools
    let result = extract_execution_steps(vec![SwapOperation {
        pool: "amm:invalid".to_string(),
        denom_in: "a".to_string(),
        denom_out: "b".to_string(),
        interface: None,
    }]);
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        ContractError::InvalidPool { .. }
    ));

    let result = extract_execution_steps(vec![SwapOperation {
        pool: "invalid:1".to_string(),
        denom_in: "a".to_string(),
        denom_out: "b".to_string(),
        interface: None,
    }]);
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        ContractError::InvalidPool { .. }
    ));

    let result = extract_execution_steps(vec![SwapOperation {
        pool: "icstaking:1".to_string(),
        denom_in: "uatom".to_string(),
        denom_out: "c:uatom".to_string(),
        interface: None,
    }]);
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        ContractError::InvalidPool { .. }
    ));

    let result = extract_execution_steps(vec![SwapOperation {
        pool: "icstaking:uatom:channel-0".to_string(),
        denom_in: "c:uatom".to_string(),
        denom_out: "uatom".to_string(),
        interface: None,
    }]);
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        ContractError::InvalidPool { .. }
    ));

    let result = extract_execution_steps(vec![SwapOperation {
        pool: "icstaking:uatom:channel-0".to_string(),
        denom_in: "c:uatom".to_string(),
        denom_out: "c:uosmo".to_string(),
        interface: None,
    }]);
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        ContractError::InvalidPool { .. }
    ));

    let result = extract_execution_steps(vec![SwapOperation {
        pool: "icstaking:uatom:channel-0".to_string(),
        denom_in: "uatom".to_string(),
        denom_out: "uosmo".to_string(),
        interface: None,
    }]);
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        ContractError::InvalidPool { .. }
    ));

    let result = extract_execution_steps(vec![SwapOperation {
        pool: "icstaking:uatom:channel-0:some".to_string(),
        denom_in: "uatom".to_string(),
        denom_out: "c:uatom".to_string(),
        interface: None,
    }]);
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        ContractError::InvalidPool { .. }
    ));
}
