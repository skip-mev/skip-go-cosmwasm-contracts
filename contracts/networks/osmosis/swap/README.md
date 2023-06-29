# Osmosis Poolmanager Swap Adapter Contract

The Osmosis Poolmanager swap adapter contract is responsible for:
1. Taking the standardized entry point swap operations message format and converting it to the Osmosis Poolmanager `SwapAmountInRoute` format.
2. Swapping by calling the Osmosis Poolmanager module.
3. Providing query methods that can be called by the entry point contract (generally, to any external actor) to simulate multi-hop swaps that either specify an exact amount in (estimating how much would be received from the swap) or an exact amount out (estimating how much is required to get the specified amount out).

Note: Swap adapter contracts expect to be called by an entry point contract that provides basic validation and minimum amount out safety guarantees for the caller. There are no slippage guarantees provided by swap adapter contracts.

WARNING: Do not send funds directly to the contract without calling one of its functions. Funds sent directly to the contract do not trigger any contract logic that performs validation / safety checks (as the Cosmos SDK handles direct fund transfers in the `Bank` module and not the `Wasm` module). There are no explicit recovery mechanisms for accidentally sent funds.

## InstantiateMsg

Instantiates a new Osmosis Poolmanager swap adapter contract.

``` json
{}
```

## ExecuteMsg

### `swap`

Swaps the coin sent using the operations provided.

Note: The `pool` string field provided in the operations must be able to be converted into a `u64` (the format used by Osmosis for pool IDs)

``` json
{
    "swap": {
        "operations": [
            {
                "pool": "1",
                "denom_in": "uosmo",
                "denom_out": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"
            },
            {
                "pool": "2",
                "denom_in": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                "denom_out": "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4"
            }
        ]
    }
}
```

### `transfer_funds_back`

Transfers all contract funds to the address provided, called by the swap adapter contract to send back the entry point contract the assets received from swapping.

Note: This function can be called by anyone as the contract is assumed to have no balance before/after it's called by the entry point contract. Do not send funds directly to this contract without calling a function.

``` json
{
    "transfer_funds_back": {
        "caller": "osmo..."
    }
}
```

## QueryMsg

### `simulate_swap_exact_coin_out`

Returns the coin in required to receive the `coin_out` specified in the call (swapped through the `swap_operatons` provided)

Query:
``` json
{
    "simulate_swap_exact_coin_out": {
        "coin_out": {
            "denom": "uosmo",
            "amount": "200000"
        },
        "swap_operations": [
            {
                "pool": "1",
                "denom_in": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                "denom_out": "uosmo"
            }
        ]
    }
}
```

Response:
``` json
{
    "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
    "amount": "100"
}
```

### `simulate_swap_exact_coin_in`

Returns the coin out that would be received from swapping the `coin_in` specified in the call (swapped through the `swap_operatons` provided)

Query:
``` json
{
    "simulate_swap_exact_coin_in": {
        "coin_in": {
            "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
            "amount": "100"
        },
        "swap_operations": [
            {
                "pool": "1",
                "denom_in": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                "denom_out": "uosmo"
            }
        ]
    }
}
```

Response:
``` json
{
    "denom": "uosmo",
    "amount": "100000"
}
```