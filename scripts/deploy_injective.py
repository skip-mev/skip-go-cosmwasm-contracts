import asyncio
import json
import sys
import toml
import os
import time
from datetime import datetime

from grpc import RpcError

from pyinjective.async_client import AsyncClient
from pyinjective.constant import GAS_FEE_BUFFER_AMOUNT, GAS_PRICE
from pyinjective.core.network import Network
from pyinjective.transaction import Transaction
from pyinjective.wallet import PrivateKey
from pyinjective.proto.cosmwasm.wasm.v1 import tx_pb2 as wasm_tx_pb

CHAIN = sys.argv[1]
NETWORK = sys.argv[2]
DEPLOYED_CONTRACTS_FOLDER_PATH = "../deployed-contracts"

if CHAIN != "injective":
    raise Exception("Must specify injective chain for 1st cli arg.")

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

if NETWORK == "mainnet":
    network = Network.mainnet()
    CHAIN_ID = config["MAINNET_CHAIN_ID"]
    SWAP_VENUES = config["swap_venues"]
elif NETWORK == "testnet":
    raise Exception("Testnet not supported.")
    network = Network.testnet()
    CHAIN_ID = config["TESTNET_CHAIN_ID"]
    SWAP_VENUES = config["testnet_swap_venues"]
else:
    raise Exception("Must specify either 'mainnet' or 'testnet' for 2nd cli arg.")

# SALT
SALT = config["SALT"].encode("utf-8")

# Pregenerated Contract Addresses
ENTRY_POINT_PRE_GENERATED_ADDRESS = config["ENTRY_POINT_PRE_GENERATED_ADDRESS"]

# Admin address for future migrations
ADMIN_ADDRESS = config["ADMIN_ADDRESS"]

# MNEMONIC
MNEMONIC = config["MNEMONIC"]

# Code IDs
ENTRY_POINT_CODE_ID = config["ENTRY_POINT_CODE_ID"]
IBC_TRANSFER_CODE_ID = config["IBC_TRANSFER_CODE_ID"]

DEPLOYED_CONTRACTS_INFO = {}

async def main() -> None:
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
        
    client = AsyncClient(network)
    composer = await client.composer()
    await client.sync_timeout_height()

    # load account
    priv_key = PrivateKey.from_mnemonic(MNEMONIC)
    pub_key = priv_key.to_public_key()
    address = pub_key.to_address()
    await client.fetch_account(address.to_acc_bech32())    
    
    balances = await client.fetch_bank_balances(address.to_acc_bech32())
    print("Address: ", address.to_acc_bech32(), " with account balance: ", balances)
    
    ibc_transfer_adapter_contract_address = await instantiate_contract(
        client, 
        composer, 
        priv_key, 
        pub_key, 
        address,
        code_id=IBC_TRANSFER_CODE_ID,
        args={"entry_point_contract_address": ENTRY_POINT_PRE_GENERATED_ADDRESS},
        label="Skip Swap IBC Transfer Adapter",
        name="ibc_transfer_adapter"
    )  
    
    entry_point_instantiate_args = {
        "swap_venues": [],
        "ibc_transfer_contract_address": ibc_transfer_adapter_contract_address,
    }
    
    for venue in SWAP_VENUES:
        args = {"entry_point_contract_address": ENTRY_POINT_PRE_GENERATED_ADDRESS}
        if "hallswap_contract_address" in venue:
            args["hallswap_contract_address"] = venue["hallswap_contract_address"]
        swap_adapter_contract_address = await instantiate_contract(
            client, 
            composer, 
            priv_key, 
            pub_key, 
            address,
            code_id=venue["code_id"],
            args=args,
            label=f"Skip Swap Swap Adapter {venue['name']}",
            name=f"swap_adapter_{venue['name']}"
        )        
        entry_point_instantiate_args["swap_venues"].append(
            {
                "name": venue["name"],
                "adapter_contract_address": swap_adapter_contract_address,
            }
        )
    
    await instantiate2_contract(
        client, 
        composer, 
        priv_key, 
        pub_key, 
        address,
        code_id=ENTRY_POINT_CODE_ID,
        args=entry_point_instantiate_args,
        label="Skip Swap Entry Point",
        name="entry_point"
    )
    
