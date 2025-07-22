#!/usr/bin/env bash
set -e

# Configuration
CHAIN_ID="mantra-dukong-1"
NODE_URL="https://rpc.dukong.mantrachain.io:443"
WALLET_NAME="admin"

# Mantra DEX contract addresses on mantra-dukong network
POOL_MANAGER_ADDRESS="mantra1vwj600jud78djej7ttq44dktu4wr3t2yrrsjgmld8v3jq8mud68q5w7455"

# Use existing Mantra IBC infrastructure for testing (doesn't need to work)
PLACEHOLDER_IBC_ADDRESS="mantra1cc0jfcd3rv3d36g6m575mdk8p2nmdjgnaf7ngq"

# Salt for deterministic addresses (using "1" as default)
SALT="1"
SALT_HEX="31" # "1" in hex

# Admin addresses
ENTRY_POINT_ADMIN="mantra10tysdwkjuqecgg9npery40dvc8ak9urhf6dj6u"
ADAPTER_ADMIN="mantra1cc0jfcd3rv3d36g6m575mdk8p2nmdjgnaf7ngq"

# Get script directory and workspace root
SCRIPT_DIR=$(realpath "$0" | sed 's|\(.*\)/.*|\1|')
WORKSPACE_ROOT=$(realpath "$SCRIPT_DIR/../../../../../")
ARTIFACTS_DIR="$WORKSPACE_ROOT/artifacts"

# Function to display usage
display_usage() {
    echo "Skip Go Deterministic Deployment Script for Mantra-Dukong"
    echo ""
    echo "Usage: ./deploy-all.sh [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -h, --help           Show this help message"
    echo ""
    echo "This script deploys contracts using the proper deterministic flow:"
    echo "  1. Pre-generate Entry Point address using instantiate2 + salt"
    echo "  2. Deploy Mantra DEX Adapter → pre-generated Entry Point address"
    echo "  3. Deploy Entry Point using instantiate2 → gets exact pre-generated address"
    echo ""
    echo "Clean, deterministic, no circular dependency issues!"
}

# Check if artifacts exist
check_artifacts() {
    if [ ! -d "$ARTIFACTS_DIR" ]; then
        echo "Error: Artifacts directory not found at $ARTIFACTS_DIR"
        echo "Please run './build-all.sh' first to build the contracts"
        exit 1
    fi

    ENTRY_POINT_WASM=$(find "$ARTIFACTS_DIR" -name "*entry_point*.wasm" | head -n 1)
    MANTRA_DEX_WASM=$(find "$ARTIFACTS_DIR" -name "*swap_adapter_mantra_dex*.wasm" | head -n 1)

    if [ -z "$ENTRY_POINT_WASM" ]; then
        echo "Error: Entry point WASM file not found in $ARTIFACTS_DIR"
        echo "Please run './build-all.sh' first to build the contracts"
        exit 1
    fi

    if [ -z "$MANTRA_DEX_WASM" ]; then
        echo "Error: Mantra DEX WASM file not found in $ARTIFACTS_DIR"
        echo "Please run './build-all.sh' first to build the contracts"
        exit 1
    fi

    echo "Found contract artifacts:"
    echo "- Entry Point: $ENTRY_POINT_WASM"
    echo "- Mantra DEX Adapter: $MANTRA_DEX_WASM"
    echo ""
}

# Store contract code and return Code ID
store_contract() {
    local wasm_file="$1"
    local contract_name="$2"

    echo "Storing $contract_name contract code on $CHAIN_ID..."

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
        echo "Error: Failed to get transaction hash for $contract_name"
        echo "Store result: $STORE_RESULT"
        return 1
    fi

    echo "Transaction submitted with hash: $TXHASH"
    echo "Waiting for transaction to be processed..."

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
        echo "Retry $retries/$max_retries: Transaction not yet processed..."
    done

    if [ $retries -eq $max_retries ]; then
        echo "Error: Transaction not processed after $max_retries attempts"
        echo "Check manually: mantrachaind query tx $TXHASH --node $NODE_URL"
        return 1
    fi

    # Extract code ID
    CODE_ID=$(echo "$TX_RESULT" | jq -r '.events[]? | select(.type=="store_code") | .attributes[]? | select(.key=="code_id") | .value' 2>/dev/null | head -n1)

    if [ -n "$CODE_ID" ] && [ "$CODE_ID" != "null" ]; then
        echo "$contract_name stored successfully with Code ID: $CODE_ID"
        echo "$CODE_ID"
        return 0
    else
        echo "Error: Failed to extract Code ID for $contract_name"
        echo "Transaction result:"
        echo "$TX_RESULT" | jq '.'
        return 1
    fi
}

