#!/usr/bin/env bash
#
# Generate TypeScript bindings for the StellarAid contracts. The generated output is
# consumed by the `sdk` repository. See the README "Releasing and generating SDK
# bindings" section.
set -euo pipefail

OUTPUT_DIR="${1:-../sdk/src/generated}"
NETWORK="${NETWORK:-testnet}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

DEPLOYMENTS_FILE="${ROOT_DIR}/deployments/${NETWORK}.json"

if [ ! -f "${DEPLOYMENTS_FILE}" ]; then
  echo "Error: ${DEPLOYMENTS_FILE} not found."
  exit 1
fi

HR_ID=$(jq -r '.contracts.help_request.id' "${DEPLOYMENTS_FILE}")
REP_ID=$(jq -r '.contracts.reputation.id' "${DEPLOYMENTS_FILE}")
ATT_ID=$(jq -r '.contracts.attestation.id' "${DEPLOYMENTS_FILE}")

mkdir -p "${OUTPUT_DIR}"

echo "Generating bindings for Help Request (${HR_ID})..."
stellar contract bindings typescript \
  --contract-id "${HR_ID}" \
  --network "${NETWORK}" \
  --output-dir "${OUTPUT_DIR}/help_request" \
  --overwrite

echo "Generating bindings for Reputation (${REP_ID})..."
stellar contract bindings typescript \
  --contract-id "${REP_ID}" \
  --network "${NETWORK}" \
  --output-dir "${OUTPUT_DIR}/reputation" \
  --overwrite

echo "Generating bindings for Attestation (${ATT_ID})..."
stellar contract bindings typescript \
  --contract-id "${ATT_ID}" \
  --network "${NETWORK}" \
  --output-dir "${OUTPUT_DIR}/attestation" \
  --overwrite

echo "Bindings generation complete! Output located at ${OUTPUT_DIR}"
