#!/usr/bin/env bash
set -e

# Configuration
CHAIN_ID="mantra-dukong-1"
NODE_URL="https://rpc.dukong.mantrachain.io:443"
WALLET_NAME="admin"

# Mantra DEX contract addresses on mantra-dukong network
POOL_MANAGER_ADDRESS="mantra1vwj600jud78djej7ttq44dktu4wr3t2yrrsjgmld8v3jq8mud68q5w7455"

# Get script directory and workspace root
SCRIPT_DIR=$(realpath "$0" | sed 's|\(.*\)/.*|\1|')
WORKSPACE_ROOT=$(realpath "$SCRIPT_DIR/../../../../../")
ARTIFACTS_DIR="$WORKSPACE_ROOT/artifacts"

# Function to display usage
display_usage() {
    echo "Skip Go Complete Deployment Script for Mantra-Dukong"
    echo ""
    echo "Usage: ./deploy-all.sh [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -h, --help           Show this help message"
    echo ""
    echo "This script will deploy all three contracts:"
    echo "  1. Entry Point Contract"
    echo "  2. IBC Hooks Adapter Contract"  
    echo "  3. Mantra DEX Adapter Contract"
    echo ""
    echo "The IBC Hooks adapter will be automatically deployed and registered with the Entry Point."
}

# Check if artifacts exist
check_artifacts() {
    if [ ! -d "$ARTIFACTS_DIR" ]; then
        echo "Error: Artifacts directory not found at $ARTIFACTS_DIR"
        echo "Please run ./build-all.sh first to build the contracts"
        exit 1
    fi
    
    ENTRY_POINT_WASM=$(find "$ARTIFACTS_DIR" -name "*entry_point*.wasm" | head -n 1)
    IBC_HOOKS_WASM=$(find "$ARTIFACTS_DIR" -name "*ibc_adapter_ibc_hooks*.wasm" | head -n 1)
    MANTRA_DEX_WASM=$(find "$ARTIFACTS_DIR" -name "*swap_adapter_mantra_dex*.wasm" | head -n 1)
    
    if [ -z "$ENTRY_POINT_WASM" ]; then
        echo "Error: Entry point WASM file not found in $ARTIFACTS_DIR"
        echo "Please run ./build-all.sh first to build the contracts"
        exit 1
    fi
    
    if [ -z "$IBC_HOOKS_WASM" ]; then
        echo "Error: IBC Hooks WASM file not found in $ARTIFACTS_DIR"
        echo "Please run ./build-all.sh first to build the contracts"
        exit 1
    fi
    
    if [ -z "$MANTRA_DEX_WASM" ]; then
        echo "Error: Mantra DEX WASM file not found in $ARTIFACTS_DIR"
        echo "Please run ./build-all.sh first to build the contracts"
        exit 1
    fi
    
    echo "Found contract artifacts:"
    echo "- Entry Point: $ENTRY_POINT_WASM"
    echo "- IBC Hooks Adapter: $IBC_HOOKS_WASM"
    echo "- Mantra DEX Adapter: $MANTRA_DEX_WASM"
}

