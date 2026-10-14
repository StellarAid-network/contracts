#!/usr/bin/env bash
#
# Generate TypeScript bindings for the StellarAid contracts. The generated output is
# consumed by the `sdk` repository. See the README "Releasing and generating SDK
# bindings" section.
#
# Bindings generation is completed with the SDK sync work; this scaffold documents the
# command shape.
set -euo pipefail

OUTPUT_DIR="${1:-./bindings}"
NETWORK="${NETWORK:-testnet}"

echo "Output: ${OUTPUT_DIR}"
echo "Network: ${NETWORK}"

echo "stellar contract bindings typescript \\"
echo "  --contract-id <CONTRACT_ID> \\"
echo "  --network ${NETWORK} \\"
echo "  --output-dir ${OUTPUT_DIR}"

echo "TODO: wire this into the SDK sync script."
