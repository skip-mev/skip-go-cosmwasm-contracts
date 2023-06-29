# Entry Point Contract

The entry point contract is responsible for providing a standardized interface (w/ safety checks) to interact with Skip Swap across all CosmWasm-enabled chains. The contract:
1. Performs basic validation on the call data
2. Dispatches the swaps provided in the call data to the relevant swap adapter contracts
3. Verifies the amount out received from the swaps is greater than the minimum amount required by the caller after all fees have been subtracted (swap, ibc, affiliate)
4. Dispatches one of the following post-swap actions with the received funds from the swap:
    - Transfer to an address on the same chain 
    - IBC transfer to an address on a different chain (which allows for multi-hop IBC transfers or contract calls if the destination chains support it)
    - Call a contract on the same chain

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
- `timeout_timestamp` is Unix epoch time in nanoseconds. The transaction will fail if the `timeout_timestamp` has passed when the contract is called.
- The `coin_in` field in `user_swap` is optional. If provided, the contract will attempt to swap the provided `coin_in`. If not provided, the contract will attempt to swap the coin that was sent to the contract by the caller minus the amount of that coin used by the `fee_swap`.
- `post_swap_action` can be one of three actions: `bank_send`, `ibc_transfer`, or `contract_call`. 
  - `bank_send`: Sends the assets received from the `user_swap` to an address on the same chain the swap occured on.
  - `ibc_transfer`: ICS-20 transfers the assets received from the swap(s) to an address on a different chain than the swap occured on. The ICS-20 transfer supports including a memo in the outgoing transfer, allowing for multi-hop transfers via Packet Forward Middleware and/or contract calls via IBC-hooks.
  - `contract_call`: Calls a contract on the same chain the swap occured, using the assets received from the swap as the contract call's funds.
- `affiliates` is a list of affiliates that will take a fee (in basis points) from the coin received from the `user_swap`. If no affiliates are associated with a call then an empty list is to be provided.

``` json
{
  "fee_swap": {
    "swap_venue_name": "neutron-astroport",
    "coin_out": {
      "denom": "untrn",
      "amount": "200000"
    },
    "operations": [
      {
        "pool": "neutron...",
        "denom_in": "uatom",
        "denom_out": "untrn"
      }
    ]
  },
  "user_swap": {
    "swap_venue_name": "neutron-astroport",
    "coin_in": {
      "denom": "uatom",
      "amount": "1000000"
    },
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
```

### `post_swap_action`

Performs a post swap action.

Note: Can only be called by the entry point contract itself, any external calls to this function will fail.

``` json
{
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
```

## QueryMsg

### `swap_venue_adapter_contract`

Returns the swap adapter contract set at instantiation for the given swap venue name provided as an argument.

``` json
{
  "name": "neutron-astroport"
}
```

### `ibc_transfer_adapter_contract`

Returns the IBC transfer adapter contract set at instantiation, requires no arguments.

``` json
{}
```