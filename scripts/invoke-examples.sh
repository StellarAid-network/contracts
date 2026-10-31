#!/usr/bin/env bash
#
# Example CLI invocations against deployed contracts. Useful for manual verification.
# See the README "Deploying" section.
set -euo pipefail

NETWORK="${NETWORK:-testnet}"
SOURCE="${SOURCE:-deployer}"
REPUTATION_ID="${REPUTATION_ID:-CC46Y6JBSNNTHPZZCLLUILMWMSNLRC3MV46UQ2REQX3N4WI6PTDPOZ2C}"
HELP_REQUEST_ID="${HELP_REQUEST_ID:-CBPW3FPPKPT4YWT7NSWM2EZ5AMONGQQQBPAEYZBSBWSOUGY5B26A7RLL}"

# Read the current badge threshold configuration on reputation.
stellar contract invoke \
  --id "${REPUTATION_ID}" \
  --source "${SOURCE}" \
  --network "${NETWORK}" \
  -- get_config

# Read a help request by id (once request 0 has been created).
# stellar contract invoke \
#   --id "${HELP_REQUEST_ID}" \
#   --source "${SOURCE}" \
#   --network "${NETWORK}" \
#   -- get_request --request_id 0
