import os
import sys
import toml
import httpx
import time
from hashlib import sha256
from base64 import b64encode
from datetime import datetime
from bip_utils import Bip39SeedGenerator, Bip44, Bip44Coins
from google.protobuf import any_pb2

from cosmpy.aerial.client import LedgerClient, NetworkConfig
from cosmpy.aerial.tx import Transaction, SigningCfg
from cosmpy.aerial.wallet import LocalWallet
from cosmpy.crypto.keypairs import PrivateKey
from cosmpy.protos.cosmwasm.wasm.v1.tx_pb2 import (
    MsgStoreCode, 
    MsgInstantiateContract, 
    MsgInstantiateContract2,
    MsgMigrateContract,
    MsgUpdateAdmin,
    )
from cosmpy.common.utils import json_encode
from cosmpy.protos.cosmos.authz.v1beta1.tx_pb2 import MsgExec
from terra_sdk.client.lcd import LCDClient
from terra_sdk.key.mnemonic import MnemonicKey

CHAIN = sys.argv[1]
NETWORK = sys.argv[2]
DEPLOYED_CONTRACTS_FOLDER_PATH = "../deployed-contracts"

if CHAIN == "injective":
    raise Exception(
        "Injective is not supported in deploy.py. Use deploy_injective.py instead."
    )

# Match the CHAIN to the file name in the configs folder
found_config = False
for file in os.listdir("configs"):
    if file == f"{CHAIN}.toml":
        config = toml.load(f"configs/{file}")
        found_config = True
        break

# Raise exception if config not found
if not found_config:
    raise Exception(
        f"Could not find config for chain {CHAIN}; Must enter a chain as 1st cli arg."
    )

# Create deployed-contracts folder if it doesn't exist
if not os.path.exists("../deployed-contracts"):
   os.makedirs("../deployed-contracts")
   
# Create chain folder if it doesn't exist within deployed-contracts
if not os.path.exists(f"../deployed-contracts/{CHAIN}"):
    os.makedirs(f"../deployed-contracts/{CHAIN}")
    
PERMISSIONED_UPLOADER_ADDRESS = None

# Choose network to deploy to based on cli args
if NETWORK == "mainnet":
    REST_URL = config["MAINNET_REST_URL"]
    RPC_URL = config["MAINNET_RPC_URL"]
    CHAIN_ID = config["MAINNET_CHAIN_ID"]
    if "PERMISSIONED_UPLOADER_ADDRESS" in config:
        PERMISSIONED_UPLOADER_ADDRESS = config["PERMISSIONED_UPLOADER_ADDRESS"]
    SWAP_VENUES = config["swap_venues"]
elif NETWORK == "testnet":
    REST_URL = config["TESTNET_REST_URL"]
    RPC_URL = config["TESTNET_RPC_URL"]
    CHAIN_ID = config["TESTNET_CHAIN_ID"]
    SWAP_VENUES = config["testnet_swap_venues"]
else:
    raise Exception(
        "Must specify either 'mainnet' or 'testnet' for 2nd cli arg."
    )

ADDRESS_PREFIX = config["ADDRESS_PREFIX"]
DENOM = config["DENOM"]
GAS_PRICE = config["GAS_PRICE"]

# Contract Paths
ENTRY_POINT_CONTRACT_PATH = config["ENTRY_POINT_CONTRACT_PATH"]
IBC_TRANSFER_ADAPTER_PATH = config["IBC_TRANSFER_ADAPTER_PATH"]
if CHAIN == "sei":
    PLACEHOLDER_CONTRACT_PATH = config["PLACEHOLDER_CONTRACT_PATH"]

# SALT
SALT = config["SALT"].encode("utf-8")

# Pregenerated Contract Addresses
ENTRY_POINT_PRE_GENERATED_ADDRESS = config["ENTRY_POINT_PRE_GENERATED_ADDRESS"]

# Admin address for future migrations
ADMIN_ADDRESS = config["ADMIN_ADDRESS"]

MNEMONIC = config["MNEMONIC"]
del config["MNEMONIC"]

DEPLOYED_CONTRACTS_INFO = {}

