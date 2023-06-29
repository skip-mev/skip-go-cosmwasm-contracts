from bip_utils import Bip39SeedGenerator, Bip44, Bip44Coins

from cosmpy.aerial.client import LedgerClient, NetworkConfig
from cosmpy.aerial.contract import LedgerContract, create_cosmwasm_execute_msg, create_cosmwasm_instantiate_msg
from cosmpy.aerial.tx import Transaction, SigningCfg
from cosmpy.aerial.wallet import LocalWallet
from cosmpy.crypto.keypairs import PrivateKey
from cosmpy.protos.cosmos.base.v1beta1.coin_pb2 import Coin
from cosmpy.protos.cosmwasm.wasm.v1.tx_pb2 import MsgStoreCode

import os
import sys
import toml
from datetime import datetime

CHAIN = sys.argv[1]
NETWORK = sys.argv[2]
DEPLOYED_CONTRACTS_FOLDER_PATH = "../deployed-contracts"

# Match the CHAIN to the file name in the configs folder
found_config = False
for file in os.listdir("configs"):
    if file == f"{CHAIN}.toml":
        config = toml.load(f"configs/{file}")
        found_config = True
        break

# Raise exception if config not found
if not found_config:
    raise Exception(f"Could not find config for chain {CHAIN}; Must enter a chain as 1st command line argument.")

# Create deployed-contracts folder if it doesn't exist
if not os.path.exists(f"../deployed-contracts"):
   os.makedirs(f"../deployed-contracts")
   
# Create chain folder if it doesn't exist within deployed-contracts
if not os.path.exists(f"../deployed-contracts/{CHAIN}"):
    os.makedirs(f"../deployed-contracts/{CHAIN}")

# Choose network to deploy to based on cli args
if NETWORK == "mainnet":
    REST_URL = config["MAINNET_REST_URL"]
    CHAIN_ID = config["MAINNET_CHAIN_ID"]
elif NETWORK == "testnet":
    REST_URL = config["TESTNET_REST_URL"]
    CHAIN_ID = config["TESTNET_CHAIN_ID"]
else:
    raise Exception("Must specify either 'mainnet' or 'testnet' for 2nd command line argument.")

ADDRESS_PREFIX = config["ADDRESS_PREFIX"]
DENOM = config["DENOM"]
GAS_PRICE = config["GAS_PRICE"]

# Contract Paths
ENTRY_POINT_CONTRACT_PATH = config["ENTRY_POINT_CONTRACT_PATH"]
SWAP_ADAPTER_PATH = config["SWAP_ADAPTER_PATH"]
IBC_TRANSFER_ADAPTER_PATH = config["IBC_TRANSFER_ADAPTER_PATH"]

MNEMONIC = config["MNEMONIC"]
del config["MNEMONIC"]

DEPLOYED_CONTRACTS_INFO = {}