# Store contract code
store_contract() {
    local wasm_file="$1"
    local contract_name="$2"
    
    echo "Storing $contract_name contract code on $CHAIN_ID..." >&2
    
    STORE_RESULT=$(mantrachaind tx wasm store "$wasm_file" \
        --from "$WALLET_NAME" \
        --chain-id "$CHAIN_ID" \
        --node "$NODE_URL" \
        --gas auto \
        --gas-adjustment 1.3 \
        --gas-prices "0.01uom" \
        --broadcast-mode sync \
        --yes \
        --output json 2>&1)
    
    # Get transaction hash
    TXHASH=$(echo "$STORE_RESULT" | jq -r '.txhash // empty' 2>/dev/null)
    
    if [ -z "$TXHASH" ]; then
        echo "Error: Failed to get transaction hash for $contract_name" >&2
        echo "Store result: $STORE_RESULT" >&2
        return 1
    fi
    
    echo "Transaction submitted with hash: $TXHASH" >&2
    echo "Waiting for transaction to be processed..." >&2
    
    # Wait and retry until transaction is processed
    local retries=0
    local max_retries=6
    while [ $retries -lt $max_retries ]; do
        sleep 10
        TX_RESULT=$(mantrachaind query tx "$TXHASH" --node "$NODE_URL" --output json 2>/dev/null)
        
        if [ $? -eq 0 ] && [ -n "$TX_RESULT" ] && [ "$TX_RESULT" != "null" ]; then
            break
        fi
        
        retries=$((retries + 1))
        echo "Retry $retries/$max_retries: Transaction not yet processed..." >&2
    done
    
    if [ $retries -eq $max_retries ]; then
        echo "Error: Transaction not processed after $max_retries attempts" >&2
        echo "Check manually: mantrachaind query tx $TXHASH --node $NODE_URL" >&2
        return 1
    fi
    
    # Extract code ID with robust parsing
    local CODE_ID=""
    
    # Try multiple parsing methods
    # Method 1: events at root level
    CODE_ID=$(echo "$TX_RESULT" | jq -r '.events[]? | select(.type=="store_code") | .attributes[]? | select(.key=="code_id") | .value' 2>/dev/null | head -n1)
    
    # Method 2: events in logs
    if [ -z "$CODE_ID" ] || [ "$CODE_ID" == "null" ]; then
        CODE_ID=$(echo "$TX_RESULT" | jq -r '.logs[]?.events[]? | select(.type=="store_code") | .attributes[]? | select(.key=="code_id") | .value' 2>/dev/null | head -n1)
    fi
    
    # Method 3: look for any code_id in the response
    if [ -z "$CODE_ID" ] || [ "$CODE_ID" == "null" ]; then
        CODE_ID=$(echo "$TX_RESULT" | jq -r '.. | objects | select(has("key") and has("value") and .key=="code_id") | .value' 2>/dev/null | head -n1)
    fi
    
    if [ -n "$CODE_ID" ] && [ "$CODE_ID" != "null" ]; then
        echo "$contract_name stored successfully with Code ID: $CODE_ID" >&2
        echo "$CODE_ID"
        return 0
    else
        echo "Error: Failed to extract Code ID for $contract_name" >&2
        echo "Transaction result:" >&2
        echo "$TX_RESULT" | jq '.' >&2
        return 1
    fi
}

# Generic contract instantiation function
instantiate_contract() {
    local code_id="$1"
    local init_msg="$2"
    local label="$3"
    local contract_name="$4"
    
    echo "Instantiating $contract_name with Code ID: $code_id" >&2
    echo "Instantiation message: $init_msg" >&2
    
    INSTANTIATE_RESULT=$(mantrachaind tx wasm instantiate "$code_id" "$init_msg" \
        --from "$WALLET_NAME" \
        --chain-id "$CHAIN_ID" \
        --node "$NODE_URL" \
        --label "$label" \
        --admin "mantra10tysdwkjuqecgg9npery40dvc8ak9urhf6dj6u" \
        --gas auto \
        --gas-adjustment 1.3 \
        --gas-prices "0.01uom" \
        --broadcast-mode sync \
        --yes \
        --output json 2>&1)
    
    # Get transaction hash
    TXHASH=$(echo "$INSTANTIATE_RESULT" | jq -r '.txhash // empty' 2>/dev/null)
    
    if [ -z "$TXHASH" ]; then
        echo "Error: Failed to get transaction hash for $contract_name" >&2
        echo "Instantiate result: $INSTANTIATE_RESULT" >&2
        return 1
    fi
    
    echo "Transaction submitted with hash: $TXHASH" >&2
    echo "Waiting for transaction to be processed..." >&2
    
    # Wait and retry until transaction is processed
    local retries=0
    local max_retries=6
    while [ $retries -lt $max_retries ]; do
        sleep 10
        TX_RESULT=$(mantrachaind query tx "$TXHASH" --node "$NODE_URL" --output json 2>/dev/null)
        
        if [ $? -eq 0 ] && [ -n "$TX_RESULT" ] && [ "$TX_RESULT" != "null" ]; then
            break
        fi
        
        retries=$((retries + 1))
        echo "Retry $retries/$max_retries: Transaction not yet processed..." >&2
    done
    
    if [ $retries -eq $max_retries ]; then
        echo "Error: Transaction not processed after $max_retries attempts" >&2
        echo "Check manually: mantrachaind query tx $TXHASH --node $NODE_URL" >&2
        return 1
    fi
    
    # Extract contract address with robust parsing
    local CONTRACT_ADDR=""
    
    # Try multiple parsing methods
    # Method 1: events at root level
    CONTRACT_ADDR=$(echo "$TX_RESULT" | jq -r '.events[]? | select(.type=="instantiate") | .attributes[]? | select(.key=="_contract_address") | .value' 2>/dev/null | head -n1)
    
    # Method 2: events in logs
    if [ -z "$CONTRACT_ADDR" ] || [ "$CONTRACT_ADDR" == "null" ]; then
        CONTRACT_ADDR=$(echo "$TX_RESULT" | jq -r '.logs[]?.events[]? | select(.type=="instantiate") | .attributes[]? | select(.key=="_contract_address") | .value' 2>/dev/null | head -n1)
    fi
    
    # Method 3: look for any _contract_address in the response
    if [ -z "$CONTRACT_ADDR" ] || [ "$CONTRACT_ADDR" == "null" ]; then
        CONTRACT_ADDR=$(echo "$TX_RESULT" | jq -r '.. | objects | select(has("key") and has("value") and .key=="_contract_address") | .value' 2>/dev/null | head -n1)
    fi
    
    if [ -n "$CONTRACT_ADDR" ] && [ "$CONTRACT_ADDR" != "null" ]; then
        echo "$contract_name instantiated successfully at: $CONTRACT_ADDR" >&2
        echo "$CONTRACT_ADDR"
        return 0
    else
        echo "Error: Failed to extract contract address for $contract_name" >&2
        echo "Transaction result:" >&2
        echo "$TX_RESULT" | jq '.' >&2
        return 1
    fi
}