# Get deployer address
get_deployer_address() {
    local deployer_address=$(mantrachaind keys show "$WALLET_NAME" -a 2>/dev/null)
    if [ -z "$deployer_address" ]; then
        echo "Error: Failed to get deployer address for wallet '$WALLET_NAME'"
        return 1
    fi
    echo "$deployer_address"
}

# Pre-generate Entry Point address using instantiate2
pre_generate_entry_point_address() {
    local code_id="$1"
    local deployer_address="$2"

    echo "Pre-generating Entry Point address using instantiate2..."
    echo "Code ID: $code_id"
    echo "Deployer: $deployer_address"
    echo "Salt: $SALT (hex: $SALT_HEX)"

    # Get code hash from stored contract
    CODE_HASH=$(mantrachaind query wasm code-info "$code_id" --node "$NODE_URL" --output json | jq -r '.data_hash')
    
    if [ -z "$CODE_HASH" ] || [ "$CODE_HASH" == "null" ]; then
        echo "Error: Failed to get code hash for code ID $code_id"
        return 1
    fi

    echo "Code Hash: $CODE_HASH"

    # Generate deterministic address
    ENTRY_POINT_ADDRESS=$(mantrachaind query wasm build-address "$CODE_HASH" "$deployer_address" "$SALT_HEX" --output json 2>/dev/null | jq -r '.address // empty')

    if [ -z "$ENTRY_POINT_ADDRESS" ]; then
        echo "Error: Failed to generate deterministic address"
        return 1
    fi

    echo "Pre-generated Entry Point Address: $ENTRY_POINT_ADDRESS"
    echo "$ENTRY_POINT_ADDRESS"
    return 0
}

# Instantiate Mantra DEX Adapter pointing to pre-generated Entry Point
instantiate_mantra_dex_adapter() {
    local code_id="$1"
    local entry_point_address="$2"

    echo "Instantiating Mantra DEX Adapter with pre-generated Entry Point"
    echo "Code ID: $code_id"
    echo "Pre-generated Entry Point Address: $entry_point_address"
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
        --admin "$ADAPTER_ADMIN" \
        --gas auto \
        --gas-adjustment 1.3 \
        --gas-prices "0.01uom" \
        --broadcast-mode sync \
        --yes \
        --output json)

    echo "Mantra DEX Adapter instantiate transaction result:"
    echo "$INSTANTIATE_RESULT"

    # Get transaction hash and wait for processing
    TXHASH=$(echo "$INSTANTIATE_RESULT" | jq -r '.txhash')

    if [ "$TXHASH" == "null" ] || [ -z "$TXHASH" ]; then
        echo "Error: Failed to get transaction hash for Mantra DEX Adapter"
        return 1
    fi

    echo "Transaction submitted with hash: $TXHASH"
    echo "Waiting for transaction to be processed..."
    sleep 15

    TX_RESULT=$(mantrachaind query tx "$TXHASH" --node "$NODE_URL" --output json 2>/dev/null || echo "null")

    if [ "$TX_RESULT" == "null" ]; then
        echo "Transaction not yet processed. Please check manually with:"
        echo "mantrachaind query tx $TXHASH --node $NODE_URL"
        return 1
    fi

    # Extract contract address
    CONTRACT_ADDR=$(echo "$TX_RESULT" | jq -r '.events[]? | select(.type=="instantiate") | .attributes[]? | select(.key=="_contract_address") | .value' 2>/dev/null | head -n1)

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

