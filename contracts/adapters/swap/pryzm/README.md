# PRYZM Swap Adapter Contract

The Pryzm swap adapter contract is responsible for:

1. Taking the standardized entry point swap operations message format and converting it to the respective messages on
   Pryzm.
2. Swapping on Pryzm's [AMM](https://docs.pryzm.zone/core/amm) or liquid staking on
   Pryzm's [ICStaking](https://docs.pryzm.zone/core/icstaking) module.
3. Providing query methods that can be called by the entry point contract (generally, to any external actor) to simulate
   multi-hop swaps that either specify an exact amount in (estimating how much would be received from the swap) or an
   exact amount out (estimating how much is required to get the specified amount out).

Note: Swap adapter contracts expect to be called by an entry point contract that provides basic validation and minimum
amount out safety guarantees for the caller. There are no slippage guarantees provided by swap adapter contracts.

WARNING: Do not send funds directly to the contract without calling one of its functions. Funds sent directly to the
contract do not trigger any contract logic that performs validation / safety checks (as the Cosmos SDK handles direct
fund transfers in the `Bank` module and not the `Wasm` module). There are no explicit recovery mechanisms for
accidentally sent funds.

## InstantiateMsg

Instantiates a new Pryzm swap adapter contract.

``` json
{
   "entry_point_contract_address": ""
}
```

## ExecuteMsg

### `swap`

Swaps the coin sent using the operations provided.

Note: The `pool` string field provided in the operations must have the following format:

* For AMM swap, it must be "amm:" appended with a valid `u64` pool id, i.e: `amm:1`
* For liquid staking, it must be "icstaking:" appended with a valid registered host chain id and the transfer channel,
  i.e: `icstaking:uatom:channel-0`

``` json
{
    "swap": {
        "operations": [
            {
                "pool": "amm:1",
                "denom_in": "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4",
                "denom_out": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"
            },
            {
                "pool": "icstaking:uatom:channel-0",
                "denom_in": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                "denom_out": "c:uatom"
            }
        ]
    }
}
```

### `transfer_funds_back`

Transfers all contract funds to the address provided, called by the swap adapter contract to send back the entry point
contract the assets received from swapping.

Note: This function can be called by anyone as the contract is assumed to have no balance before/after it's called by
the entry point contract. Do not send funds directly to this contract without calling a function.

``` json
{
    "transfer_funds_back": {
        "swapper": "pryzm...",
        "return_denom": "c:uatom"
    }
}
```

## QueryMsg

### `simulate_swap_exact_asset_in`

Returns the asset_out that would be received from swapping the `asset_in` specified in the call (swapped through
the `swap_operatons` provided)

Query:

``` json
{
    "simulate_swap_exact_asset_in": {
        "asset_in": {
            "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
            "amount": "100"
        },
        "swap_operations": [
            {
                "pool": "amm:1",
                "denom_in": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                "denom_out": "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4"
            }
        ]
    }
}
```

Response:

``` json
{
    "denom": "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4",
    "amount": "100000"
}
```

### `simulate_swap_exact_asset_out`

Returns the asset_in required to receive the `asset_out` specified in the call (swapped through the `swap_operatons`
provided)

Query:

``` json
{
    "simulate_swap_exact_asset_out": {
        "asset_out": {
            "denom": "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4",
            "amount": "100000"
        },
        "swap_operations": [
            {
                "pool": "amm:1",
                "denom_in": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                "denom_out": "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4"
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

### `simulate_swap_exact_asset_in_with_metadata`

Similar to `simulate_swap_exact_asset_in`, but also includes swap spot price if requested.

Query:

``` json
{
    "simulate_swap_exact_asset_in_with_metadata": {
        "asset_in": {
            "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
            "amount": "100"
        },
        "swap_operations": [
            {
                "pool": "amm:1",
                "denom_in": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                "denom_out": "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4"
            }
        ]
    }
}
```

Response:

``` json
{
    "denom": "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4",
    "amount": "100000"
}
```

### `simulate_swap_exact_asset_out_with_metadata`

Similar to `simulate_swap_exact_asset_out`, but also includes swap spot price if requested.

Query:

``` json
{
    "simulate_swap_exact_asset_out_with_metadata": {
        "asset_out": {
            "denom": "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4",
            "amount": "100000"
        },
        "swap_operations": [
            {
                "pool": "amm:1",
                "denom_in": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                "denom_out": "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4"
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

### `simulate_smart_swap_exact_asset_in`

Returns the asset_out that would be received from swapping an asset through multiple routes (the asset_in amount is 
divided into multiple parts and each part is swapped using a different route)

Query:

``` json
{
    "simulate_swap_exact_asset_in": {
        "asset_in": {
            "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
            "amount": "100"
        },
        "routes": [
            {
                "offer_asset": {
                    "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                    "amount": "70"
                },
                "swap_operations": [
                    {
                        "pool": "amm:1",
                        "denom_in": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                        "denom_out": "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4"
                    }
                ]       
            },
            {
                "offer_asset": {
                    "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                    "amount": "30"
                },
                "swap_operations": [
                    {
                        "pool": "amm:1",
                        "denom_in": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                        "denom_out": "c:uatom"
                    },
                    {
                        "pool": "amm:5",
                        "denom_in": "c:uatom",
                        "denom_out": "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4"
                    }
                ]       
            }
        ]
    }
}
```

Response:

``` json
{
    "denom": "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4",
    "amount": "100000"
}
```

### `simulate_smart_swap_exact_asset_in_with_metadata`

Similar to `simulate_smart_swap_exact_asset_in`, but also return the swap weighted spot price if requested.

Query:

``` json
{
    "simulate_smart_swap_exact_asset_in_with_metadata": {
        "asset_in": {
            "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
            "amount": "100"
        },
        "routes": [
            {
                "offer_asset": {
                    "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                    "amount": "70"
                },
                "swap_operations": [
                    {
                        "pool": "amm:1",
                        "denom_in": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                        "denom_out": "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4"
                    }
                ]       
            },
            {
                "offer_asset": {
                    "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                    "amount": "30"
                },
                "swap_operations": [
                    {
                        "pool": "amm:3",
                        "denom_in": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                        "denom_out": "c:uatom"
                    },
                    {
                        "pool": "amm:5",
                        "denom_in": "c:uatom",
                        "denom_out": "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4"
                    }
                ]       
            }
        ]
    }
}
```

Response:

``` json
{
    "denom": "ibc/987C17B11ABC2B20019178ACE62929FE9840202CE79498E29FE8E5CB02B7C0A4",
    "amount": "100000",
    "spot_price": "1000"
}
```