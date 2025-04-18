# Elys AMM Swap Adapter Contract

The Elys AMM swap adapter contract is responsible for:
1. Taking the standardized entry point swap operations message format and converting it to the Elys amm `SwapAmountInRoute` format.
2. Swapping by calling the Elys amm module.
3. Providing query methods that can be called by the entry point contract (generally, to any external actor) to simulate multi-hop swaps that either specify an exact amount in (estimating how much would be received from the swap) or an exact amount out (estimating how much is required to get the specified amount out).

Note: Swap adapter contracts expect to be called by an entry point contract that provides basic validation and minimum amount out safety guarantees for the caller. There are no slippage guarantees provided by swap adapter contracts.

WARNING: Do not send funds directly to the contract without calling one of its functions. Funds sent directly to the contract do not trigger any contract logic that performs validation / safety checks (as the Cosmos SDK handles direct fund transfers in the `Bank` module and not the `Wasm` module). There are no explicit recovery mechanisms for accidentally sent funds.

## InstantiateMsg

Instantiates a new Elys AMM swap adapter contract.

``` json
{
   "entry_point_contract_address": "elys..."
}
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
                "denom_in": "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9",
                "denom_out": "ibc/F082B65C88E4B6D5EF1DB243CDA1D331D002759E938A0F5CD3FFDC5D53B3E349"
            },
            {
                "pool": "2",
                "denom_in": "ibc/F082B65C88E4B6D5EF1DB243CDA1D331D002759E938A0F5CD3FFDC5D53B3E349",
                "denom_out": "ibc/45D6B52CAD911A15BD9C2F5FFDA80E26AFCB05C7CD520070790ABC86D2B24229"
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
        "caller": "elys..."
    }
}
```

## QueryMsg

### `simulate_swap_exact_asset_out`

Returns the asset in required to receive the `asset_out` specified in the call (swapped through the `swap_operatons` provided)

Query:
``` json
{
    "simulate_swap_exact_asset_out": {
        "asset_out": {
            "denom": "ibc/F082B65C88E4B6D5EF1DB243CDA1D331D002759E938A0F5CD3FFDC5D53B3E349",
            "amount": "300000"
        },
        "swap_operations": [
            {
                "pool": "4",
                "denom_in": "uelys",
                "denom_out": "ibc/F082B65C88E4B6D5EF1DB243CDA1D331D002759E938A0F5CD3FFDC5D53B3E349"
            }
        ]
    }
}
```

Response:
``` json
{
    "denom": "uelys",
    "amount": "900000"
}
```

### `simulate_swap_exact_asset_in`

Returns the asset out that would be received from swapping the `asset_in` specified in the call (swapped through the `swap_operatons` provided)

Query:
``` json
{
    "simulate_swap_exact_asset_in": {
        "asset_in": {
            "denom": "uelys",
            "amount": "300"
        },
        "swap_operations": [
            {
                "pool": "4",
                "denom_in": "uelys",
                "denom_out": "ibc/F082B65C88E4B6D5EF1DB243CDA1D331D002759E938A0F5CD3FFDC5D53B3E349"
            }
        ]
    }
}
```

Response:
``` json
{
    "denom": "ibc/F082B65C88E4B6D5EF1DB243CDA1D331D002759E938A0F5CD3FFDC5D53B3E349",
    "amount": "100"
}
```