# Instantiate IBC hooks adapter contract
instantiate_ibc_hooks() {
    local code_id="$1"
    local entry_point_address="$2"
    
    echo "Instantiating IBC Hooks Adapter contract with Code ID: $code_id"
    
    # Create instantiation message
    INIT_MSG=$(cat <<EOF
{
  "entry_point_contract_address": "$entry_point_address"
}
EOF
)
    
    echo "IBC Hooks Adapter instantiation message:"
    echo "$INIT_MSG"
    
    INSTANTIATE_RESULT=$(mantrachaind tx wasm instantiate "$code_id" "$INIT_MSG" \
        --from "$WALLET_NAME" \
        --chain-id "$CHAIN_ID" \
        --node "$NODE_URL" \
        --label "skip-go-ibc-hooks-adapter" \
        --admin "mantra10tysdwkjuqecgg9npery40dvc8ak9urhf6dj6u" \
        --gas auto \
        --gas-adjustment 1.3 \
        --gas-prices "0.01uom" \
        --broadcast-mode sync \
        --yes \
        --output json)
    
    echo "IBC Hooks Adapter instantiate transaction result:"
    echo "$INSTANTIATE_RESULT"
    
    # Get transaction hash
    TXHASH=$(echo "$INSTANTIATE_RESULT" | jq -r '.txhash')
    
    if [ "$TXHASH" == "null" ] || [ -z "$TXHASH" ]; then
        echo "Error: Failed to get transaction hash for IBC Hooks Adapter"
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
    CONTRACT_ADDR=$(echo "$TX_RESULT" | jq -r '.events[]? | select(.type=="instantiate")? | .attributes[]? | select(.key=="_contract_address")? | .value' 2>/dev/null || echo "")
    
    # Fallback: try to extract from logs if events method fails
    if [ -z "$CONTRACT_ADDR" ] || [ "$CONTRACT_ADDR" == "null" ]; then
        CONTRACT_ADDR=$(echo "$TX_RESULT" | jq -r '.logs[]?.events[]? | select(.type=="instantiate")? | .attributes[]? | select(.key=="_contract_address")? | .value' 2>/dev/null || echo "")
    fi
    
    if [ "$CONTRACT_ADDR" != "null" ] && [ -n "$CONTRACT_ADDR" ]; then
        echo "IBC Hooks Adapter contract instantiated successfully!"
        echo "Contract Address: $CONTRACT_ADDR"
        echo "$CONTRACT_ADDR"
        return 0
    else
        echo "Error: Failed to extract IBC Hooks Adapter contract address"
        return 1
    fi
}

