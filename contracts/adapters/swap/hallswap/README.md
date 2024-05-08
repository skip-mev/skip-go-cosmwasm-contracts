# Hallswap Adapter Contract

The Hallswap adapter contract is responsible for:

1. Taking the standardized entry point swap operations message format and converting it to Hallswap entry point swap execute message.
2. Swapping by dispatching the multi-hop swap operations to the Hallswap contract.
3. Providing query methods that can be called by the entry point contract (generally, to any external actor) to simulate multi-hop swaps that either specify an exact amount in (estimating how much would be received from the swap).

Note: Swap adapter contracts expect to be called by an entry point contract that provides basic validation and minimum amount out safety guarantees for the caller. There are no slippage guarantees provided by swap adapter contracts.

WARNING: Do not send funds directly to the contract without calling one of its functions. Funds sent directly to the contract do not trigger any contract logic that performs validation / safety checks (as the Cosmos SDK handles direct fund transfers in the `Bank` module and not the `Wasm` module). There are no explicit recovery mechanisms for accidentally sent funds.

## InstantiateMsg

Instantiates a new Dexter swap adapter contract using the Entrypoint contract address provided in the instantiation message.

``` json
{
    "entry_point_contract_address": "terra1....",
    "hallswap_contract_address": "terra1...."
}
```

## ExecuteMsg

### `swap`

Swaps the coin sent using the operations provided.

``` json
{
    "swap": {
        "offer_asset": {
            "native": {
                "denom": "uluna", 
                "amount": "10000"
            }
        },
        "operations": [
            {
                "pool": "terra...",
                "denom_in": "uluna",
                "denom_out": "ibc/..."
            },
            {
                "pool": "terra...",
                "denom_in": "ibc/...",
                "denom_out": "factory/..."
            }
        ]
    }
}
```

## QueryMsg

### `simulate_swap_exact_coin_in`

Returns the coin out that would be received from swapping the `coin_in` specified in the call (swapped through the `swap_operatons` provided)

Query:

``` json
{
    "simulate_swap_exact_coin_in": {
        "asset_in": {
            "native": {
                "denom": "uluna",
                "amount": "2000000"
            }
        },
        "swap_operations": [
            {
                "pool": "terra...",
                "denom_in": "uluna",
                "denom_out": "ibc/..."
            },
            {
                "pool": "terra...",
                "denom_in": "ibc/...",
                "denom_out": "factory/..."
            }
        ]
    }
}
```

Response:

``` json
{
    "native": {
        "denom": "factory/...",
        "amount": "1900000"
    }
}
```
