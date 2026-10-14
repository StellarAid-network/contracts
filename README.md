# 📜 stellar-aid / contracts

**Soroban smart contracts that power StellarAid: help-request escrow, funding goals, reputation, and attestations.**

![Rust](https://img.shields.io/badge/rust-stable-orange)
![Soroban](https://img.shields.io/badge/soroban-smart%20contracts-7B61FF)
![License](https://img.shields.io/badge/license-MIT-green)
![Status](https://img.shields.io/badge/status-in%20development-yellow)

This repository is the **source of truth for on-chain rules** in the StellarAid ecosystem. Every release here produces WASM binaries and TypeScript bindings that are published through the [`sdk`](https://github.com/stellar-aid/sdk) repo.

---

## Table of contents

1. [Overview](#overview)
2. [Contracts](#contracts)
3. [Repository layout](#repository-layout)
4. [Prerequisites](#prerequisites)
5. [Getting started](#getting-started)
6. [Build, test, lint](#build-test-lint)
7. [Deploying](#deploying)
8. [Contract reference](#contract-reference)
9. [Events](#events)
10. [Errors](#errors)
11. [Security model](#security-model)
12. [Storage and cost considerations](#storage-and-cost-considerations)
13. [Upgrades and versioning](#upgrades-and-versioning)
14. [Releasing and generating SDK bindings](#releasing-and-generating-sdk-bindings)
15. [CI/CD](#cicd)
16. [Contributing](#contributing)
17. [License](#license)

---

## Overview

Not everything needs a contract. StellarAid deliberately splits responsibilities:

| Feature | Implemented with | Lives in |
|---|---|---|
| Direct donations and tips | Native Stellar payments | `sdk` / `web` |
| Giveaways | **Native claimable balances** | `sdk` / `web` |
| Help-request escrow, goals, refunds, milestones | **Soroban contract** | **this repo** |
| Reputation scores and badges | **Soroban contract** | **this repo** |
| Verified-need attestations | **Soroban contract** | **this repo** |

Contracts are kept small and single-purpose to reduce audit surface and cost.

---

## Contracts

| Contract | Crate | Purpose | Status |
|---|---|---|---|
| Help Request | `contracts/help_request` | Funding-goal escrow with deadline, refunds, and milestone release | 🚧 |
| Reputation | `contracts/reputation` | Records scored events and badges per address | 📋 |
| Attestation | `contracts/attestation` | Lets approved attestors vouch for a request or user | 📋 |

### Help Request (summary)

1. A **creator** opens a request: token (for example USDC), goal amount, deadline, milestone schedule, and a content hash pointing at the off-chain description (IPFS CID or SHA-256).
2. **Contributors** transfer tokens into the contract.
3. **If the goal is reached** by the deadline, the request becomes `Funded` and milestones can be released.
4. **If the goal is not reached**, the request becomes `Failed` and each contributor can call `refund`.
5. **Milestones** are released when the configured approver set confirms (creator-provided evidence hash plus approvals).
6. Unreleased funds can be refunded if the request is cancelled or milestones are rejected.

State machine:

```
        create
          │
          ▼
       ┌──────┐  deadline passed,   ┌────────┐
       │ Open │── goal not met ────►│ Failed │──► contributors refund()
       └──┬───┘                     └────────┘
          │ goal met
          ▼
      ┌────────┐  milestone approved   ┌───────────┐
      │ Funded │──────────────────────►│ Releasing │──► ... ──► Completed
      └────┬───┘                       └─────┬─────┘
           │ cancel / rejected               │ rejected
           ▼                                 ▼
       ┌───────────┐                  ┌───────────┐
       │ Cancelled │──► refund()      │ Disputed  │
       └───────────┘                  └───────────┘
```

*(Exact states and transitions are defined in `contracts/help_request/src/types.rs` and are the authoritative reference.)*

### Reputation (summary)

- Authorized reporter contracts (for example the Help Request contract) call `record_event` to add or subtract score for an address.
- Scores are derived from verifiable outcomes: completed requests, successful milestone delivery, refunds after failure, upheld reports.
- Badges are awarded at thresholds and are read-only for everyone else.

### Attestation (summary)

- An **admin-managed registry of attestors** (initially trusted organizations, later community-governed).
- Attestors sign statements such as "request X is genuine" or "user Y is verified", stored as a hash plus metadata with an expiry.

---

## Repository layout

```
contracts/
├── Cargo.toml                  # Workspace manifest
├── Makefile                    # Common tasks (build, test, deploy, bindings)
├── rust-toolchain.toml         # Pinned toolchain
├── contracts/
│   ├── help_request/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs          # Contract entrypoints
│   │       ├── types.rs        # Structs, enums, storage keys
│   │       ├── errors.rs       # Error enum
│   │       ├── events.rs       # Event helpers
│   │       └── test.rs         # Unit and integration tests
│   ├── reputation/
│   └── attestation/
├── shared/                     # Shared crate: common types, helpers, constants
├── scripts/
│   ├── deploy.sh               # Deploy and initialize contracts
│   ├── bindings.sh             # Generate TypeScript bindings
│   └── invoke-examples.sh      # Example CLI invocations
├── deployments/
│   ├── testnet.json            # Contract IDs and wasm hashes (testnet)
│   └── mainnet.json            # Contract IDs and wasm hashes (mainnet)
├── audits/                     # Published audit reports
├── docs/                       # Design docs and threat model
└── .github/workflows/          # CI and release workflows
```

---

## Prerequisites

| Tool | Version | Notes |
|---|---|---|
| Rust | stable (pinned in `rust-toolchain.toml`) | Install with [rustup](https://rustup.rs) |
| WASM target | per your Stellar CLI version | Newer CLI versions use `wasm32v1-none`, older ones use `wasm32-unknown-unknown`. Check `stellar contract build` output. |
| Stellar CLI | latest stable | See the [Stellar docs](https://developers.stellar.org/docs/tools/cli) |
| Docker (optional) | any recent | For running a local Stellar quickstart network |

```bash
# Verify
rustc --version
stellar --version
```

---

## Getting started

```bash
git clone https://github.com/stellar-aid/contracts.git
cd contracts

# Build all contracts to WASM
stellar contract build

# Run all tests
cargo test
```

### Create a funded testnet identity

```bash
stellar keys generate deployer --network testnet --fund
stellar keys address deployer
```

---

## Build, test, lint

```bash
make build        # stellar contract build for the whole workspace
make test         # cargo test --workspace
make lint         # cargo clippy --workspace --all-targets -- -D warnings
make fmt          # cargo fmt --all
make optimize     # build optimized WASM for deployment
make coverage     # test coverage report (cargo-llvm-cov)
```

Testing guidelines:

- Use Soroban's test environment (`soroban_sdk::testutils`) with mocked auth for unit tests.
- Every public function needs tests for the **success path**, **each error variant**, and **authorization failures**.
- Test deadlines and ledger time by setting the ledger timestamp in the test env.
- Add **property-style tests** for fund conservation: total in equals total out plus remaining balance.

---

## Deploying

### 1. Build optimized WASM

```bash
make optimize
```

### 2. Deploy to testnet

```bash
./scripts/deploy.sh testnet
```

Under the hood this runs steps like:

```bash
stellar contract deploy \
  --wasm target/wasm32v1-none/release/help_request.wasm \
  --source deployer \
  --network testnet \
  --alias help_request
```

Then it initializes the contract (admin, allowed tokens, fee config, reputation contract address) and writes IDs to `deployments/testnet.json`.

### 3. Verify

```bash
stellar contract invoke --id help_request --source deployer --network testnet -- get_config
```

### Mainnet checklist

- [ ] Independent audit completed and report committed to `/audits`
- [ ] All findings fixed or accepted with written rationale
- [ ] Admin is a **multisig** account, never a single hot key
- [ ] Allowed token list reviewed (for example the canonical USDC issuer)
- [ ] Testnet soak period completed with realistic traffic
- [ ] Deployment performed from a clean, tagged release in CI
- [ ] `deployments/mainnet.json` updated and `sdk` released

---

## Contract reference

> The signatures below describe the intended interface. The generated docs (`cargo doc --open`) and `src/lib.rs` are authoritative.

### `help_request`

| Function | Auth | Description |
|---|---|---|
| `initialize(admin, config)` | admin | One-time setup |
| `create_request(creator, token, goal, deadline, milestones, content_hash) -> u64` | creator | Open a new request and return its id |
| `contribute(request_id, contributor, amount)` | contributor | Transfer tokens into escrow |
| `finalize(request_id)` | anyone | After the deadline, moves the request to `Funded` or `Failed` |
| `submit_milestone(request_id, index, evidence_hash)` | creator | Submit proof for a milestone |
| `approve_milestone(request_id, index)` | approver | Approve a milestone |
| `release_milestone(request_id, index)` | anyone | Pay the milestone amount once enough approvals exist |
| `refund(request_id, contributor)` | contributor | Withdraw contribution if failed or cancelled |
| `cancel(request_id)` | creator | Cancel before funded, enabling refunds |
| `get_request(request_id) -> Request` | none | Read request state |
| `get_contribution(request_id, contributor) -> i128` | none | Read a contributor's total |
| `set_config(config)` | admin | Update fee and token settings |

### `reputation`

| Function | Auth | Description |
|---|---|---|
| `initialize(admin)` | admin | One-time setup |
| `add_reporter(address)` / `remove_reporter(address)` | admin | Manage contracts allowed to record events |
| `record_event(subject, kind, delta, ref_id)` | reporter | Apply a score change tied to a reference |
| `get_score(subject) -> i64` | none | Current score |
| `get_badges(subject) -> Vec<Badge>` | none | Earned badges |

### `attestation`

| Function | Auth | Description |
|---|---|---|
| `add_attestor(address, meta_hash)` / `remove_attestor(address)` | admin | Manage attestors |
| `attest(attestor, subject, kind, content_hash, expires_at)` | attestor | Create an attestation |
| `revoke(attestor, attestation_id)` | attestor | Revoke an attestation |
| `get_attestations(subject) -> Vec<Attestation>` | none | Read attestations |

---

## Events

Contracts emit events that the `api` indexer consumes. Treat event shapes as **public API**: changing them is a breaking change.

| Contract | Event | Data |
|---|---|---|
| help_request | `created` | request_id, creator, token, goal, deadline |
| help_request | `contributed` | request_id, contributor, amount, total_raised |
| help_request | `funded` / `failed` | request_id, total_raised |
| help_request | `milestone_submitted` / `milestone_approved` / `milestone_released` | request_id, index, ... |
| help_request | `refunded` | request_id, contributor, amount |
| help_request | `cancelled` | request_id |
| reputation | `score_changed` | subject, delta, new_score, ref_id |
| attestation | `attested` / `revoked` | attestation_id, attestor, subject |

> Soroban RPC retains events only for a limited window, so the indexer must run continuously and persist its cursor.

---

## Errors

Each contract defines a `#[contracterror]` enum with stable numeric codes. Example for `help_request`:

| Code | Name | Meaning |
|---|---|---|
| 1 | `NotInitialized` | Contract has not been initialized |
| 2 | `AlreadyInitialized` | Initialization attempted twice |
| 3 | `RequestNotFound` | Unknown request id |
| 4 | `InvalidAmount` | Amount is zero or negative |
| 5 | `DeadlinePassed` | Action not allowed after the deadline |
| 6 | `DeadlineNotReached` | Finalize attempted too early |
| 7 | `NotOpen` | Request is not accepting contributions |
| 8 | `Unauthorized` | Caller lacks permission |
| 9 | `TokenNotAllowed` | Token is not on the allowed list |
| 10 | `NothingToRefund` | No refundable balance |

*(Numbers are illustrative. Do not renumber codes once released.)*

---

## Security model

**Principles**

- **Check authorization on every state-changing function** using `require_auth()`.
- **Checks-effects-interactions**: update state before external token transfers.
- **No admin custody**: the admin can change configuration but **can never withdraw escrowed funds**.
- **Fund conservation**: accounting invariants are tested and documented.
- **Checked arithmetic**: all math uses checked operations. No silent overflow.
- **Token allowlist**: only approved assets can be used, to avoid malicious token contracts.
- **Bounded loops and storage**: no unbounded iteration over user-controlled data.

**Threat model highlights**

| Threat | Mitigation |
|---|---|
| Malicious token contract | Allowlist of tokens |
| Creator withdraws and disappears | Escrow plus milestone approvals |
| Fake milestone evidence | Approver set, attestations, dispute path |
| Admin key compromise | Multisig admin, no fund-withdrawal path |
| Griefing with dust contributions | Minimum contribution amount |
| Storage exhaustion | Per-contribution rent paid through fees, bounded structures |
| Reentrancy via token callbacks | State updated before transfers |

Full threat model: [`docs/threat-model.md`](./docs/threat-model.md).

**Reporting vulnerabilities:** see [SECURITY.md](https://github.com/stellar-aid/.github/blob/main/SECURITY.md). Do not open public issues.

---

## Storage and cost considerations

Soroban storage has rent and size limits, so design choices matter:

- Use **persistent** storage for requests and balances, **instance** storage for config, and **temporary** storage only for disposable data.
- Keep entries small. Store **content hashes**, never descriptions or media.
- Extend TTL on read and write paths for active data (`extend_ttl`).
- Avoid a single giant `Vec` of contributors. Use one key per contributor (`(request_id, contributor)`) instead.

---

## Upgrades and versioning

- Contracts follow **semantic versioning**, tracked in each crate's `Cargo.toml` and in git tags (`help_request-v1.2.0`).
- Upgrades use Soroban's WASM upgrade mechanism (`update_current_contract_wasm`), restricted to the admin multisig and gated by a **time-lock** on mainnet.
- A change to a function signature, event shape, or storage layout is **breaking** and requires a major bump, a migration plan, and a matching major release of `sdk`.
- Storage keys are versioned. Migrations are explicit functions covered by tests.

---

## Releasing and generating SDK bindings

```bash
# 1. Tag and build
git tag help_request-v1.0.0
make optimize

# 2. Generate TypeScript bindings
./scripts/bindings.sh
# wraps: stellar contract bindings typescript --wasm <path> --output-dir <dir>
#        (or --contract-id <id> --network testnet)

# 3. Deployments file is committed alongside the tag
cat deployments/testnet.json
```

CI automates this: on a release tag it builds WASM, runs the full test suite, generates bindings, and opens a PR (or publishes) against the [`sdk`](https://github.com/stellar-aid/sdk) repo so it ships a matching version.

Example `deployments/testnet.json`:

```json
{
  "network": "testnet",
  "networkPassphrase": "Test SDF Network ; September 2015",
  "contracts": {
    "help_request": { "id": "C...", "wasmHash": "..." },
    "reputation":   { "id": "C...", "wasmHash": "..." },
    "attestation":  { "id": "C...", "wasmHash": "..." }
  }
}
```

---

## CI/CD

| Workflow | Trigger | Does |
|---|---|---|
| `ci.yml` | Pull request | fmt, clippy, tests, WASM build, size check |
| `audit.yml` | Weekly and on PR | `cargo audit` and `cargo deny` for dependency advisories |
| `deploy-testnet.yml` | Merge to `main` | Deploys to testnet and updates `deployments/testnet.json` |
| `release.yml` | Version tag | Builds, generates bindings, publishes artifacts, notifies `sdk` |

Mainnet deployment is **manual**, requires approval from two maintainers, and runs from a tagged release.

---

## Contributing

1. Open an issue describing the change. Contract changes that alter ABI or storage need an RFC in the org [`.github`](https://github.com/stellar-aid/.github) repo.
2. Branch from `main`, and write tests first for security-relevant logic.
3. Run `make fmt lint test` before pushing.
4. Use Conventional Commits and fill in the PR template, including **security considerations**.
5. At least one maintainer review is required. Two for anything touching fund movement.

See the org [Contributing guide](https://github.com/stellar-aid/.github/blob/main/CONTRIBUTING.md).

---

## License

MIT. See [LICENSE](./LICENSE).