def main():
    # Create network config and client
    cfg = NetworkConfig(
        chain_id=CHAIN_ID,
        url=REST_URL,
        fee_minimum_gas_price=.01,
        fee_denomination=DENOM,
        staking_denomination=DENOM,
    )
    client = LedgerClient(cfg)

    # Create wallet from mnemonic
    wallet = create_wallet(client)
    
    # Initialize deployed contracts info
    init_deployed_contracts_info()
        
    # Get checksums for deployed contracts info
    with open("../artifacts/checksums.txt", "r") as f:
        checksums = f.read().split()
        
    # Store checksums for deployed contracts info
    for i in range(0, len(checksums), 2):
        DEPLOYED_CONTRACTS_INFO["checksums"][checksums[i+1]] = checksums[i]
        with open(f"{DEPLOYED_CONTRACTS_FOLDER_PATH}/{CHAIN}/{NETWORK}.toml", "w") as f:
            toml.dump(DEPLOYED_CONTRACTS_INFO, f)
    
    # Store contracts
    swap_adapter_contract_code_id = store_contract(client, wallet, SWAP_ADAPTER_PATH, "swap_adapter")
    ibc_transfer_adapter_contract_code_id = store_contract(client, wallet, IBC_TRANSFER_ADAPTER_PATH, "ibc_transfer_adapter")
    entry_point_contract_code_id = store_contract(client, wallet, ENTRY_POINT_CONTRACT_PATH, "entry_point")
        
    # Intantiate contracts
    if "router_contract_address" in config["swap_venues"][0]:
        swap_adapter_args = {"router_contract_address": f"{config['swap_venues'][0]['router_contract_address']}"}
    else:
        swap_adapter_args = {}
    swap_adapter_contract_address = instantiate_contract(
        client, 
        wallet, 
        swap_adapter_contract_code_id, 
        swap_adapter_args, 
        "Skip Swap Swap Adapter", 
        "swap_adapter"
    )
    ibc_transfer_adapter_contract_address = instantiate_contract(
        client, 
        wallet, 
        ibc_transfer_adapter_contract_code_id, 
        {}, 
        "Skip Swap IBC Transfer Adapter", 
        "ibc_transfer_adapter"
    )
    entry_point_contract_address = instantiate_contract(
        client=client, 
        wallet=wallet, 
        code_id=entry_point_contract_code_id, 
        args={
            "swap_venues": [
                {
                    "name": config["swap_venues"][0]["name"],
                    "adapter_contract_address": swap_adapter_contract_address,
                }
            ],
            "ibc_transfer_contract_address": ibc_transfer_adapter_contract_address,
        },
        label="Skip Swap Entry Point",
        name="entry_point"
    )
    
def create_tx(msg,
              client, 
              wallet, 
              gas_limit: int, 
              fee: str,
              ) -> tuple[bytes, str]:
    tx = Transaction()
    tx.add_message(msg)
    
    # Get account
    account = client.query_account(str(wallet.address()))
    
    # Seal, Sign, and Complete Tx
    tx.seal(signing_cfgs=[SigningCfg.direct(wallet.public_key(), account.sequence)], fee = fee, gas_limit=gas_limit)
    tx.sign(wallet.signer(), client.network_config.chain_id, account.number)
    tx.complete()
    
    return tx
    
def create_wasm_store_tx(client, 
                         wallet, 
                         address: str,
                         gas_fee: str,
                         gas_limit: int, 
                         file: str,
                         ) -> tuple[bytes, str]:
    msg = MsgStoreCode(
        sender=address,
        wasm_byte_code=open(file, "rb").read(),
        instantiate_permission=None
    )
    return create_tx(msg=msg, 
                     client=client, 
                     wallet=wallet, 
                     gas_limit=gas_limit,
                     fee=gas_fee)
    
def create_wasm_instantiate_tx(
                         client, 
                         wallet, 
                         address: str,
                         gas_fee: str,
                         gas_limit: int, 
                         code_id: int,
                         args: dict,
                         label: str,
                         ) -> tuple[bytes, str]:
    msg = create_cosmwasm_instantiate_msg(
        code_id=code_id,
        args=args,
        label=label,
        sender_address=address,
    )
    return create_tx(msg=msg, 
                     client=client, 
                     wallet=wallet, 
                     gas_limit=gas_limit,
                     fee=gas_fee)
    
def create_wasm_execute_tx(
                         client, 
                         wallet, 
                         contract_address: str,
                         args: dict,
                         address: str,
                         gas_fee: str,
                         gas_limit: int, 
                         funds_coin: Coin | None,
                         ) -> tuple[bytes, str]:
    msg = create_cosmwasm_execute_msg(
        contract_address=contract_address,
        args=args,
        sender_address=address
    )
    if funds_coin:
        msg.funds.append(funds_coin)
    return create_tx(msg=msg, 
                     client=client, 
                     wallet=wallet, 
                     gas_limit=gas_limit,
                     fee=gas_fee)
    
