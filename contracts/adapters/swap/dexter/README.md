# Dexter Swap Adapter Contract

The Dexter swap adapter contract is responsible for:
1. Taking the standardized entry point swap operations message format and converting it to Dexter Router's swap operations.
2. Swapping by dispatching the multi-hop swap operations to the Dexter Router contract.
3. Providing query methods that can be called by the entry point contract (generally, to any external actor) to simulate multi-hop swaps that either specify an exact amount in (estimating how much would be received from the swap) or an exact amount out (estimating how much is required to get the specified amount out).

Note: Swap adapter contracts expect to be called by an entry point contract that provides basic validation and minimum amount out safety guarantees for the caller. There are no slippage guarantees provided by swap adapter contracts.

WARNING: Do not send funds directly to the contract without calling one of its functions. Funds sent directly to the contract do not trigger any contract logic that performs validation / safety checks (as the Cosmos SDK handles direct fund transfers in the `Bank` module and not the `Wasm` module). There are no explicit recovery mechanisms for accidentally sent funds.

## InstantiateMsg

Instantiates a new Dexter swap adapter contract using the Entrypoint contract address provided in the instantiation message.

``` json
{
    "entry_point_contract_address": "persistence1....",
    "dexter_vault_address": "persistence1....",
    "dexter_router_address": "persistence1...."
}
```

## ExecuteMsg

### `swap`

Swaps the coin sent using the operations provided.

``` json
{
    "swap": {
        "operations": [
            {
                "pool": "persistence...",
                "denom_in": "uxprt",
                "denom_out": "stk/uxprt"
            },
            {
                "pool": "persistence...",
                "denom_in": "stk/uxprt",
                "denom_out": "stk/uatom"
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
        "caller": "persistence..."
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
            "denom": "uxprt",
            "amount": "200000"
        },
        "swap_operations": [
            {
                "pool": "persistence...",
                "denom_in": "stk/uxprt",
                "denom_out": "uxprt"
            }
        ]
    }
}
```

Response:
``` json
{
    "denom": "stk/uxprt",
    "amount": "190000"
}
```

### `simulate_swap_exact_coin_in`

Returns the coin out that would be received from swapping the `coin_in` specified in the call (swapped through the `swap_operatons` provided)

Query:
``` json
{
    "simulate_swap_exact_coin_in": {
        "coin_in": {
            "denom": "uxprt",
            "amount": "2000000"
        },
        "swap_operations": [
            {
                "pool": "persistence...",
                "denom_in": "uxprt",
                "denom_out": "stk/uxprt"
            }
        ]
    }
}
```

Response:
``` json
{
    "denom": "stk/uxprt",
    "amount": "1900000"
}
```