# Instantiate entry point contract with initial empty configuration
instantiate_entry_point_initial() {
    local code_id="$1"
    
    echo "Instantiating Entry Point contract with Code ID: $code_id"
    
    # Create instantiation message for entry point with empty swap venues and placeholder IBC
    INIT_MSG=$(cat <<EOF
{
  "swap_venues": [],
  "ibc_transfer_contract_address": "mantra10tysdwkjuqecgg9npery40dvc8ak9urhf6dj6u",
  "hyperlane_transfer_contract_address": null
}
EOF
)
    
    echo "Entry Point instantiation message:"
    echo "$INIT_MSG"
    
    INSTANTIATE_RESULT=$(mantrachaind tx wasm instantiate "$code_id" "$INIT_MSG" \
        --from "$WALLET_NAME" \
        --chain-id "$CHAIN_ID" \
        --node "$NODE_URL" \
        --label "skip-go-entry-point" \
        --admin "mantra10tysdwkjuqecgg9npery40dvc8ak9urhf6dj6u" \
        --gas auto \
        --gas-adjustment 1.3 \
        --gas-prices "0.01uom" \
        --broadcast-mode sync \
        --yes \
        --output json)
    
    echo "Entry Point instantiate transaction result:"
    echo "$INSTANTIATE_RESULT"
    
    # Get transaction hash
    TXHASH=$(echo "$INSTANTIATE_RESULT" | jq -r '.txhash')
    
    if [ "$TXHASH" == "null" ] || [ -z "$TXHASH" ]; then
        echo "Error: Failed to get transaction hash for Entry Point"
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
    CONTRACT_ADDR=$(echo "$TX_RESULT" | jq -r '.events[]? | select(.type=="instantiate")? | .attributes[]? | select(.key=="_contract_address")? | .value' 2>/dev/null || echo "")
    
    # Fallback: try to extract from logs if events method fails
    if [ -z "$CONTRACT_ADDR" ] || [ "$CONTRACT_ADDR" == "null" ]; then
        CONTRACT_ADDR=$(echo "$TX_RESULT" | jq -r '.logs[]?.events[]? | select(.type=="instantiate")? | .attributes[]? | select(.key=="_contract_address")? | .value' 2>/dev/null || echo "")
    fi
    
    if [ "$CONTRACT_ADDR" != "null" ] && [ -n "$CONTRACT_ADDR" ]; then
        echo "Entry Point contract instantiated successfully!"
        echo "Contract Address: $CONTRACT_ADDR"
        echo "$CONTRACT_ADDR"
        return 0
    else
        echo "Error: Failed to extract Entry Point contract address"
        return 1
    fi
}

# Instantiate entry point contract
instantiate_entry_point() {
    local code_id="$1"
    local ibc_transfer_address="$2"
    local mantra_dex_address="$3"
    
    echo "Instantiating Entry Point contract with Code ID: $code_id"
    
    # Create instantiation message for entry point with swap venues
    INIT_MSG=$(cat <<EOF
{
  "swap_venues": [
    {
      "name": "mantra-dex",
      "adapter_contract_address": "$mantra_dex_address"
    }
  ],
  "ibc_transfer_contract_address": "$ibc_transfer_address"
}
EOF
)
    
    echo "Entry Point instantiation message:"
    echo "$INIT_MSG"
    
    INSTANTIATE_RESULT=$(mantrachaind tx wasm instantiate "$code_id" "$INIT_MSG" \
        --from "$WALLET_NAME" \
        --chain-id "$CHAIN_ID" \
        --node "$NODE_URL" \
        --label "skip-go-entry-point" \
        --admin "mantra10tysdwkjuqecgg9npery40dvc8ak9urhf6dj6u" \
        --gas auto \
        --gas-adjustment 1.3 \
        --gas-prices "0.01uom" \
        --broadcast-mode sync \
        --yes \
        --output json)
    
    echo "Entry Point instantiate transaction result:"
    echo "$INSTANTIATE_RESULT"
    
    # Get transaction hash
    TXHASH=$(echo "$INSTANTIATE_RESULT" | jq -r '.txhash')
    
    if [ "$TXHASH" == "null" ] || [ -z "$TXHASH" ]; then
        echo "Error: Failed to get transaction hash for Entry Point"
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
    CONTRACT_ADDR=$(echo "$TX_RESULT" | jq -r '.events[]? | select(.type=="instantiate")? | .attributes[]? | select(.key=="_contract_address")? | .value' 2>/dev/null || echo "")
    
    # Fallback: try to extract from logs if events method fails
    if [ -z "$CONTRACT_ADDR" ] || [ "$CONTRACT_ADDR" == "null" ]; then
        CONTRACT_ADDR=$(echo "$TX_RESULT" | jq -r '.logs[]?.events[]? | select(.type=="instantiate")? | .attributes[]? | select(.key=="_contract_address")? | .value' 2>/dev/null || echo "")
    fi
    
    if [ "$CONTRACT_ADDR" != "null" ] && [ -n "$CONTRACT_ADDR" ]; then
        echo "Entry Point contract instantiated successfully!"
        echo "Contract Address: $CONTRACT_ADDR"
        echo "$CONTRACT_ADDR"
        return 0
    else
        echo "Error: Failed to extract Entry Point contract address"
        return 1
    fi
}

