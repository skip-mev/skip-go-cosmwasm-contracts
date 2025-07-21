#!/usr/bin/env bash
set -e

# Find workspace root (5 levels up from current script directory)
workspaceRoot=$(realpath "$0" | sed 's|\(.*\)/.*|\1|' | xargs -I{} realpath {}/../../../../../)

echo "Building Skip Go Mantra DEX Swap Adapter..."
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
    docker_command=("docker" "run" "${docker_options[@]}" "cosmwasm/optimizer-arm64:0.16.0" "./contracts/adapters/swap/mantra-dex")
else
    echo "Using standard optimizer for architecture: $arch"
    docker_command=("docker" "run" "${docker_options[@]}" "cosmwasm/optimizer:0.16.0" "./contracts/adapters/swap/mantra-dex")
fi

echo "Running Docker command:"
echo "${docker_command[@]}"

# Execute the Docker command
"${docker_command[@]}"

echo "Build completed successfully!"

# Check if artifacts directory was created
if [ -d "$workspaceRoot/artifacts" ]; then
    echo "Optimized artifacts available in: $workspaceRoot/artifacts"
    ls -la "$workspaceRoot/artifacts"
    
    # Copy artifacts to parent directory for convenience
    cp -r "$workspaceRoot/artifacts" ../
    echo "Artifacts copied to parent directory"
else
    echo "Warning: No artifacts directory found"
fi