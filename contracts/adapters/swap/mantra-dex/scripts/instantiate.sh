#!/usr/bin/env bash
set -e

# Configuration
CHAIN_ID="mantra-dukong-1"
NODE_URL="https://rpc.dukong.mantrachain.io:443"
WALLET_NAME="admin"
CONTRACT_NAME="skip-go-swap-adapter-mantra-dex"

# Mantra DEX contract addresses on mantra-dukong network
POOL_MANAGER_ADDRESS="mantra1vwj600jud78djej7ttq44dktu4wr3t2yrrsjgmld8v3jq8mud68q5w7455"

# Get script directory
SCRIPT_DIR=$(realpath "$0" | sed 's|\(.*\)/.*|\1|')

# Function to display usage
display_usage() {
    echo "Mantra DEX Swap Adapter Instantiation Script"
    echo ""
    echo "Usage: ./instantiate.sh [OPTIONS] <entry_point_address>"
    echo ""
    echo "Arguments:"
    echo "  entry_point_address  The address of the Skip Go entry point contract"
    echo ""
    echo "Options:"
    echo "  -h, --help          Show this help message"
    echo ""
    echo "Examples:"
    echo "  ./instantiate.sh mantra1abc...def    # Instantiate with entry point address"
}

# Check if code_id.txt exists
check_code_id() {
    if [ ! -f "$SCRIPT_DIR/code_id.txt" ]; then
        echo "Error: code_id.txt not found. Please run ./deploy.sh -s first to store the contract."
        exit 1
    fi
    
    CODE_ID=$(cat "$SCRIPT_DIR/code_id.txt")
    if [ -z "$CODE_ID" ]; then
        echo "Error: Empty code_id.txt. Please run ./deploy.sh -s first to store the contract."
        exit 1
    fi
    
    echo "Found Code ID: $CODE_ID"
}

# Instantiate contract
instantiate_contract() {
    local entry_point_address="$1"
    
    echo "Instantiating contract with Code ID: $CODE_ID"
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
    
    echo "Instantiation message:"
    echo "$INIT_MSG"
    
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
    
    # Get transaction hash
    TXHASH=$(echo "$INSTANTIATE_RESULT" | jq -r '.txhash')
    
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
        echo "Transaction hash: $TXHASH" > "$SCRIPT_DIR/instantiate_tx_hash.txt"
        return 1
    fi
    
    echo "Transaction result:"
    echo "$TX_RESULT"
    
    # Extract contract address from transaction result
    CONTRACT_ADDR=$(echo "$TX_RESULT" | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value')
    
    if [ "$CONTRACT_ADDR" == "null" ] || [ -z "$CONTRACT_ADDR" ]; then
        echo "Error: Failed to extract contract address from transaction result"
        echo "Transaction hash: $TXHASH" > "$SCRIPT_DIR/instantiate_tx_hash.txt"
        return 1
    fi
    
    echo ""
    echo "✅ Contract instantiated successfully!"
    echo "Contract Address: $CONTRACT_ADDR"
    echo "Entry Point Address: $entry_point_address"
    echo "Pool Manager Address: $POOL_MANAGER_ADDRESS"
    echo ""
    
    # Save contract address to file
    echo "$CONTRACT_ADDR" > "$SCRIPT_DIR/contract_address.txt"
    echo "Contract address saved to: $SCRIPT_DIR/contract_address.txt"
    
    return 0
}

# Main script logic
main() {
    if [ $# -eq 0 ]; then
        display_usage
        exit 1
    fi
    
    case "$1" in
        -h|--help)
            display_usage
            exit 0
            ;;
        *)
            if [ $# -ne 1 ]; then
                echo "Error: Expected exactly one argument (entry point address)"
                display_usage
                exit 1
            fi
            
            ENTRY_POINT_ADDRESS="$1"
            
            # Validate that the address looks correct (basic validation)
            if [[ ! "$ENTRY_POINT_ADDRESS" =~ ^mantra1[a-z0-9]{38}$ ]]; then
                echo "Error: Invalid entry point address format. Expected mantra1... address"
                exit 1
            fi
            
            check_code_id
            instantiate_contract "$ENTRY_POINT_ADDRESS"
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