# Instantiate mantra dex adapter contract
instantiate_mantra_adapter() {
    local code_id="$1"
    local entry_point_address="$2"
    
    echo "Instantiating Mantra DEX Adapter contract with Code ID: $code_id"
    echo "Entry Point Address: $entry_point_address"
    echo "Pool Manager Address: $POOL_MANAGER_ADDRESS"
    
    # Create instantiation message
    INIT_MSG=$(cat <<EOF
{
  "entry_point_contract_address": "$entry_point_address",
  "mantra_pool_manager_address": "$POOL_MANAGER_ADDRESS"
}
EOF
)
    
    echo "Mantra DEX Adapter instantiation message:"
    echo "$INIT_MSG"
    
    INSTANTIATE_RESULT=$(mantrachaind tx wasm instantiate "$code_id" "$INIT_MSG" \
        --from "$WALLET_NAME" \
        --chain-id "$CHAIN_ID" \
        --node "$NODE_URL" \
        --label "skip-go-swap-adapter-mantra-dex" \
        --admin "mantra10tysdwkjuqecgg9npery40dvc8ak9urhf6dj6u" \
        --gas auto \
        --gas-adjustment 1.3 \
        --gas-prices "0.01uom" \
        --broadcast-mode sync \
        --yes \
        --output json)
    
    echo "Mantra DEX Adapter instantiate transaction result:"
    echo "$INSTANTIATE_RESULT"
    
    # Get transaction hash
    TXHASH=$(echo "$INSTANTIATE_RESULT" | jq -r '.txhash')
    
    if [ "$TXHASH" == "null" ] || [ -z "$TXHASH" ]; then
        echo "Error: Failed to get transaction hash for Mantra DEX Adapter"
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
    CONTRACT_ADDR=$(echo "$TX_RESULT" | jq -r '.events[]? | select(.type=="instantiate")? | .attributes[]? | select(.key=="_contract_address")? | .value' 2>/dev/null || echo "")
    
    # Fallback: try to extract from logs if events method fails
    if [ -z "$CONTRACT_ADDR" ] || [ "$CONTRACT_ADDR" == "null" ]; then
        CONTRACT_ADDR=$(echo "$TX_RESULT" | jq -r '.logs[]?.events[]? | select(.type=="instantiate")? | .attributes[]? | select(.key=="_contract_address")? | .value' 2>/dev/null || echo "")
    fi
    
    if [ "$CONTRACT_ADDR" != "null" ] && [ -n "$CONTRACT_ADDR" ]; then
        echo "Mantra DEX Adapter contract instantiated successfully!"
        echo "Contract Address: $CONTRACT_ADDR"
        echo "$CONTRACT_ADDR"
        return 0
    else
        echo "Error: Failed to extract Mantra DEX Adapter contract address"
        return 1
    fi
}