# Instantiate Entry Point using instantiate2 with deterministic address
instantiate_entry_point_deterministic() {
    local code_id="$1"
    local mantra_dex_address="$2"
    local expected_address="$3"

    echo "Instantiating Entry Point using instantiate2 (deterministic)"
    echo "Code ID: $code_id"
    echo "Mantra DEX Address: $mantra_dex_address"
    echo "Expected Address: $expected_address"
    echo "Salt: $SALT"

    # Create instantiation message with Mantra DEX configured
    INIT_MSG=$(cat <<EOF
{
  "swap_venues": [
    {
      "name": "mantra-dex",
      "adapter_contract_address": "$mantra_dex_address"
    }
  ],
  "ibc_transfer_contract_address": "$PLACEHOLDER_IBC_ADDRESS",
  "hyperlane_transfer_contract_address": null
}
EOF
)

    echo "Entry Point instantiation message:"
    echo "$INIT_MSG"

    INSTANTIATE_RESULT=$(mantrachaind tx wasm instantiate2 "$code_id" "$SALT" "$INIT_MSG" \
        --from "$WALLET_NAME" \
        --chain-id "$CHAIN_ID" \
        --node "$NODE_URL" \
        --label "skip-go-entry-point-deterministic" \
        --admin "$ENTRY_POINT_ADMIN" \
        --gas auto \
        --gas-adjustment 1.3 \
        --gas-prices "0.01uom" \
        --broadcast-mode sync \
        --yes \
        --output json)

    echo "Entry Point instantiate2 transaction result:"
    echo "$INSTANTIATE_RESULT"

    # Get transaction hash and wait for processing
    TXHASH=$(echo "$INSTANTIATE_RESULT" | jq -r '.txhash')

    if [ "$TXHASH" == "null" ] || [ -z "$TXHASH" ]; then
        echo "Error: Failed to get transaction hash for Entry Point"
        return 1
    fi

    echo "Transaction submitted with hash: $TXHASH"
    echo "Waiting for transaction to be processed..."
    sleep 15

    TX_RESULT=$(mantrachaind query tx "$TXHASH" --node "$NODE_URL" --output json 2>/dev/null || echo "null")

    if [ "$TX_RESULT" == "null" ]; then
        echo "Transaction not yet processed. Please check manually with:"
        echo "mantrachaind query tx $TXHASH --node $NODE_URL"
        return 1
    fi

    # Extract contract address
    CONTRACT_ADDR=$(echo "$TX_RESULT" | jq -r '.events[]? | select(.type=="instantiate") | .attributes[]? | select(.key=="_contract_address") | .value' 2>/dev/null | head -n1)

    if [ "$CONTRACT_ADDR" != "null" ] && [ -n "$CONTRACT_ADDR" ]; then
        echo "Entry Point contract instantiated successfully!"
        echo "Actual Address: $CONTRACT_ADDR"
        
        # Verify it matches expected address
        if [ "$CONTRACT_ADDR" == "$expected_address" ]; then
            echo "✅ Address matches pre-generated address! Deterministic deployment successful."
        else
            echo "⚠️  Warning: Address mismatch!"
            echo "   Expected: $expected_address"
            echo "   Actual:   $CONTRACT_ADDR"
        fi
        
        echo "$CONTRACT_ADDR"
        return 0
    else
        echo "Error: Failed to extract Entry Point contract address"
        return 1
    fi
}

