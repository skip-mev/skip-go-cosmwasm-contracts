# Neutron Astrovault Swap Adapter Contract

The Neutron Astrovault swap adapter contract is responsible for:

1. Taking the standardized entry point swap operations message format and converting it to Astrovault pool swaps message format.
2. Swapping by dispatching swaps to Astrovault router contract.
3. Providing query methods that can be called by the entry point contract (generally, to any external actor) to simulate multi-hop swaps that specify an exact amount in (estimating how much would be received from the swap)

Note: Swap adapter contracts expect to be called by an entry point contract that provides basic validation and minimum amount out safety guarantees for the caller. There are no slippage guarantees provided by swap adapter contracts.

WARNING: Do not send funds directly to the contract without calling one of its functions. Funds sent directly to the contract do not trigger any contract logic that performs validation / safety checks (as the Cosmos SDK handles direct fund transfers in the `Bank` module and not the `Wasm` module). There are no explicit recovery mechanisms for accidentally sent funds.

## InstantiateMsg

Instantiates a new Neutron Astrovault swap adapter contract using the Entrypoint contract address provided in the instantiation message.

```json
{
  "entry_point_contract_address": "neutron...",
  "astrovault_router_address": "neutron..."
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
        "pool": "neutron...",
        "denom_in": "ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81",
        "denom_out": "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9"
      },
      {
        "pool": "neutron...",
        "denom_in": "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9",
        "denom_out": "untrn"
      }
    ]
  }
}
```

### `transfer_funds_back`

Transfers all contract funds to the address provided, called by the swap adapter contract to send back the entry point contract the assets received from swapping.

Note: This function can be called by anyone as the contract is assumed to have no balance before/after it's called by the entry point contract. Do not send funds directly to this contract without calling a function.

```json
{
  "transfer_funds_back": {
    "caller": "neutron..."
  }
}
```

## QueryMsg

### `simulate_swap_exact_coin_in`

Returns the coin out that would be received from swapping the `coin_in` specified in the call (swapped through the `swap_operatons` provided)

Query:

```json
{
  "simulate_swap_exact_coin_in": {
    "coin_in": {
      "denom": "untrn",
      "amount": "1000000"
    },
    "swap_operations": [
      {
        "pool": "neutron...",
        "denom_in": "untrn",
        "denom_out": "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9"
      }
    ]
  }
}
```

Response:

```json
{
  "denom": "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9",
  "amount": "1000"
}
```