async def instantiate_contract(
    client, 
    composer, 
    priv_key, 
    pub_key, 
    address,
    code_id,
    args,
    label,
    name,
):
    msg = wasm_tx_pb.MsgInstantiateContract(
        sender=address.to_acc_bech32(),
        admin=ADMIN_ADDRESS,
        code_id=code_id,
        label=label,
        msg=json.dumps(args).encode('utf-8'),
    )
    return await broadcast_tx(
        msg,
        client, 
        composer, 
        priv_key, 
        pub_key, 
        code_id,
        name,
    )
    
    
async def instantiate2_contract(
    client, 
    composer, 
    priv_key, 
    pub_key, 
    address,
    code_id,
    args,
    label,
    name,
):
    msg = wasm_tx_pb.MsgInstantiateContract2(
        sender=address.to_acc_bech32(),
        admin=ADMIN_ADDRESS,
        code_id=code_id,
        label=label,
        msg=json.dumps(args).encode('utf-8'),
        salt=SALT,
        fix_msg=False,
    )
    return await broadcast_tx(
        msg,
        client, 
        composer, 
        priv_key, 
        pub_key, 
        code_id,
        name,
    )
    
    
async def broadcast_tx(
    msg,
    client, 
    composer, 
    priv_key, 
    pub_key, 
    code_id,
    name,
):
    # build sim tx
    tx = (
        Transaction()
        .with_messages(msg)
        .with_sequence(client.get_sequence())
        .with_account_num(client.get_number())
        .with_chain_id(network.chain_id)
    )
    sim_sign_doc = tx.get_sign_doc(pub_key)
    sim_sig = priv_key.sign(sim_sign_doc.SerializeToString())
    sim_tx_raw_bytes = tx.get_tx_data(sim_sig, pub_key)

    # simulate tx
    try:
        sim_res = await client.simulate(sim_tx_raw_bytes)
    except RpcError as ex:
        print(ex)
        return
    
    print("sim_res: ", sim_res)
    
    # build tx
    gas_price = GAS_PRICE
    gas_limit = int(sim_res["gasInfo"]["gasUsed"]) + GAS_FEE_BUFFER_AMOUNT  # add buffer for gas fee computation
    gas_fee = "{:.18f}".format((gas_price * gas_limit) / pow(10, 18)).rstrip("0")
    fee = [
        composer.Coin(
            amount=gas_price * gas_limit,
            denom=network.fee_denom,
        )
    ]
    tx = tx.with_gas(gas_limit).with_fee(fee).with_memo("").with_timeout_height(client.timeout_height)
    sign_doc = tx.get_sign_doc(pub_key)
    sig = priv_key.sign(sign_doc.SerializeToString())
    tx_raw_bytes = tx.get_tx_data(sig, pub_key)
    
    print("tx: ", tx)
    print("gas price: ", gas_price)
    print("gas limit: ", gas_limit)
    print("gas fee: ", gas_fee)
    print("fee: ", fee)
    print("Broadcasting tx...")

    # broadcast tx: send_tx_async_mode, send_tx_sync_mode, send_tx_block_mode
    res = await client.broadcast_tx_sync_mode(tx_raw_bytes)
    print(res)
    print("gas wanted: {}".format(gas_limit))
    print("gas fee: {} INJ".format(gas_fee))
    
    print("Sleeping for 10 seconds...")
    time.sleep(10)
    
    tx_hash = res['txResponse']['txhash']
    print("tx hash: ", tx_hash)
    tx_logs = await client.fetch_tx(hash=tx_hash)
    print(tx_logs)
    contract_address = get_contract_address(tx_logs)
    
    print(f"Skip Swap {name} Contract Address:", contract_address)
    DEPLOYED_CONTRACTS_INFO["code-ids"][f"{name}_contract_code_id"] = code_id
    DEPLOYED_CONTRACTS_INFO["contract-addresses"][f"{name}_contract_address"] = contract_address
    DEPLOYED_CONTRACTS_INFO["tx-hashes"][f"instantiate_{name}_tx_hash"] = tx_hash
    with open(f"{DEPLOYED_CONTRACTS_FOLDER_PATH}/{CHAIN}/{NETWORK}.toml", "w") as f:
        toml.dump(DEPLOYED_CONTRACTS_INFO, f)
        
    return contract_address

    
def get_contract_address(tx_logs):
    for event in tx_logs['txResponse']['logs'][0]['events']:
        if event['type'] == "instantiate":
            for attr in event['attributes']:
                if attr['key'] == "_contract_address":
                    return attr['value']
    return None


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
        

if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())