# Main deployment function - Proper Deterministic Flow
deploy_deterministic() {
    echo "=========================================="
    echo "🏗️  DETERMINISTIC DEPLOYMENT FLOW 🏗️ "
    echo "=========================================="
    echo ""
    echo "Using instantiate2 for predictable addresses"
    echo "to solve circular dependency cleanly!"
    echo ""

    # Step 1: Store contracts
    echo "=== STEP 1: Storing Contracts ==="
    echo ""

    echo "Storing Entry Point Contract..."
    ENTRY_POINT_CODE_ID=$(store_contract "$ENTRY_POINT_WASM" "Entry Point")
    if [ $? -ne 0 ]; then
        echo "Failed to store Entry Point contract"
        exit 1
    fi
    echo "$ENTRY_POINT_CODE_ID" > "$SCRIPT_DIR/entry_point_code_id.txt"
    echo ""

    echo "Storing Mantra DEX Adapter Contract..."
    MANTRA_DEX_CODE_ID=$(store_contract "$MANTRA_DEX_WASM" "Mantra DEX Adapter")
    if [ $? -ne 0 ]; then
        echo "Failed to store Mantra DEX Adapter contract"
        exit 1
    fi
    echo "$MANTRA_DEX_CODE_ID" > "$SCRIPT_DIR/mantra_dex_code_id.txt"
    echo ""

    # Step 2: Get deployer address
    echo "=== STEP 2: Getting Deployer Address ==="
    DEPLOYER_ADDRESS=$(get_deployer_address)
    if [ $? -ne 0 ]; then
        echo "Failed to get deployer address"
        exit 1
    fi
    echo "Deployer Address: $DEPLOYER_ADDRESS"
    echo ""

    # Step 3: Pre-generate Entry Point address
    echo "=== STEP 3: Pre-generating Entry Point Address ==="
    PRE_GENERATED_ENTRY_POINT=$(pre_generate_entry_point_address "$ENTRY_POINT_CODE_ID" "$DEPLOYER_ADDRESS")
    if [ $? -ne 0 ]; then
        echo "Failed to pre-generate Entry Point address"
        exit 1
    fi
    echo ""

    # Step 4: Deploy Mantra DEX Adapter → pre-generated Entry Point
    echo "=== STEP 4: Deploy Mantra DEX Adapter → Pre-generated Entry Point ==="
    MANTRA_DEX_ADDRESS=$(instantiate_mantra_dex_adapter "$MANTRA_DEX_CODE_ID" "$PRE_GENERATED_ENTRY_POINT")
    if [ $? -ne 0 ]; then
        echo "Failed to instantiate Mantra DEX Adapter contract"
        exit 1
    fi
    echo "$MANTRA_DEX_ADDRESS" > "$SCRIPT_DIR/mantra_dex_address.txt"
    echo ""

    # Step 5: Deploy Entry Point using instantiate2 → gets pre-generated address
    echo "=== STEP 5: Deploy Entry Point using instantiate2 ==="
    ENTRY_POINT_ADDRESS=$(instantiate_entry_point_deterministic "$ENTRY_POINT_CODE_ID" "$MANTRA_DEX_ADDRESS" "$PRE_GENERATED_ENTRY_POINT")
    if [ $? -ne 0 ]; then
        echo "Failed to instantiate Entry Point contract"
        exit 1
    fi
    echo "$ENTRY_POINT_ADDRESS" > "$SCRIPT_DIR/entry_point_address.txt"
    echo ""

    # Summary
    echo "=========================================="
    echo "🎉 DETERMINISTIC DEPLOYMENT COMPLETED! 🎉"
    echo "=========================================="
    echo ""
    echo "✅ No circular dependency issues!"
    echo "✅ No contract waste!"
    echo "✅ Predictable addresses!"
    echo ""
    echo "Final Contract Addresses:"
    echo ""
    echo "Entry Point Contract:"
    echo "  Code ID: $ENTRY_POINT_CODE_ID"
    echo "  Address: $ENTRY_POINT_ADDRESS"
    echo ""
    echo "Mantra DEX Adapter Contract:"
    echo "  Code ID: $MANTRA_DEX_CODE_ID"
    echo "  Address: $MANTRA_DEX_ADDRESS"
    echo ""
    echo "Configuration:"
    echo "  Pool Manager: $POOL_MANAGER_ADDRESS"
    echo "  IBC (Placeholder): $PLACEHOLDER_IBC_ADDRESS"
    echo "  Salt Used: $SALT"
    echo ""
    echo "Files saved:"
    echo "  - entry_point_code_id.txt / entry_point_address.txt"
    echo "  - mantra_dex_code_id.txt / mantra_dex_address.txt"
    echo ""
    echo "🎯 SYSTEM IS READY FOR SWAP TESTING!"
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
            deploy_deterministic
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