# Main deployment function
deploy_all() {
    echo "Starting complete deployment process..."
    echo ""
    
    # Step 1: Store all contracts
    echo "=== STEP 1: Storing Entry Point Contract ==="
    ENTRY_POINT_CODE_ID=$(store_contract "$ENTRY_POINT_WASM" "Entry Point")
    if [ $? -ne 0 ]; then
        echo "Failed to store Entry Point contract"
        exit 1
    fi
    echo "$ENTRY_POINT_CODE_ID" > "$SCRIPT_DIR/entry_point_code_id.txt"
    echo ""
    
    echo "=== STEP 2: Storing IBC Hooks Adapter Contract ==="
    IBC_HOOKS_CODE_ID=$(store_contract "$IBC_HOOKS_WASM" "IBC Hooks Adapter")
    if [ $? -ne 0 ]; then
        echo "Failed to store IBC Hooks Adapter contract"
        exit 1
    fi
    echo "$IBC_HOOKS_CODE_ID" > "$SCRIPT_DIR/ibc_hooks_code_id.txt"
    echo ""
    
    echo "=== STEP 3: Storing Mantra DEX Adapter Contract ==="
    MANTRA_DEX_CODE_ID=$(store_contract "$MANTRA_DEX_WASM" "Mantra DEX Adapter")
    if [ $? -ne 0 ]; then
        echo "Failed to store Mantra DEX Adapter contract"
        exit 1
    fi
    echo "$MANTRA_DEX_CODE_ID" > "$SCRIPT_DIR/mantra_dex_code_id.txt"
    echo ""
    
    # Step 4: Instantiate Entry Point first with empty swap venues
    echo "=== STEP 4: Instantiating Entry Point Contract with empty configuration ==="
    ENTRY_POINT_ADDRESS=$(instantiate_entry_point_initial "$ENTRY_POINT_CODE_ID")
    if [ $? -ne 0 ]; then
        echo "Failed to instantiate Entry Point contract"
        exit 1
    fi
    echo "$ENTRY_POINT_ADDRESS" > "$SCRIPT_DIR/entry_point_address.txt"
    echo ""
    
    echo "=== STEP 5: Instantiating IBC Hooks Adapter Contract ==="
    IBC_HOOKS_ADDRESS=$(instantiate_ibc_hooks "$IBC_HOOKS_CODE_ID" "$ENTRY_POINT_ADDRESS")
    if [ $? -ne 0 ]; then
        echo "Failed to instantiate IBC Hooks Adapter contract"
        exit 1
    fi
    echo "$IBC_HOOKS_ADDRESS" > "$SCRIPT_DIR/ibc_hooks_address.txt"
    echo ""
    
    echo "=== STEP 6: Instantiating Mantra DEX Adapter Contract ==="
    MANTRA_DEX_ADDRESS=$(instantiate_mantra_adapter "$MANTRA_DEX_CODE_ID" "$ENTRY_POINT_ADDRESS")
    if [ $? -ne 0 ]; then
        echo "Failed to instantiate Mantra DEX Adapter contract"
        exit 1
    fi
    echo "$MANTRA_DEX_ADDRESS" > "$SCRIPT_DIR/mantra_dex_address.txt"
    echo ""
    
    # Summary
    echo "=========================================="
    echo "🎉 DEPLOYMENT COMPLETED SUCCESSFULLY! 🎉"
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
    echo "  - entry_point_code_id.txt / entry_point_address.txt"
    echo "  - ibc_hooks_code_id.txt / ibc_hooks_address.txt"  
    echo "  - mantra_dex_code_id.txt / mantra_dex_address.txt"
    echo ""
    echo "NEXT STEPS:"
    echo "You need to configure the Entry Point contract to register the adapters:"
    echo "  1. Add IBC adapter: $IBC_HOOKS_ADDRESS"
    echo "  2. Add Mantra DEX adapter: $MANTRA_DEX_ADDRESS"
    echo ""
    echo "The Entry Point is currently configured with:"
    echo "  - Empty swap venues (adapters need to be registered)"
    echo "  - Placeholder IBC transfer adapter (needs to be updated)"
}

# Main script logic
main() {
    case "${1:-}" in
        -h|--help)
            display_usage
            exit 0
            ;;
        "")
            check_artifacts
            deploy_all
            ;;
        *)
            echo "Error: Unknown option $1"
            display_usage
            exit 1
            ;;
    esac
}

# Check if mantrachaind is available
if ! command -v mantrachaind &> /dev/null; then
    echo "Error: mantrachaind command not found"
    echo "Please ensure Mantra Chain CLI is installed and in your PATH"
    exit 1
fi

# Check if jq is available for JSON parsing
if ! command -v jq &> /dev/null; then
    echo "Error: jq command not found"
    echo "Please install jq for JSON parsing: sudo apt-get install jq"
    exit 1
fi

main "$@"