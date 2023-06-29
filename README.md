![Skip Swap](skip_swirl.png "Skipping and Swapping")

# Skip API Contracts

The contracts in this repository are used in [Skip API](https://api-swagger.skip.money/) to enable any-to-any swaps as part of multi-chain workflows.

Skip API is a unified REST API + SDK that helps developers create more seamless cross-chain experiences for their end users with IBC [(Inter-Blockchain Communication protocol)](https://ibcprotocol.dev/). 

Skip API is designed so that even developers who are new to IBC can offer incredible cross-chain experiences, like swaps and transfers between any two IBC-enabled chains and tokens in as few transactions as possible, with reliable multi-chain relaying, packet tracking, and more.

# Overview

The on-chain components of the swapping functionality consist of:
1. A main entry point contract
2. Chain/dex-specific swap adapter contracts 
3. Chain-specific IBC transfer adapter contracts

## Entry Point Contract

The entry point contract is responsible for providing a standardized interface (w/ safety checks) to interact with Skip Swap across all CosmWasm-enabled chains. The contract:
1. Performs basic validation on the call data
2. If a fee swap is provided, queries the swap adapter contract to determine how much of the coin sent with the contract call is needed to receive the required fee coin(s), and dispatches the swap.
3. Dispatches the user swap provided in the call data to the relevant swap adapter contract.
4. Verifies the amount out received from the swap(s) is greater than the minimum amount required by the caller after all fees have been subtracted (swap, ibc, affiliate)
5. Dispatches one of the following post-swap actions with the received funds from the swap:
    - Transfer to an address on the same chain 
    - IBC transfer to an address on a different chain (which allows for multi-hop IBC transfers or contract calls if the destination chains support it)
    - Call a contract on the same chain

## Swap Adapter Contracts

Swap Adapter contracts are developed and deployed for each swap venue supported by Skip Swap. The contracts are responsible for:
1. Taking the standardized Skip Swap entry point message format and converting it to the specific swap venue's format
2. Swapping by calling the swap venue's respective smart contract or module
3. Providing query methods that can be called by the entry point contract (generally, to any external actor) to simulate multi-hop swaps that either specify an exact amount in (estimating how much would be received from the swap) or an exact amount out (estimating how much is required to get the specified amount out)

## IBC Transfer Adapter Contracts

IBC Transfer adapter contracts are developed and deployed for each chain supported by Skip Swap. The contracts are responsible for:
1. Dispatching the IBC transfer (with the appropriate IBC fees if required)
2. Failing the entire transaction if the IBC transfer errors on the swap chain (sending the caller back their original funds)
3. Refunding the caller on the swap chain if the IBC transfer errors or times out once it reaches the destination chain (also refunding unused IBC fees)

# Repository Structure

The repository is organized in the following way:
```
│
├── contracts/              <- Contains all contracts
│   ├── entry-point/        <- Contains source code and tests for entry point contract
│   └── networks/           <- Contains source code and tests for all network adapter contracts
│       ├── neutron/
│       │   ├── ibc-transfer/
│       │   └── swap/
│       └── osmosis/
│           ├── ibc-transfer/
│           └── swap/
│
├── deployed-contracts/     <- Contains deployed contracts info for each network
│   ├── neutron/
│   └── osmosis/
│
├── packages/               <- Contains all package code used by the contracts
│   └── skip/
│
├── scripts/                <- Contains all configs and deployment scripts
│   ├── configs/
│   ├── deploy.py
│   └── requirements.txt
│
├── README.md
├── Cargo.lock
├── Cargo.toml
├── Makefile
└── README.md
```

# Testing

All tests can be found in the tests folder in each respective contract package.

Run all tests in the repo:
```bash
make test
```

Note: Due to the nature of the adapter contracts using stargate messages and interacting with chain-specific modules, integration testing is conducted on the respective testnets. See Deployment section for deployment instructions.

# Development Processes

The repository's CI is triggered on pull requests and will fail if any error or warnings appear running the `check`, `clippy`, and `fmt` commands found in the Makefile.

Each command and how to run them are as follows:

`cargo check --target wasm32-unknown-unknown` is used to compile the contracts and verify they are valid wasm:
``` bash
make check
```

`clippy` is used for linting:
``` bash
make clippy
```

`rustfmt` is used for formatting:
``` bash
make fmt
```

# Deployment

To deploy the Skip Swap contracts, the steps are as follows:

1. Build the optimized wasm bytecode of the contracts by running (they will appear in an artifacts folder):

    ``` bash
    make optimize
    ```

2. Ensure you have python 3.10 installed to run the deploy script. Download python 3.10 if you don't have it installed.
    ``` bash
    python3.10 --version
    ```

3. Go into the scripts directory and create a virtual environment to download the python dependencies:
    ``` bash
    cd scripts
    python3.10 -m venv venv
    ```

4. Activate virtual environment, (venv) will show on left-hand side of shell
    ``` bash
    source venv/bin/activate
    ```

5. Install all the dependencies:
    ```
    pip install -r requirements.txt
    ```

6. Add the mnemonic of the deployer address in the respsective chain's config toml file (located in configs folder):

    ``` toml
    # Enter your mnemonic here
    MNEMONIC = "<YOUR MNEMONIC HERE>"
    ```

7. Run the deploy script with the following format (changing the chain [options: osmosis, neutron] and network [options: testnet, mainnet] depending on what is to be deployed):
    ``` bash
    python deploy.py <CHAIN> <NETWORK>
    ```

    Example:
    ``` bash
    python deploy.py osmosis testnet
    ```

8. After running the deploy script, a toml file will be added/updated in the deployed-contracts/{CHAIN} folder with all relevant info for the deployment.

# About Skip

Skip helps developers provide extraordinary user experiences across all stages of the transaction lifecycle, from transaction construction, through cross-chain relaying + tracking, to block construction.