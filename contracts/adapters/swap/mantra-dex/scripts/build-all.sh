#!/usr/bin/env bash
set -e

# Find workspace root (5 levels up from current script directory)
workspaceRoot=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | xargs -I{} realpath {}/../../../../../)

echo "Building Skip Go Entry Point and Mantra DEX Swap Adapter contracts..."
echo "Workspace root: $workspaceRoot"

# Docker options for CW Optimizer targeting workspace root
docker_options=(
    --rm
    -v "$workspaceRoot":/code
    --mount type=volume,source="skip-go-contracts_cache",target=/target
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry
)

# Determine architecture and select appropriate Docker optimizer
arch=$(uname -m)

if [[ "$arch" == "aarch64" || "$arch" == "arm64" ]]; then
    echo "Using ARM64 optimizer for architecture: $arch"
    docker_command=("docker" "run" "${docker_options[@]}" "cosmwasm/optimizer-arm64:0.16.0")
else
    echo "Using standard optimizer for architecture: $arch"
    docker_command=("docker" "run" "${docker_options[@]}" "cosmwasm/optimizer:0.16.0")
fi

# Build all three contracts one at a time
echo "Building contracts:"
echo "1. Entry Point Contract: ./contracts/entry-point"
echo "2. IBC Hooks Adapter: ./contracts/adapters/ibc/ibc-hooks"
echo "3. Mantra DEX Adapter: ./contracts/adapters/swap/mantra-dex"

# Build Entry Point contract first
echo "Building Entry Point contract..."
entry_point_command=("${docker_command[@]}" "./contracts/entry-point")
echo "Running: ${entry_point_command[@]}"
"${entry_point_command[@]}"

echo "Entry Point contract built successfully!"

# Build IBC Hooks Adapter contract
echo "Building IBC Hooks Adapter contract..."
ibc_hooks_command=("${docker_command[@]}" "./contracts/adapters/ibc/ibc-hooks")
echo "Running: ${ibc_hooks_command[@]}"
"${ibc_hooks_command[@]}"

echo "IBC Hooks Adapter contract built successfully!"

# Build Mantra DEX Adapter contract
echo "Building Mantra DEX Adapter contract..."
mantra_dex_command=("${docker_command[@]}" "./contracts/adapters/swap/mantra-dex")
echo "Running: ${mantra_dex_command[@]}"
"${mantra_dex_command[@]}"

echo "Mantra DEX Adapter contract built successfully!"

echo "Build completed successfully!"

# Check if artifacts directory was created
if [ -d "$workspaceRoot/artifacts" ]; then
    echo "Optimized artifacts available in: $workspaceRoot/artifacts"
    ls -la "$workspaceRoot/artifacts"
    
    # Copy artifacts to parent directory for convenience
    mkdir -p ../artifacts
    cp -r "$workspaceRoot/artifacts"/* ../artifacts/
    echo "Artifacts copied to parent directory"
    
    echo ""
    echo "Built contracts:"
    echo "- Entry Point: $(find ../artifacts -name "*entry_point*.wasm" 2>/dev/null | head -n 1 || find "$workspaceRoot/artifacts" -name "*entry_point*.wasm" | head -n 1)"
    echo "- IBC Hooks Adapter: $(find ../artifacts -name "*ibc_hooks*.wasm" 2>/dev/null | head -n 1 || find "$workspaceRoot/artifacts" -name "*ibc_hooks*.wasm" | head -n 1)"
    echo "- Mantra DEX Adapter: $(find ../artifacts -name "*mantra_dex*.wasm" 2>/dev/null | head -n 1 || find "$workspaceRoot/artifacts" -name "*mantra_dex*.wasm" | head -n 1)"
else
    echo "Warning: No artifacts directory found"
fi