def main():
    # Create network config and client
    cfg = NetworkConfig(
        chain_id=CHAIN_ID,
        url=f"rest+{REST_URL}",
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
            
    # Check if chain doesn't support instantiate2
    supports_instantiate2 = True
    if "NO_INSTANTIATE2" in config:
        supports_instantiate2 = False
            
    # IBC Contracts
    if supports_instantiate2:
        # Store and instantiate IBC transfer adapter contract
        ibc_transfer_adapter_contract_code_id = store_contract(
            client, 
            wallet, 
            IBC_TRANSFER_ADAPTER_PATH, 
            "ibc_transfer_adapter", 
            PERMISSIONED_UPLOADER_ADDRESS
        )
        ibc_transfer_adapter_contract_address = instantiate_contract(
            client, 
            wallet, 
            ADMIN_ADDRESS,
            ibc_transfer_adapter_contract_code_id, 
            {"entry_point_contract_address": ENTRY_POINT_PRE_GENERATED_ADDRESS}, 
            "Skip Swap IBC Transfer Adapter", 
            "ibc_transfer_adapter"
        )
    else:
        if CHAIN != "sei":
            raise Exception(
                "Sei is the only supported chain that doesn't support instantiate2."
            )
        # Store and instantiate placeholder contract
        ibc_placeholder_contract_code_id = store_contract(
            client, 
            wallet, 
            PLACEHOLDER_CONTRACT_PATH, 
            "ibc_transfer_adapter", 
            PERMISSIONED_UPLOADER_ADDRESS
        )
        ibc_transfer_adapter_contract_address = instantiate_contract(
            client, 
            wallet, 
            str(wallet.address()),
            ibc_placeholder_contract_code_id, 
            {}, 
            "Skip Swap IBC Transfer Adapter", 
            "ibc_transfer_adapter"
        )
    
    entry_point_instantiate_args = {
        "swap_venues": [],
        "ibc_transfer_contract_address": ibc_transfer_adapter_contract_address,
    }
    
    # Swap Contracts
    for venue in SWAP_VENUES:
        if supports_instantiate2:
            swap_adapter_contract_code_id = store_contract(
                client, 
                wallet, 
                venue["swap_adapter_path"], 
                f"swap_adapter_{venue['name']}", 
                PERMISSIONED_UPLOADER_ADDRESS
            )
            swap_adapter_instantiate_args = {
                "entry_point_contract_address": ENTRY_POINT_PRE_GENERATED_ADDRESS
            }
            if "lido_satellite_contract_address" in venue:
                swap_adapter_instantiate_args["lido_satellite_contract_address"] = venue["lido_satellite_contract_address"]
            if "hallswap_contract_address" in venue:
                swap_adapter_instantiate_args["hallswap_contract_address"] = venue["hallswap_contract_address"]
            if "dexter_vault_contract_address" in venue:
                swap_adapter_instantiate_args["dexter_vault_contract_address"] = venue["dexter_vault_contract_address"]
            if "dexter_router_contract_address" in venue:
                swap_adapter_instantiate_args["dexter_router_contract_address"] = venue["dexter_router_contract_address"]
            
            swap_adapter_contract_address = instantiate_contract(
                client, 
                wallet, 
                ADMIN_ADDRESS,
                swap_adapter_contract_code_id, 
                swap_adapter_instantiate_args, 
                f"Skip Swap Swap Adapter {venue['name']}", 
                f"swap_adapter_{venue['name']}"
            )
            
            entry_point_instantiate_args["swap_venues"].append(
                {
                    "name": venue["name"],
                    "adapter_contract_address": swap_adapter_contract_address,
                }
            )
        else:
            # Store and instantiate placeholder contract
            swap_placeholder_contract_code_id = store_contract(
                client, 
                wallet, 
                PLACEHOLDER_CONTRACT_PATH, 
                f"swap_adapter_{venue['name']}", 
                PERMISSIONED_UPLOADER_ADDRESS
            )
            swap_adapter_contract_address = instantiate_contract(
                client, 
                wallet, 
                str(wallet.address()),
                swap_placeholder_contract_code_id, 
                {}, 
                f"Skip Swap Swap Adapter {venue['name']}", 
                f"swap_adapter_{venue['name']}"
            )
            # Add swap adapter contract address to entry point instantiate args
            entry_point_instantiate_args["swap_venues"].append(
                {
                    "name": venue["name"],
                    "adapter_contract_address": swap_adapter_contract_address,
                }
            )
    
    # Entry Point Contract
    entry_point_contract_code_id = store_contract(
        client, 
        wallet, 
        ENTRY_POINT_CONTRACT_PATH, 
        "entry_point", 
        PERMISSIONED_UPLOADER_ADDRESS
    )
    
    if supports_instantiate2:
        instantiate2_contract(
            client=client, 
            wallet=wallet, 
            code_id=entry_point_contract_code_id, 
            args=entry_point_instantiate_args,
            label="Skip Swap Entry Point",
            name="entry_point",
            pre_gen_address=ENTRY_POINT_PRE_GENERATED_ADDRESS
        )
    else:
        entry_point_contract_address = instantiate_contract(
            client, 
            wallet, 
            ADMIN_ADDRESS,
            entry_point_contract_code_id, 
            entry_point_instantiate_args, 
            "Skip Swap Entry Point", 
            "entry_point"
        )
        
        # Store IBC transfer adapter contract
        ibc_transfer_adapter_contract_code_id = store_contract(
            client, 
            wallet, 
            IBC_TRANSFER_ADAPTER_PATH, 
            "ibc_transfer_adapter", 
            PERMISSIONED_UPLOADER_ADDRESS
        )
        
        # Migrate IBC transfer adapter contract
        ibc_transfer_adapter_contract_address = migrate_contract(
            client, 
            wallet,
            entry_point_instantiate_args["ibc_transfer_contract_address"],
            ibc_transfer_adapter_contract_code_id, 
            {"entry_point_contract_address": entry_point_contract_address}, 
            "ibc_transfer_adapter"
        )
        
        # Update Admin for IBC transfer adapter contract back to real admin
        update_admin(
            client, 
            wallet, 
            ibc_transfer_adapter_contract_address, 
            "ibc_transfer_adapter"
        )
        
        # Store, migrate, and update admin for swap adapter contracts
        for i, venue in enumerate(SWAP_VENUES):
            swap_adapter_contract_code_id = store_contract(
                client, 
                wallet, 
                venue["swap_adapter_path"], 
                f"swap_adapter_{venue['name']}", 
                PERMISSIONED_UPLOADER_ADDRESS
            )
            args = {"entry_point_contract_address": entry_point_contract_address}
            if "hallswap_contract_address" in venue:
                args["hallswap_contract_address"] = venue["hallswap_contract_address"]
                
            swap_adapter_contract_address = migrate_contract(
                client, 
                wallet, 
                entry_point_instantiate_args["swap_venues"][i]["adapter_contract_address"],
                swap_adapter_contract_code_id, 
                args, 
                f"swap_adapter_{venue['name']}"
            )
            update_admin(
                client, 
                wallet, 
                swap_adapter_contract_address, 
                f"swap_adapter_{venue['name']}"
            )
    
    
def create_tx(msg,
              client, 
              wallet, 
              gas_limit: int, 
              fee: str,
              ) -> tuple[bytes, str]:
    time.sleep(5)
    tx = Transaction()
    tx.add_message(msg)
    
    # Get account
    account = client.query_account(str(wallet.address()))
    
    # Seal, Sign, and Complete Tx
    tx.seal(
        signing_cfgs=[SigningCfg.direct(wallet.public_key(), account.sequence)], 
        fee=fee, 
        gas_limit=gas_limit
    )
    tx.sign(wallet.signer(), client.network_config.chain_id, account.number)
    tx.complete()
    
    return tx

    
def create_wallet(client) -> LocalWallet:
    """ Create a wallet from a mnemonic and return it"""
    if CHAIN == "terra":
        mk = MnemonicKey(mnemonic=MNEMONIC)
        terra = LCDClient(REST_URL, CHAIN_ID)
        terra_wallet = terra.wallet(mk)
        wallet = LocalWallet(PrivateKey(terra_wallet.key.private_key), prefix="terra")
        balance = client.query_bank_balance(str(wallet.address()), DENOM)
        print("Wallet Address: ", wallet.address(), " with account balance: ", balance)
    else:
        seed_bytes = Bip39SeedGenerator(MNEMONIC).Generate()
        bip44_def_ctx = Bip44.FromSeed(seed_bytes, Bip44Coins.COSMOS).DeriveDefaultPath()
        wallet = LocalWallet(
            PrivateKey(bip44_def_ctx.PrivateKey().Raw().ToBytes()), 
            prefix=ADDRESS_PREFIX
        )  
        balance = client.query_bank_balance(str(wallet.address()), DENOM)
        print("Wallet Address: ", wallet.address(), " with account balance: ", balance)
    return wallet


def init_deployed_contracts_info():
    DEPLOYED_CONTRACTS_INFO["info"] = {}
    DEPLOYED_CONTRACTS_INFO["info"]["chain_id"] = CHAIN_ID
    DEPLOYED_CONTRACTS_INFO["info"]["network"] = NETWORK
    DEPLOYED_CONTRACTS_INFO["info"]["deploy_date"] = datetime.now().strftime("%d/%m/%Y %H:%M:%S")
    DEPLOYED_CONTRACTS_INFO["info"]["commit_hash"] = config["COMMIT_HASH"]
    DEPLOYED_CONTRACTS_INFO["info"]["salt"] = config["SALT"]
    DEPLOYED_CONTRACTS_INFO["checksums"] = {}
    DEPLOYED_CONTRACTS_INFO["code-ids"] = {}
    DEPLOYED_CONTRACTS_INFO["contract-addresses"] = {}
    DEPLOYED_CONTRACTS_INFO["tx-hashes"] = {}
    with open(f"{DEPLOYED_CONTRACTS_FOLDER_PATH}/{CHAIN}/{NETWORK}.toml", "w") as f:
        toml.dump(DEPLOYED_CONTRACTS_INFO, f)


def store_contract(
    client, 
    wallet, 
    file_path, 
    name, 
    permissioned_uploader_address
) -> int:
    if CHAIN == "osmosis":
        gas_limit = 9000000
    else:
        gas_limit = 5000000
        
    if permissioned_uploader_address is not None:
        msg_store_code = MsgStoreCode(
            sender=permissioned_uploader_address,
            wasm_byte_code=open(file_path, "rb").read(),
            instantiate_permission=None
        )
        msg = create_exec_msg(msg=msg_store_code, grantee_address=str(wallet.address()))
    else:
        msg = MsgStoreCode(
            sender=str(wallet.address()),
            wasm_byte_code=open(file_path, "rb").read(),
            instantiate_permission=None
        )
    store_tx = create_tx(
        msg=msg, 
        client=client, 
        wallet=wallet, 
        gas_limit=gas_limit,
        fee=f"{int(GAS_PRICE*gas_limit)}{DENOM}"
    )
    tx_hash = sha256(store_tx.tx.SerializeToString()).hexdigest()
    print("Tx hash: ", tx_hash)
    resp: httpx.Response = broadcast_tx(store_tx)
    contract_code_id: str = get_attribute_value(resp, "store_code", "code_id")
    print(f"Skip Swap {name} Contract Code ID:", contract_code_id)
    DEPLOYED_CONTRACTS_INFO["code-ids"][f"{name}_contract_code_id"] = contract_code_id
    DEPLOYED_CONTRACTS_INFO["tx-hashes"][f"store_{name}_tx_hash"] = tx_hash
    with open(f"{DEPLOYED_CONTRACTS_FOLDER_PATH}/{CHAIN}/{NETWORK}.toml", "w") as f:
        toml.dump(DEPLOYED_CONTRACTS_INFO, f)
    return int(contract_code_id)


def instantiate_contract(client, wallet, admin, code_id, args, label, name) -> str:
    if CHAIN == "osmosis":
        gas_limit = 600000
    else:
        gas_limit = 300000
    msg = MsgInstantiateContract(
        sender=str(wallet.address()),
        admin=admin,
        code_id=code_id,
        msg=json_encode(args).encode("UTF8"),
        label=label,
    )
    instantiate_tx = create_tx(
        msg=msg, 
        client=client, 
        wallet=wallet, 
        gas_limit=gas_limit,
        fee=f"{int(GAS_PRICE*gas_limit)}{DENOM}"
    )
    tx_hash = sha256(instantiate_tx.tx.SerializeToString()).hexdigest()
    print("Tx hash: ", tx_hash)
    resp: httpx.Response = broadcast_tx(instantiate_tx)
    contract_address: str = get_attribute_value(resp, "instantiate", "_contract_address")
    print(f"Skip Swap {name} Contract Address:", contract_address)
    DEPLOYED_CONTRACTS_INFO["contract-addresses"][f"{name}_contract_address"] = contract_address
    DEPLOYED_CONTRACTS_INFO["tx-hashes"][f"instantiate_{name}_tx_hash"] = tx_hash
    with open(f"{DEPLOYED_CONTRACTS_FOLDER_PATH}/{CHAIN}/{NETWORK}.toml", "w") as f:
        toml.dump(DEPLOYED_CONTRACTS_INFO, f)
    return contract_address

def migrate_contract(client, wallet, contract_address, code_id, args, name) -> str:
    if CHAIN == "osmosis":
        gas_limit = 600000
    else:
        gas_limit = 300000
    msg = MsgMigrateContract(
        sender=str(wallet.address()),
        contract=contract_address,
        code_id=code_id,
        msg=json_encode(args).encode("UTF8"),
    )
    migrate_tx = create_tx(
        msg=msg, 
        client=client, 
        wallet=wallet, 
        gas_limit=gas_limit,
        fee=f"{int(GAS_PRICE*gas_limit)}{DENOM}"
    )
    tx_hash = sha256(migrate_tx.tx.SerializeToString()).hexdigest()
    print("Tx hash: ", tx_hash)
    broadcast_tx(migrate_tx)
    DEPLOYED_CONTRACTS_INFO["tx-hashes"][f"migrate_{name}_tx_hash"] = tx_hash
    with open(f"{DEPLOYED_CONTRACTS_FOLDER_PATH}/{CHAIN}/{NETWORK}.toml", "w") as f:
        toml.dump(DEPLOYED_CONTRACTS_INFO, f)
    return contract_address

def update_admin(client, wallet, contract_address, name):
    if CHAIN == "osmosis":
        gas_limit = 600000
    else:
        gas_limit = 300000
    msg = MsgUpdateAdmin(
        sender=str(wallet.address()),
        new_admin=ADMIN_ADDRESS,
        contract=contract_address,
    )
    update_admin_tx = create_tx(
        msg=msg, 
        client=client, 
        wallet=wallet, 
        gas_limit=gas_limit,
        fee=f"{int(GAS_PRICE*gas_limit)}{DENOM}"
    )
    tx_hash = sha256(update_admin_tx.tx.SerializeToString()).hexdigest()
    print("Tx hash: ", tx_hash)
    broadcast_tx(update_admin_tx)
    DEPLOYED_CONTRACTS_INFO["tx-hashes"][f"update_admin_{name}_tx_hash"] = tx_hash
    with open(f"{DEPLOYED_CONTRACTS_FOLDER_PATH}/{CHAIN}/{NETWORK}.toml", "w") as f:
        toml.dump(DEPLOYED_CONTRACTS_INFO, f)
    return None



def instantiate2_contract(
    client, 
    wallet, 
    code_id, 
    args, 
    label, 
    name, 
    pre_gen_address
) -> str:
    if CHAIN == "osmosis":
        gas_limit = 600000
    else:
        gas_limit = 300000
    msg = MsgInstantiateContract2(
        sender=str(wallet.address()),
        admin=ADMIN_ADDRESS,
        code_id=code_id,
        msg=json_encode(args).encode("UTF8"),
        label=label,
        salt=SALT,
        fix_msg=False,
    )
    instantiate_2_tx = create_tx(
        msg=msg, 
        client=client, 
        wallet=wallet, 
        gas_limit=gas_limit,
        fee=f"{int(GAS_PRICE*gas_limit)}{DENOM}"
    )
    tx_hash = sha256(instantiate_2_tx.tx.SerializeToString()).hexdigest()
    print("Tx hash: ", tx_hash)
    resp: httpx.Response = broadcast_tx(instantiate_2_tx)
    contract_address: str = get_attribute_value(resp, "instantiate", "_contract_address")
    print(f"Skip Swap {name} Contract Address:", contract_address)
    DEPLOYED_CONTRACTS_INFO["contract-addresses"][f"{name}_contract_address"] = contract_address
    DEPLOYED_CONTRACTS_INFO["tx-hashes"][f"instantiate_{name}_tx_hash"] = tx_hash
    with open(f"{DEPLOYED_CONTRACTS_FOLDER_PATH}/{CHAIN}/{NETWORK}.toml", "w") as f:
        toml.dump(DEPLOYED_CONTRACTS_INFO, f)
    return contract_address


def create_any_msg(msg):
    any_msg = any_pb2.Any()
    any_msg.Pack(msg, "")
    return any_msg


def create_exec_msg(msg, grantee_address: str) -> MsgExec:
    authz_exec_any = create_any_msg(msg)
    msg_exec = MsgExec(grantee=grantee_address, msgs = [authz_exec_any])
    return msg_exec


def broadcast_tx(tx) -> httpx.Response:
    tx_bytes = tx.tx.SerializeToString()
    encoded_tx = b64encode(tx_bytes).decode("utf-8")
    data = {
        'jsonrpc': '2.0',
        'method': "broadcast_tx_sync",
        'params': [encoded_tx],
        'id': 1
    }
    postResp = httpx.post(RPC_URL, json=data, timeout=60)
    print("postResp.json(): ", postResp.json())
    print("Sleeping for 20 seconds...")
    time.sleep(20)
    resp = httpx.get(
        REST_URL + f"/cosmos/tx/v1beta1/txs/{sha256(tx_bytes).hexdigest()}", 
        timeout=60
    )
    return resp


def get_attribute_value(resp, event_type, attr_key):
    if resp.json()['tx_response']['logs'] != []:
        events = resp.json()['tx_response']['logs'][0]['events']
    else:
        events = resp.json()['tx_response']['events']
        
    for event in events:
        if event['type'] == event_type:
            for attr in event['attributes']:
                if attr['key'] == attr_key:
                    return attr['value']
    return None
    
    
if __name__ == "__main__":
    main()