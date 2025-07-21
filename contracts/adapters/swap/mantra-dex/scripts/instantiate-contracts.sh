#!/usr/bin/env bash
set -e

# Configuration
CHAIN_ID="mantra-dukong-1"
NODE_URL="https://rpc.dukong.mantrachain.io:443"
WALLET_NAME="admin"

# Mantra DEX contract addresses on mantra-dukong network
POOL_MANAGER_ADDRESS="mantra1vwj600jud78djej7ttq44dktu4wr3t2yrrsjgmld8v3jq8mud68q5w7455"

# Get script directory
SCRIPT_DIR=$(realpath "$0" | sed 's|\(.*\)/.*|\1|')

# Read stored code IDs
ENTRY_POINT_CODE_ID=$(cat "$SCRIPT_DIR/entry_point_code_id.txt" | tr -d ' \t\n\r')
IBC_HOOKS_CODE_ID=$(cat "$SCRIPT_DIR/ibc_hooks_code_id.txt" | tr -d ' \t\n\r')
MANTRA_DEX_CODE_ID=$(cat "$SCRIPT_DIR/mantra_dex_code_id.txt" | tr -d ' \t\n\r')

echo "Using stored Code IDs:"
echo "- Entry Point: $ENTRY_POINT_CODE_ID"
echo "- IBC Hooks Adapter: $IBC_HOOKS_CODE_ID"
echo "- Mantra DEX Adapter: $MANTRA_DEX_CODE_ID"
echo ""

# Instantiate contract function
instantiate_contract() {
    local code_id="$1"
    local init_msg="$2"
    local label="$3"
    local contract_name="$4"
    
    echo "Instantiating $contract_name with Code ID: $code_id"
    echo "Message: $init_msg"
    
    INSTANTIATE_RESULT=$(mantrachaind tx wasm instantiate "$code_id" "$init_msg" \
        --from "$WALLET_NAME" \
        --chain-id "$CHAIN_ID" \
        --node "$NODE_URL" \
        --label "$label" \
        --no-admin \
        --gas 300000 \
        --gas-prices "0.01uom" \
        --broadcast-mode sync \
        --yes \
        --output json)
    
    echo "Instantiate transaction result for $contract_name:"
    echo "$INSTANTIATE_RESULT"
    
    # Get transaction hash
    TXHASH=$(echo "$INSTANTIATE_RESULT" | jq -r '.txhash' 2>/dev/null || echo "")
    
    if [ "$TXHASH" == "null" ] || [ -z "$TXHASH" ]; then
        echo "Error: Failed to get transaction hash for $contract_name"
        echo "Raw result: $INSTANTIATE_RESULT"
        return 1
    fi
    
    echo "Transaction submitted with hash: $TXHASH"
    echo "Waiting for transaction to be processed..."
    sleep 10
    
    # Query the transaction result
    TX_RESULT=$(mantrachaind query tx "$TXHASH" --node "$NODE_URL" --output json 2>/dev/null || echo "null")
    
    if [ "$TX_RESULT" == "null" ]; then
        echo "Transaction not yet processed. Please check manually with:"
        echo "mantrachaind query tx $TXHASH --node $NODE_URL"
        return 1
    fi
    
    # Extract contract address from transaction result
    CONTRACT_ADDR=$(echo "$TX_RESULT" | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value')
    
    if [ "$CONTRACT_ADDR" != "null" ] && [ -n "$CONTRACT_ADDR" ]; then
        echo "$contract_name contract instantiated successfully!"
        echo "Contract Address: $CONTRACT_ADDR"
        echo "$CONTRACT_ADDR"
        return 0
    else
        echo "Error: Failed to extract $contract_name contract address"
        return 1
    fi
}

echo "Starting contract instantiation..."
echo ""

# Step 1: Instantiate IBC Hooks Adapter with placeholder entry point
echo "=== STEP 1: Instantiating IBC Hooks Adapter ==="
TEMP_ENTRY_POINT="$POOL_MANAGER_ADDRESS"
IBC_HOOKS_INIT='{"entry_point_contract_address":"'$TEMP_ENTRY_POINT'"}'

IBC_HOOKS_ADDRESS=$(instantiate_contract "$IBC_HOOKS_CODE_ID" "$IBC_HOOKS_INIT" "skip-go-ibc-hooks-adapter" "IBC Hooks Adapter")
if [ $? -ne 0 ]; then
    echo "Failed to instantiate IBC Hooks Adapter contract"
    exit 1
fi
echo "$IBC_HOOKS_ADDRESS" > "$SCRIPT_DIR/ibc_hooks_address.txt"
echo ""

# Step 2: Instantiate Mantra DEX Adapter with placeholder entry point
echo "=== STEP 2: Instantiating Mantra DEX Adapter ==="
MANTRA_DEX_INIT='{"entry_point_contract_address":"'$TEMP_ENTRY_POINT'","mantra_pool_manager_address":"'$POOL_MANAGER_ADDRESS'"}'

MANTRA_DEX_ADDRESS=$(instantiate_contract "$MANTRA_DEX_CODE_ID" "$MANTRA_DEX_INIT" "skip-go-swap-adapter-mantra-dex" "Mantra DEX Adapter")
if [ $? -ne 0 ]; then
    echo "Failed to instantiate Mantra DEX Adapter contract"
    exit 1
fi
echo "$MANTRA_DEX_ADDRESS" > "$SCRIPT_DIR/mantra_dex_address.txt"
echo ""

# Step 3: Instantiate Entry Point with real adapter addresses
echo "=== STEP 3: Instantiating Entry Point ==="
ENTRY_POINT_INIT='{"swap_venues":[{"name":"mantra-dex","adapter_contract_address":"'$MANTRA_DEX_ADDRESS'"}],"ibc_transfer_contract_address":"'$IBC_HOOKS_ADDRESS'"}'

ENTRY_POINT_ADDRESS=$(instantiate_contract "$ENTRY_POINT_CODE_ID" "$ENTRY_POINT_INIT" "skip-go-entry-point" "Entry Point")
if [ $? -ne 0 ]; then
    echo "Failed to instantiate Entry Point contract"
    exit 1
fi
echo "$ENTRY_POINT_ADDRESS" > "$SCRIPT_DIR/entry_point_address.txt"
echo ""

# Summary
echo "=========================================="
echo "🎉 INSTANTIATION COMPLETED SUCCESSFULLY! 🎉"
echo "=========================================="
echo ""
echo "Entry Point Contract:"
echo "  Code ID: $ENTRY_POINT_CODE_ID"
echo "  Address: $ENTRY_POINT_ADDRESS"
echo ""
echo "IBC Hooks Adapter Contract:"
echo "  Code ID: $IBC_HOOKS_CODE_ID"
echo "  Address: $IBC_HOOKS_ADDRESS"
echo ""
echo "Mantra DEX Adapter Contract:"
echo "  Code ID: $MANTRA_DEX_CODE_ID"
echo "  Address: $MANTRA_DEX_ADDRESS"
echo ""
echo "Configuration:"
echo "  Pool Manager: $POOL_MANAGER_ADDRESS"
echo ""
echo "Files created:"
echo "  - entry_point_address.txt"
echo "  - ibc_hooks_address.txt"
echo "  - mantra_dex_address.txt"
echo ""
echo "NOTE: The adapters were initially deployed with a placeholder entry point"
echo "address. You may need to migrate them to use the real entry point address:"
echo "  Entry Point: $ENTRY_POINT_ADDRESS"