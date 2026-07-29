# Dezswap Swap Adapter Contract

The Dezswap swap adapter contract is responsible for:
1. Taking the standardized entry point swap operations message format and converting it to Dezswap pair swap message format.
2. Swapping by dispatching swaps to Dezswap pair contracts.
3. Providing query methods that can be called by the entry point contract (generally, to any external actor) to simulate multi-hop swaps that either specify an exact amount in (estimating how much would be received from the swap) or an exact amount out (estimating how much is required to get the specified amount out).

Dezswap ([dezswap-contracts](https://github.com/dezswap/dezswap-contracts), [docs](https://docs.dezswap.io/docs/introduction/about/)) is an Astroport fork deployed on XPLA. This adapter depends on the [`dezswap`](https://crates.io/crates/dezswap) crate and dispatches `DezswapPoolSwap` (not `AstroportPoolSwap`) so that any future divergence is caught at compile time rather than silently at runtime. Unlike the current Astroport 2.9 / Oroswap pair interface, Dezswap's `pair::ExecuteMsg::Swap`, `Cw20HookMsg::Swap`, `QueryMsg::Simulation`, and `QueryMsg::ReverseSimulation` do not carry an `ask_asset_info`/`offer_asset_info` field (2-asset pools only) and instead add an optional `deadline` field, which this adapter always sets to `None`. Dezswap also does not export a `MAX_ALLOWED_SLIPPAGE` constant, so it is pinned locally to `"0.5"` (Astroport 2.9's value).

Note: Swap adapter contracts expect to be called by an entry point contract that provides basic validation and minimum amount out safety guarantees for the caller. There are no slippage guarantees provided by swap adapter contracts.

WARNING: Do not send funds directly to the contract without calling one of its functions. Funds sent directly to the contract do not trigger any contract logic that performs validation / safety checks (as the Cosmos SDK handles direct fund transfers in the `Bank` module and not the `Wasm` module). There are no explicit recovery mechanisms for accidentally sent funds.

## InstantiateMsg

Instantiates a new Dezswap swap adapter contract using the Entrypoint contract address provided in the instantiation message.

``` json
{
    "entry_point_contract_address": "xpla1..."
}
```

## ExecuteMsg

### `swap`

Swaps the coin sent using the operations provided. Each `pool` is a Dezswap pair contract address. The adapter dispatches a separate `DezswapPoolSwap` self-call per operation followed by `TransferFundsBack` at the end.

``` json
{
    "swap": {
        "operations": [
            {
                "pool": "xpla1...",
                "denom_in": "axpla",
                "denom_out": "cw20:xpla1..."
            }
        ]
    }
}
```

### `transfer_funds_back`

Transfers all contract funds to the address provided. Called by the swap adapter contract itself to send the assets received from swapping back to the entry point contract.

Note: This function can be called by anyone as the contract is assumed to have no balance before/after it's called by the entry point contract. Do not send funds directly to this contract without calling a function.

``` json
{
    "transfer_funds_back": {
        "swapper": "xpla1...",
        "return_denom": "cw20:xpla1..."
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
            "native": { "denom": "axpla", "amount": "200000" }
        },
        "swap_operations": [
            {
                "pool": "xpla1...",
                "denom_in": "cw20:xpla1...",
                "denom_out": "axpla"
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
            "native": { "denom": "axpla", "amount": "100" }
        },
        "swap_operations": [
            {
                "pool": "xpla1...",
                "denom_in": "axpla",
                "denom_out": "cw20:xpla1..."
            }
        ]
    }
}
```
