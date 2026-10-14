#!/usr/bin/env bash
#
# Deploy and initialize the StellarAid contracts, then record their IDs and WASM
# hashes in deployments/<network>.json. See the README "Deploying" section.
#
# The deployment automation is completed alongside the deployment work; this scaffold
# documents the intended steps so the flow is reviewable from the start.
set -euo pipefail

NETWORK="${1:-testnet}"
SOURCE="${SOURCE:-deployer}"

echo "Network: ${NETWORK}"
echo "Source:  ${SOURCE}"

echo "1. Build optimized WASM (make optimize)"
echo "2. Deploy help_request, reputation, attestation with:"
echo "   stellar contract deploy --wasm target/wasm32v1-none/release/<name>.wasm \\"
echo "     --source ${SOURCE} --network ${NETWORK} --alias <name>"
echo "3. initialize each contract (admin, config)"
echo "4. Write IDs and WASM hashes to deployments/${NETWORK}.json"

echo "TODO: real deployment is implemented in the deployment release."
