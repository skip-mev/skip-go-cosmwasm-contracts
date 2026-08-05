# Wasmswap Swap Adapter Contract

The wasmswap swap adapter contract is responsible for:

1. Taking the standardized entry point swap operations message format and converting it to wasmswap pool swap message format.
2. Swapping by dispatching swaps directly to each wasmswap pool contract in the route.
3. Providing query methods that can be called by the entry point contract (generally, to any external actor) to simulate multi-hop swaps that specify either an exact amount in or an exact amount out.

Note: Swap adapter contracts expect to be called by an entry point contract that provides basic validation and minimum amount out safety guarantees for the caller. There are no slippage guarantees provided by swap adapter contracts.

WARNING: Do not send funds directly to the contract without calling one of its functions. Funds sent directly to the contract do not trigger any contract logic that performs validation / safety checks (as the Cosmos SDK handles direct fund transfers in the `Bank` module and not the `Wasm` module). There are no explicit recovery mechanisms for accidentally sent funds.

## Protocol notes

Wasmswap (the AMM originally known as JunoSwap) differs from astroport / white-whale style AMMs in two ways that shape this adapter:

1. **There is no cw20 `Send` hook.** A pool pulls cw20 funds with `TransferFrom`, so a cw20 input requires an `IncreaseAllowance` on the token contract before the swap message. Native inputs are attached as funds.
2. **There is no reverse price query.** The pool only exposes `Token1ForToken2Price` / `Token2ForToken1Price` (exact amount in). `SimulateSwapExactAssetOut` therefore derives the required input from the pool's reserves and fee, inverting the pool's own constant product formula and rounding up.

Each pool has its own fee, which is read per hop from the pool's `Fee` query and converted to basis points (`lp_fee_percent + protocol_fee_percent`).

There is no router contract: every pool is a standalone contract, and `SwapOperation.pool` carries its address.

## InstantiateMsg

Instantiates a new wasmswap swap adapter contract using the entry point contract address provided in the instantiation message.

```json
{
  "entry_point_contract_address": "axm1..."
}
```

## ExecuteMsg

### `swap`

Swaps the coin sent using the operations provided.

```json
{
  "swap": {
    "operations": [
      {
        "pool": "axm1...",
        "denom_in": "uaxm",
        "denom_out": "axm1... (cw20 contract address)"
      }
    ]
  }
}
```

### `transfer_funds_back`

Transfers all of the contract's balance for the given denom back to the swapper. Can only be called by the contract itself.

### `wasm_swap_pool_swap`

Dispatches a single swap to a wasmswap pool. Can only be called by the contract itself.

## QueryMsg

### `simulate_swap_exact_asset_in`

Returns the asset received from swapping the given asset in through the given operations.

### `simulate_swap_exact_asset_out`

Returns the asset required to receive the given asset out through the given operations.

### `simulate_swap_exact_asset_in_with_metadata` / `simulate_swap_exact_asset_out_with_metadata`

Same as above, optionally including the route's spot price.

### `simulate_smart_swap_exact_asset_in` / `simulate_smart_swap_exact_asset_in_with_metadata`

Returns the asset received from swapping the given asset in over multiple routes.
