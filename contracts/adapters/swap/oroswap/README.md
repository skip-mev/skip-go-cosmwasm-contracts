# Oroswap Swap Adapter Contract

The Oroswap swap adapter contract is responsible for:
1. Taking the standardized entry point swap operations message format and converting it to Oroswap pair swap message format.
2. Swapping by dispatching swaps to Oroswap pair contracts.
3. Providing query methods that can be called by the entry point contract (generally, to any external actor) to simulate multi-hop swaps that either specify an exact amount in (estimating how much would be received from the swap) or an exact amount out (estimating how much is required to get the specified amount out).

Oroswap is an Astroport fork deployed on ZIGChain. Although the upstream Astroport adapter served as a starting template, this adapter depends on the [`oroswap-core`](https://github.com/oroswap/oroswap-core) types and dispatches `OroswapPoolSwap` (not `AstroportPoolSwap`) so that any future divergence is caught at compile time rather than silently at runtime. The pair-level wire format (`ExecuteMsg::Swap`, `Cw20HookMsg::Swap`, `QueryMsg::Simulation`, `QueryMsg::ReverseSimulation`, `MAX_ALLOWED_SLIPPAGE`) is currently identical to Astroport 2.9.

Note: Swap adapter contracts expect to be called by an entry point contract that provides basic validation and minimum amount out safety guarantees for the caller. There are no slippage guarantees provided by swap adapter contracts.

WARNING: Do not send funds directly to the contract without calling one of its functions. Funds sent directly to the contract do not trigger any contract logic that performs validation / safety checks (as the Cosmos SDK handles direct fund transfers in the `Bank` module and not the `Wasm` module). There are no explicit recovery mechanisms for accidentally sent funds.

## InstantiateMsg

Instantiates a new Oroswap swap adapter contract using the Entrypoint contract address provided in the instantiation message.

``` json
{
    "entry_point_contract_address": "zig1..."
}
```

## ExecuteMsg

### `swap`

Swaps the coin sent using the operations provided. Each `pool` is an Oroswap pair contract address. The adapter dispatches a separate `OroswapPoolSwap` self-call per operation followed by `TransferFundsBack` at the end.

``` json
{
    "swap": {
        "operations": [
            {
                "pool": "zig1...",
                "denom_in": "uzig",
                "denom_out": "coin.zig1<issuer>.husky"
            },
            {
                "pool": "zig1...",
                "denom_in": "coin.zig1<issuer>.husky",
                "denom_out": "coin.zig1<issuer>.bitbull"
            }
        ]
    }
}
```

Note on denoms: native tokenfactory denoms on ZIGChain follow the `coin.<creator_addr>.<subdenom>` format (rather than the Cosmos SDK standard `factory/<creator>/<subdenom>`). LP tokens issued by Oroswap pairs are also native tokenfactory denoms, not CW20 contracts.

### `transfer_funds_back`

Transfers all contract funds to the address provided. Called by the swap adapter contract itself to send the assets received from swapping back to the entry point contract.

Note: This function can be called by anyone as the contract is assumed to have no balance before/after it's called by the entry point contract. Do not send funds directly to this contract without calling a function.

``` json
{
    "transfer_funds_back": {
        "swapper": "zig1...",
        "return_denom": "coin.zig1<issuer>.bitbull"
    }
}
```

## QueryMsg

### `simulate_swap_exact_asset_out`

Returns the asset in required to receive the `asset_out` specified in the call (swapped through the `swap_operations` provided). Each pair contract's `ReverseSimulation` query is called in reverse order.

``` json
{
    "simulate_swap_exact_asset_out": {
        "asset_out": {
            "native": { "denom": "uzig", "amount": "200000" }
        },
        "swap_operations": [
            {
                "pool": "zig1...",
                "denom_in": "coin.zig1<issuer>.husky",
                "denom_out": "uzig"
            }
        ]
    }
}
```

### `simulate_swap_exact_asset_in`

Returns the asset out that would be received from swapping the `asset_in` specified in the call (swapped through the `swap_operations` provided). Each pair contract's `Simulation` query is called in order.

``` json
{
    "simulate_swap_exact_asset_in": {
        "asset_in": {
            "native": { "denom": "uzig", "amount": "100" }
        },
        "swap_operations": [
            {
                "pool": "zig1...",
                "denom_in": "uzig",
                "denom_out": "coin.zig1<issuer>.husky"
            }
        ]
    }
}
```
