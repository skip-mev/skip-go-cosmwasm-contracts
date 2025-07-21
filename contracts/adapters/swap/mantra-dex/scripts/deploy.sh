#!/usr/bin/env bash
set -e

# Configuration
CHAIN_ID="mantra-dukong-1"
NODE_URL="https://rpc.dukong.mantrachain.io:443"
WALLET_NAME="admin"
CONTRACT_NAME="skip-go-swap-adapter-mantra-dex"

# Get script directory and workspace root
SCRIPT_DIR=$(realpath "$0" | sed 's|\(.*\)/.*|\1|')
WORKSPACE_ROOT=$(realpath "$SCRIPT_DIR/../../../../../")
ARTIFACTS_DIR="$WORKSPACE_ROOT/artifacts"

# Function to display usage
display_usage() {
    echo "Mantra DEX Swap Adapter Deployment Script"
    echo ""
    echo "Usage: ./deploy.sh [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -s, --store-only     Store contract code only (don't instantiate)"
    echo "  -d, --deploy         Store and instantiate contract"
    echo "  -h, --help          Show this help message"
    echo ""
    echo "Examples:"
    echo "  ./deploy.sh -s       # Store contract code only"
    echo "  ./deploy.sh -d       # Store and instantiate contract"
}

# Check if artifacts exist
check_artifacts() {
    if [ ! -d "$ARTIFACTS_DIR" ]; then
        echo "Error: Artifacts directory not found at $ARTIFACTS_DIR"
        echo "Please run ./build.sh first to build the contract"
        exit 1
    fi
    
    WASM_FILE="$ARTIFACTS_DIR/skip_go_swap_adapter_mantra_dex.wasm"
    if [ ! -f "$WASM_FILE" ]; then
        echo "Error: Contract WASM file not found at $WASM_FILE"
        echo "Please run ./build.sh first to build the contract"
        exit 1
    fi
    
    echo "Found contract artifact: $WASM_FILE"
}

# Store contract code
store_contract() {
    echo "Storing contract code on $CHAIN_ID..."
    
    STORE_RESULT=$(mantrachaind tx wasm store "$WASM_FILE" \
        --from "$WALLET_NAME" \
        --chain-id "$CHAIN_ID" \
        --node "$NODE_URL" \
        --gas auto \
        --gas-adjustment 1.3 \
        --gas-prices "0.01uom" \
        --broadcast-mode sync \
        --yes \
        --output json)
    
    echo "Store transaction result:"
    echo "$STORE_RESULT"
    
    # Get transaction hash
    TXHASH=$(echo "$STORE_RESULT" | jq -r '.txhash')
    
    if [ "$TXHASH" == "null" ] || [ -z "$TXHASH" ]; then
        echo "Error: Failed to get transaction hash"
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
        echo "Transaction hash: $TXHASH" > "$SCRIPT_DIR/store_tx_hash.txt"
        return 1
    fi
    
    echo "Transaction result:"
    echo "$TX_RESULT"
    
    # Extract code ID from transaction result
    CODE_ID=$(echo "$TX_RESULT" | jq -r '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value')
    
    if [ "$CODE_ID" != "null" ] && [ -n "$CODE_ID" ]; then
        echo "Contract stored successfully with Code ID: $CODE_ID"
        echo "$CODE_ID" > "$SCRIPT_DIR/code_id.txt"
        return 0
    else
        echo "Error: Failed to extract Code ID from transaction result"
        echo "Transaction hash: $TXHASH" > "$SCRIPT_DIR/store_tx_hash.txt"
        return 1
    fi
}

# Instantiate contract
instantiate_contract() {
    if [ ! -f "$SCRIPT_DIR/code_id.txt" ]; then
        echo "Error: code_id.txt not found. Please store the contract first."
        return 1
    fi
    
    CODE_ID=$(cat "$SCRIPT_DIR/code_id.txt")
    echo "Instantiating contract with Code ID: $CODE_ID"
    
    # Default instantiation message - modify as needed for your contract
    INIT_MSG='{"admin":"mantra1admin..."}'  # Replace with actual admin address
    
    INSTANTIATE_RESULT=$(mantrachaind tx wasm instantiate "$CODE_ID" "$INIT_MSG" \
        --from "$WALLET_NAME" \
        --chain-id "$CHAIN_ID" \
        --node "$NODE_URL" \
        --label "$CONTRACT_NAME" \
        --gas auto \
        --gas-adjustment 1.3 \
        --gas-prices "0.01uom" \
        --broadcast-mode sync \
        --yes \
        --output json)
    
    echo "Instantiate transaction result:"
    echo "$INSTANTIATE_RESULT"
    
    # Extract contract address
    CONTRACT_ADDR=$(echo "$INSTANTIATE_RESULT" | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value')
    
    if [ "$CONTRACT_ADDR" != "null" ] && [ -n "$CONTRACT_ADDR" ]; then
        echo "Contract instantiated successfully!"
        echo "Contract Address: $CONTRACT_ADDR"
        echo "$CONTRACT_ADDR" > "$SCRIPT_DIR/contract_address.txt"
        return 0
    else
        echo "Error: Failed to extract contract address from transaction result"
        return 1
    fi
}

# Main script logic
main() {
    if [ $# -eq 0 ]; then
        display_usage
        exit 1
    fi
    
    check_artifacts
    
    case "$1" in
        -s|--store-only)
            echo "Storing contract code only..."
            store_contract
            ;;
        -d|--deploy)
            echo "Storing and instantiating contract..."
            if store_contract; then
                echo "Contract stored, proceeding with instantiation..."
                instantiate_contract
            else
                echo "Failed to store contract, aborting deployment"
                exit 1
            fi
            ;;
        -h|--help)
            display_usage
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