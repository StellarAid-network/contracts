#!/usr/bin/env bash
#
# Deploy and initialize the StellarAid contracts, then record their IDs and WASM
# hashes in deployments/<network>.json. See the README "Deploying" section.
set -euo pipefail

NETWORK="${1:-testnet}"
SOURCE="${SOURCE:-deployer}"

echo "Deploying StellarAid contracts to network: ${NETWORK} using source: ${SOURCE}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${ROOT_DIR}"

echo "1. Building optimized WASM contracts..."
stellar contract build

WASM_DIR="target/wasm32v1-none/release"

echo "2. Deploying reputation contract..."
REP_ID=$(stellar contract deploy --wasm "${WASM_DIR}/reputation.wasm" --source "${SOURCE}" --network "${NETWORK}")
echo "Reputation contract deployed: ${REP_ID}"

echo "3. Deploying attestation contract..."
ATT_ID=$(stellar contract deploy --wasm "${WASM_DIR}/attestation.wasm" --source "${SOURCE}" --network "${NETWORK}")
echo "Attestation contract deployed: ${ATT_ID}"

echo "4. Deploying help_request contract..."
HR_ID=$(stellar contract deploy --wasm "${WASM_DIR}/help_request.wasm" --source "${SOURCE}" --network "${NETWORK}")
echo "Help Request contract deployed: ${HR_ID}"

DEPLOYER_ADDR=$(stellar keys address "${SOURCE}")

echo "5. Initializing reputation contract..."
stellar contract invoke --id "${REP_ID}" --source "${SOURCE}" --network "${NETWORK}" -- initialize --admin "${DEPLOYER_ADDR}"

echo "6. Initializing attestation contract..."
stellar contract invoke --id "${ATT_ID}" --source "${SOURCE}" --network "${NETWORK}" -- initialize --admin "${DEPLOYER_ADDR}"

echo "7. Authorizing help_request as reporter in reputation..."
stellar contract invoke --id "${REP_ID}" --source "${SOURCE}" --network "${NETWORK}" -- add_reporter --reporter "${HR_ID}"

echo "8. Initializing help_request contract..."
NATIVE_TOKEN=$(stellar contract id asset --asset native --network "${NETWORK}")
stellar contract invoke --id "${HR_ID}" --source "${SOURCE}" --network "${NETWORK}" -- initialize \
  --admin "${DEPLOYER_ADDR}" \
  --config "{\"allowed_tokens\":[\"${NATIVE_TOKEN}\"],\"approval_threshold\":1,\"approvers\":[\"${DEPLOYER_ADDR}\"],\"min_contribution\":\"1000000\",\"reputation_contract\":\"${REP_ID}\"}"

echo "9. Writing deployment IDs to deployments/${NETWORK}.json..."
cat <<EOF > "deployments/${NETWORK}.json"
{
  "network": "${NETWORK}",
  "networkPassphrase": "Test SDF Network ; September 2015",
  "contracts": {
    "help_request": {
      "id": "${HR_ID}",
      "wasmHash": "$(sha256sum ${WASM_DIR}/help_request.wasm | cut -d' ' -f1)"
    },
    "reputation": {
      "id": "${REP_ID}",
      "wasmHash": "$(sha256sum ${WASM_DIR}/reputation.wasm | cut -d' ' -f1)"
    },
    "attestation": {
      "id": "${ATT_ID}",
      "wasmHash": "$(sha256sum ${WASM_DIR}/attestation.wasm | cut -d' ' -f1)"
    }
  }
}
EOF

echo "Deployment complete!"