def create_wallet(client) -> LocalWallet:
    """ Create a wallet from a mnemonic and return it"""
    seed_bytes = Bip39SeedGenerator(MNEMONIC).Generate()
    bip44_def_ctx = Bip44.FromSeed(seed_bytes, Bip44Coins.COSMOS).DeriveDefaultPath()
    wallet = LocalWallet(PrivateKey(bip44_def_ctx.PrivateKey().Raw().ToBytes()), prefix=ADDRESS_PREFIX)  
    balance = client.query_bank_balance(str(wallet.address()), DENOM)
    print("Wallet Address: ", wallet.address(), " with account balance: ", balance)
    return wallet

def init_deployed_contracts_info():
    DEPLOYED_CONTRACTS_INFO["info"] = {}
    DEPLOYED_CONTRACTS_INFO["info"]["chain_id"] = CHAIN_ID
    DEPLOYED_CONTRACTS_INFO["info"]["network"] = NETWORK
    DEPLOYED_CONTRACTS_INFO["info"]["deploy_date"] = datetime.now().strftime("%d/%m/%Y %H:%M:%S")
    DEPLOYED_CONTRACTS_INFO["info"]["commit_hash"] = config["COMMIT_HASH"]
    DEPLOYED_CONTRACTS_INFO["checksums"] = {}
    DEPLOYED_CONTRACTS_INFO["code-ids"] = {}
    DEPLOYED_CONTRACTS_INFO["contract-addresses"] = {}
    DEPLOYED_CONTRACTS_INFO["tx-hashes"] = {}
    with open(f"{DEPLOYED_CONTRACTS_FOLDER_PATH}/{CHAIN}/{NETWORK}.toml", "w") as f:
        toml.dump(DEPLOYED_CONTRACTS_INFO, f)

def store_contract(client, wallet, file_path, name) -> int:
    gas_limit = 3000000
    store_ibc_adapter_tx = create_wasm_store_tx(
        client=client,
        wallet=wallet,
        address=str(wallet.address()),
        gas_fee=f"{int(GAS_PRICE*gas_limit)}{DENOM}",
        gas_limit=gas_limit,
        file=file_path,
    )
    submitted_tx = client.broadcast_tx(store_ibc_adapter_tx)
    print("Tx hash: ", submitted_tx.tx_hash)
    submitted_tx.wait_to_complete(timeout=60)
    contract_code_id = submitted_tx.contract_code_id
    print(f"Skip Swap {name} Contract Code ID:", submitted_tx.contract_code_id)
    DEPLOYED_CONTRACTS_INFO["code-ids"][f"{name}_contract_code_id"] = contract_code_id
    DEPLOYED_CONTRACTS_INFO["tx-hashes"][f"store_{name}_tx_hash"] = submitted_tx.tx_hash
    with open(f"{DEPLOYED_CONTRACTS_FOLDER_PATH}/{CHAIN}/{NETWORK}.toml", "w") as f:
        toml.dump(DEPLOYED_CONTRACTS_INFO, f)
    return int(contract_code_id)

def instantiate_contract(client, wallet, code_id, args, label, name) -> str:
    gas_limit = 200000
    instantiate_swap_adapter_tx = create_wasm_instantiate_tx(
        client=client,
        wallet=wallet,
        address=str(wallet.address()),
        gas_fee=f"{int(GAS_PRICE*gas_limit)}{DENOM}",
        gas_limit=gas_limit,
        code_id=code_id,
        args=args,
        label=label
    )
    submitted_tx = client.broadcast_tx(instantiate_swap_adapter_tx)
    print("Tx hash: ", submitted_tx.tx_hash)
    submitted_tx.wait_to_complete(timeout=60)
    contract_address = submitted_tx.contract_address.__str__()
    print(f"Skip Swap {name} Contract Address:", contract_address)
    DEPLOYED_CONTRACTS_INFO["contract-addresses"][f"{name}_contract_address"] = contract_address
    DEPLOYED_CONTRACTS_INFO["tx-hashes"][f"instantiate_{name}_tx_hash"] = submitted_tx.tx_hash
    with open(f"{DEPLOYED_CONTRACTS_FOLDER_PATH}/{CHAIN}/{NETWORK}.toml", "w") as f:
        toml.dump(DEPLOYED_CONTRACTS_INFO, f)
    return contract_address
    
if __name__ == "__main__":
    main()