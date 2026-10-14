#!/usr/bin/env bash
#
# Example CLI invocations against a deployed contract. Useful for manual verification.
# See the README "Deploying" section.
set -euo pipefail

NETWORK="${NETWORK:-testnet}"
SOURCE="${SOURCE:-deployer}"
HELP_REQUEST_ID="${HELP_REQUEST_ID:-help_request}"

# Read the current configuration.
stellar contract invoke \
  --id "${HELP_REQUEST_ID}" \
  --source "${SOURCE}" \
  --network "${NETWORK}" \
  -- get_config

# Read a request by id.
# stellar contract invoke \
#   --id "${HELP_REQUEST_ID}" \
#   --source "${SOURCE}" \
#   --network "${NETWORK}" \
#   -- get_request --request_id 1
