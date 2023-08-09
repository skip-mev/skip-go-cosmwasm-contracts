# Entry Point Contract

The entry point contract is responsible for providing a standardized interface (w/ safety checks) to interact with Skip Swap across all CosmWasm-enabled chains. The contract:
1. Performs basic validation on the call data
2. If a fee swap is provided, queries the swap adapter contract to determine how much of the coin sent with the contract call is needed to receive the required fee coin(s), and dispatches the swap.
3. Dispatches the user swap provided in the call data to the relevant swap adapter contract.
4. Handles affiliate fee payments if provided.
5. Verifies the amount out received from the swap(s) is greater than the minimum amount required by the caller after all fees have been subtracted (swap, ibc, affiliate)
6. Dispatches one of the following post-swap actions with the received funds from the swap:
    - Transfer to an address on the same chain 
    - IBC transfer to an address on a different chain (which allows for multi-hop IBC transfers or contract calls if the destination chains support it)
    - Call a contract on the same chain

WARNING: Do not send funds directly to the entry point contract without calling one of its functions. Funds sent directly to the contract do not trigger any contract logic that performs validation / safety checks (as the Cosmos SDK handles direct fund transfers in the `Bank` module and not the `Wasm` module). There are no explicit recovery mechanisms for accidentally sent funds.

## InstantiateMsg

Instantiates a new entry point contract using the adapter contracts provided in the instantiation message.

``` json
{
    "swap_venues": [
        {
            "name": "neutron-astroport",
            "adapter_contract_address": "neutron..."
        }
    ],
    "ibc_transfer_contract_address": "neutron..."
}
```

## ExecuteMsg

### `swap_and_action`

Swaps the coin sent and performs a post-swap action.

Optional fields:
- `fee_swap` is used if a fee is required by the IBC transfer.

Notes:
- Only one coin can be sent to the contract when calling `swap_and_action` otherwise the transaction will fail.
- `timeout_timestamp` is Unix epoch time in nanoseconds. The transaction will fail if the `timeout_timestamp` has passed when the contract is called.
- `post_swap_action` can be one of three actions: `bank_send`, `ibc_transfer`, or `contract_call`. 
  - `bank_send`: Sends the assets received from the `user_swap` to an address on the same chain the swap occured on.
  - `ibc_transfer`: ICS-20 transfers the assets received from the swap(s) to an address on a different chain than the swap occured on. The ICS-20 transfer supports including a memo in the outgoing transfer, allowing for multi-hop transfers via Packet Forward Middleware and/or contract calls via IBC-hooks.
  - `contract_call`: Calls a contract on the same chain the swap occured, using the assets received from the swap as the contract call's funds.
- `affiliates` is a list of affiliates that will take a fee (in basis points) from the `min_coin` provided. If no affiliates are associated with a call then an empty list is to be provided.
- The vector of coins provided in `ibc_info.fee` must all be the same denom.
- A `fee_swap` is only valid if the `post_swap_action` is an `ibc_transfer` with a provided `ibc_info.fee`. The `coin_out` used for the fee swap is dervied from the provided `ibc_info.fee`.
- The `coin_in` used in the `user_swap` is derived based on the coin sent to the contract from the user's contract call, after accounting for the fee swap and if the `user_swap` is a `SwapExactCoinIn` or `SwapExactCoinOut`

#### Examples

SwapExactCoinIn:

``` json
{
    "swap_and_action": {
        "user_swap": {
            "swap_exact_coin_in": {
                "swap_venue_name": "neutron-astroport",
                "operations": [
                    {
                        "pool": "neutron...",
                        "denom_in": "uatom",
                        "denom_out": "untrn"
                    },
                    {
                        "pool": "neutron...",
                        "denom_in": "untrn",
                        "denom_out": "uosmo"
                    }
                ]
            },
        },
        "min_coin": {
            "denom": "uosmo",
            "amount": "1000000"
        },
        "timeout_timestamp": 1000000000000,
        "post_swap_action": {
            "ibc_transfer": {
                "ibc_info": {
                    "source_channel": "channel-1",
                    "receiver": "cosmos...",
                    "fee": {
                        "recv_fee": [],
                        "ack_fee": [
                            {
                                "denom": "untrn",
                                "amount": "100"
                            }
                        ],
                        "timeout_fee": [
                            {
                                "denom": "untrn",
                                "amount": "100"
                            }
                        ]
                    },
                    "memo": "",
                    "recover_address": "neutron..."
                }
                "fee_swap": {
                    "swap_venue_name": "neutron-astroport",
                    "operations": [
                        {
                            "pool": "neutron...",
                            "denom_in": "uatom",
                            "denom_out": "untrn"
                        }
                    ]
                },
            }
        },
        "affiliates": [
            {
                "basis_points_fee": 10,
                "address": "neutron..."
            }
        ]
    }
}
```

SwapExactCoinOut:

``` json
{
    "swap_and_action": {
        "user_swap": {
            "swap_exact_coin_out": {
                "swap_venue_name": "neutron-astroport",
                "operations": [
                    {
                        "pool": "neutron...",
                        "denom_in": "uatom",
                        "denom_out": "untrn"
                    },
                    {
                        "pool": "neutron...",
                        "denom_in": "untrn",
                        "denom_out": "uosmo"
                    }
                ],
                "refund_address": "neutron..."
            },
        },
        "min_coin": {
            "denom": "uosmo",
            "amount": "1000000"
        },
        "timeout_timestamp": 1000000000000,
        "post_swap_action": {
            "bank_send": {
                "to_address": "neutron..."
            }
        },
        "affiliates": [
            {
                "basis_points_fee": 10,
                "address": "neutron..."
            }
        ]
    }
}
```

### `user_swap`

Dispatches the user swap to the relevant swap adapter contract and affiliate fee bank send messages. If the user swap is a `SwapExactCoinOut` it also dispatches the refund bank send message to the provided `refund_address`

Note: Can only be called by the entry point contract itself, any external calls to this function will fail.

``` json
{
    "user_swap": {
        "swap": {
            "swap_exact_coin_out": {
                "swap_venue_name": "neutron-astroport",
                "operations": [
                    {
                        "pool": "neutron...",
                        "denom_in": "uatom",
                        "denom_out": "untrn"
                    },
                    {
                        "pool": "neutron...",
                        "denom_in": "untrn",
                        "denom_out": "uosmo"
                    }
                ],
                "refund_address": "neutron..."
            },
        },
        "min_coin": {
            "denom": "uosmo",
            "amount": "1000000"
        },
        "remaining_coin": {
            "denom": "uatom",
            "amount": "100000"
        },
        "affiliates": []
    }
}
```

### `post_swap_action`

Performs a post swap action.

Note: Can only be called by the entry point contract itself, any external calls to this function will fail.

``` json
{
    "post_swap_action": {
        "min_coin": {
            "denom": "uosmo",
            "amount": "1000000"
        },
        "timeout_timestamp": 1000000000000,
        "post_swap_action": {
            "bank_send": {
                "to_address": "neutron..."
            }
        },
        "exact_out": false,
    }
}
```

## QueryMsg

### `swap_venue_adapter_contract`

Returns the swap adapter contract set at instantiation for the given swap venue name provided as an argument.

Query:
``` json
{
    "swap_venue_adapter_contract": {
        "name": "neutron-astroport"
    }
}
```

Response:
``` json
"neutron..."
```

### `ibc_transfer_adapter_contract`

Returns the IBC transfer adapter contract set at instantiation, requires no arguments.

Query:
``` json
{
    "ibc_transfer_adapter_contract": {}
}
```

Response:
``` json
"neutron..."
```