# Placeholder Contract

The Placeholder Contract is a contract that has an instantiate method, and nothing else. The main use case for this contract is to instantiate a specific contract address, later migrating the contract to the real contract desired.

WARNING: Do not send funds directly to the contract without calling one of its functions. Funds sent directly to the contract do not trigger any contract logic that performs validation / safety checks (as the Cosmos SDK handles direct fund transfers in the `Bank` module and not the `Wasm` module). There are no explicit recovery mechanisms for accidentally sent funds.

## InstantiateMsg

Instantiates a new Placeholder contract.

``` json
{}
```