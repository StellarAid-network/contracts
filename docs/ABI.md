# StellarAid Smart Contracts — Public ABI Reference (`v0.1.0-testnet`)

This document is the **authoritative compatibility contract** between `contracts`, [`sdk`](https://github.com/StellarAid-network/sdk), and the [`api`](https://github.com/StellarAid-network/api) indexer.

- **Workspace Version:** `0.1.0` (`v0.1.0-testnet`)
- **Soroban SDK:** `29.0.1`
- **Storage Schema Version:** `STORAGE_VERSION_V1 = 1`
- **Breaking Change Policy:** Any modification to function names, parameter types/ordering, return types, event topics/payloads, or numeric error codes requires an approved RFC in `.github/rfcs/`, a major version bump across `contracts` and `sdk`, and a storage migration plan.

---

## 1. Deployed Testnet Addresses (`deployments/testnet.json`)

| Contract | Contract ID | Optimized WASM SHA-256 (`wasmHash`) |
|---|---|---|
| `help_request` | `CBPW3FPPKPT4YWT7NSWM2EZ5AMONGQQQBPAEYZBSBWSOUGY5B26A7RLL` | `273b024842b5c11fd85deb2766fd6b7cf54c595149a62efcc2b6a0e7d060087e` |
| `reputation` | `CC46Y6JBSNNTHPZZCLLUILMWMSNLRC3MV46UQ2REQX3N4WI6PTDPOZ2C` | `f9b88fb4a31247ab6eaff4845622c7db28358c3e6cc9bd52add577ac36ffb6cf` |
| `attestation` | `CBGSEMRF6DKWPGYGFJBQG5UEUBGFDHEJQUB5EOYTS4C746J54AYAS7GN` | `21b89312d055119971c3e7c9e7e21c314d14edab292ac3fa9e1236579b0454b2` |

---

## 2. `help_request` Contract (`HelpRequest`)

### 2.1 Public Types

#### `Status` (`#[contracttype]` enum)
- `Open` — Accepting contributions before `deadline`.
- `Funded` — `raised >= goal`; milestone evidence may be submitted and approved.
- `Releasing` — At least one milestone paid out; subsequent milestones pending.
- `Completed` — All scheduled milestones released.
- `Failed` — `deadline` elapsed with `raised < goal`; 100% refunds enabled.
- `Cancelled` — Creator cancelled while `Open`; 100% refunds enabled.
- `Disputed` — An authorized approver disputed an unreleased milestone; proportional refunds of unreleased funds enabled.

#### `Milestone` (`#[contracttype]` struct)
| Field | Rust Type | Soroban XDR Type | Description |
|---|---|---|---|
| `index` | `u32` | `U32` | Zero-based sequential position (`0..len`, max 10) |
| `amount` | `i128` | `I128` | Payout amount in the token's smallest unit (7 decimals for SAC) |
| `evidence_hash` | `Option<BytesN<32>>` | `Option<BytesN<32>>` | 32-byte digest of off-chain proof, `None` until submitted |
| `released` | `bool` | `Bool` | `true` once transferred to `creator` |

#### `Request` (`#[contracttype]` struct)
| Field | Rust Type | Soroban XDR Type | Description |
|---|---|---|---|
| `id` | `u64` | `U64` | Monotonic request identifier starting at `0` |
| `creator` | `Address` | `Address` | Account that opened the request and receives milestone payouts |
| `token` | `Address` | `Address` | Allowlisted SAC token contract address |
| `goal` | `i128` | `I128` | Target funding goal (`> 0`, equals sum of milestone amounts) |
| `raised` | `i128` | `I128` | Cumulative escrowed contributions |
| `total_released` | `i128` | `I128` | Cumulative amount paid out across released milestones |
| `deadline` | `u64` | `U64` | Funding deadline (ledger UNIX timestamp in seconds) |
| `status` | `Status` | `ScVal::Vec([Symbol])` | Lifecycle state |
| `content_hash` | `BytesN<32>` | `BytesN<32>` | 32-byte SHA-256 / CID digest of off-chain request metadata |
| `milestones` | `Vec<Milestone>` | `Vec<Milestone>` | 1..=10 milestones summing to `goal` |
| `created_at` | `u64` | `U64` | Creation ledger UNIX timestamp in seconds |

#### `Config` (`#[contracttype]` struct)
| Field | Rust Type | Soroban XDR Type | Description |
|---|---|---|---|
| `allowed_tokens` | `Vec<Address>` | `Vec<Address>` | Allowlisted SAC token contracts |
| `min_contribution` | `i128` | `I128` | Minimum single contribution amount |
| `approvers` | `Vec<Address>` | `Vec<Address>` | Accounts authorized to approve or dispute milestones |
| `approval_threshold` | `u32` | `U32` | Distinct approvals required before `release_milestone` |
| `reputation_contract` | `Option<Address>` | `Option<Address>` | Optional `reputation` contract notified via isolated `try_invoke_contract` |

### 2.2 Public Entrypoints (`#[contractimpl]`)

| Function | Parameters | Return | Auth | Description |
|---|---|---|---|---|
| `initialize` | `admin: Address, config: Config` | `Result<(), Error>` | `admin` | One-time initialization of admin, config, request counter (`0`), and storage schema version (`1`). |
| `set_config` | `config: Config` | `Result<(), Error>` | `Admin` | Update protocol config. Cannot move or access escrowed funds. |
| `create_request` | `creator: Address, token: Address, goal: i128, deadline: u64, milestones: Vec<Milestone>, content_hash: BytesN<32>` | `Result<u64, Error>` | `creator` | Create an `Open` request (auto-creates 1 full-goal milestone if `milestones` is empty) and emit `created`. |
| `contribute` | `request_id: u64, contributor: Address, amount: i128` | `Result<(), Error>` | `contributor` | Escrow `amount` of `request.token`. Auto-transitions to `Funded` if `raised >= goal`. Emits `contributed` (and `funded` if reached). |
| `finalize` | `request_id: u64` | `Result<(), Error>` | Permissionless | After `now >= deadline`, moves an `Open` request to `Funded` (`raised >= goal`) or `Failed` (`raised < goal`). |
| `cancel` | `request_id: u64` | `Result<(), Error>` | `request.creator` | Cancel an `Open` request before funding completes; emits `cancelled`. |
| `submit_milestone` | `request_id: u64, index: u32, evidence_hash: BytesN<32>` | `Result<(), Error>` | `request.creator` | Attach `evidence_hash` to sequential milestone `index`; emits `milestone_submitted`. |
| `approve_milestone` | `request_id: u64, index: u32, approver: Address` | `Result<(), Error>` | `approver` (in `config.approvers`) | Record distinct approval for submitted milestone `index`; emits `milestone_approved`. |
| `release_milestone` | `request_id: u64, index: u32` | `Result<(), Error>` | Permissionless | Transfer milestone `amount` to `creator` once `approvals >= approval_threshold`; transitions to `Releasing` or `Completed`; emits `milestone_released` and notifies `reputation`. |
| `dispute_milestone` | `request_id: u64, index: u32, approver: Address` | `Result<(), Error>` | `approver` (in `config.approvers`) | Mark unreleased milestone as disputed, moving request to `Disputed` and enabling proportional refunds. |
| `refund` | `request_id: u64, contributor: Address` | `Result<(), Error>` | `contributor` | Zero contributor balance and return full (`Failed` / `total_released == 0`) or proportional (`(amount * (raised - total_released)) / raised`) share; emits `refunded`. |
| `get_request` | `request_id: u64` | `Result<Request, Error>` | None | Read request state and extend TTL. |
| `get_contribution` | `request_id: u64, contributor: Address` | `i128` | None | Read contributor's remaining unrefunded contribution (`0` if none). |

*(Note: Rust library helpers `propose_wasm_upgrade`, `execute_wasm_upgrade`, and `migrate_version` are defined on `impl HelpRequest` using `shared::upgrade` for time-locked admin upgrades and versioned migrations.)*

### 2.3 Events (`help_request`)

All events use `#[contractevent]` with a single static symbol topic (`topics = ["<name>"]`) and a map/struct data payload:

| Topic (`topic[0]`) | SDK Decoded `type` | Payload Fields |
|---|---|---|
| `created` | `help_request.created` | `request_id: u64, creator: Address, token: Address, goal: i128, deadline: u64` |
| `contributed` | `help_request.contributed` | `request_id: u64, contributor: Address, amount: i128, total_raised: i128` |
| `funded` | `help_request.funded` | `request_id: u64, total_raised: i128` |
| `failed` | `help_request.failed` | `request_id: u64, total_raised: i128` |
| `milestone_submitted` | `help_request.milestone_submitted` | `request_id: u64, index: u32, evidence_hash: BytesN<32>` |
| `milestone_approved` | `help_request.milestone_approved` | `request_id: u64, index: u32, approver: Address` |
| `milestone_released` | `help_request.milestone_released` | `request_id: u64, index: u32, amount: i128` |
| `refunded` | `help_request.refunded` | `request_id: u64, contributor: Address, amount: i128` |
| `cancelled` | `help_request.cancelled` | `request_id: u64` |

### 2.4 Error Codes (`help_request::Error`, `#[repr(u32)]`)

| Code | Name | Meaning |
|---:|---|---|
| `1` | `NotInitialized` | Contract has not been initialized |
| `2` | `AlreadyInitialized` | `initialize` called more than once |
| `3` | `RequestNotFound` | Unknown `request_id` |
| `4` | `InvalidAmount` | Amount is `<= 0`, below `min_contribution`, or overflows `i128`/`u64` |
| `5` | `DeadlinePassed` | Action not allowed at or after `deadline` |
| `6` | `DeadlineNotReached` | `finalize` or time-lock execution attempted early |
| `7` | `NotOpen` | Request status does not permit this transition |
| `8` | `Unauthorized` | Caller is not authorized for the role |
| `9` | `TokenNotAllowed` | `token` is not in `config.allowed_tokens` |
| `10` | `NothingToRefund` | Contributor has `0` refundable balance or all funds were released |
| `11` | `MilestoneNotFound` | `index >= request.milestones.len()` |
| `12` | `MilestoneAlreadyReleased` | Milestone has already been paid out |
| `13` | `MilestoneNotSubmitted` | Milestone has no `evidence_hash` yet |
| `14` | `MilestoneAlreadySubmitted` | Milestone evidence was already submitted |
| `15` | `MilestoneAlreadyApproved` | `approver` already approved `(request_id, index)` |
| `16` | `InsufficientApprovals` | Approvals `< config.approval_threshold` |
| `17` | `InvalidMilestoneSchedule` | Count `> 10`, non-positive milestone amount, pre-released/submitted flag, or sum `!= goal` |
| `18` | `MilestoneOutOfOrder` | Non-sequential milestone index or prior milestone not yet released |
| `19` | `NotDisputedOrCancelled` | Reserved error variant for non-refundable dispute states |

---

## 3. `reputation` Contract (`Reputation`)

### 3.1 Public Types

#### `Badge` (`#[contracttype]` struct)
| Field | Rust Type | Description |
|---|---|---|
| `name` | `Symbol` | Badge label (`Helper`, `Advocate`, `Champion`) |
| `tier` | `u32` | Tier level (`1`, `2`, `3`) |
| `threshold` | `i64` | Score threshold required |

#### `Config` (`#[contracttype]` struct)
| Field | Rust Type | Default |
|---|---|---|
| `tier1_threshold` | `i64` | `10` |
| `tier2_threshold` | `i64` | `50` |
| `tier3_threshold` | `i64` | `100` |

### 3.2 Public Entrypoints (`#[contractimpl]`)

| Function | Parameters | Return | Auth | Description |
|---|---|---|---|---|
| `initialize` | `admin: Address` | `Result<(), Error>` | `admin` | Set admin and default badge thresholds (`10 / 50 / 100`). |
| `set_config` | `config: Config` | `Result<(), Error>` | `Admin` | Update badge score thresholds. |
| `get_config` | *(none)* | `Result<Config, Error>` | None | Read current badge thresholds. |
| `add_reporter` | `reporter: Address` | `Result<(), Error>` | `Admin` | Authorize a contract or account to record reputation events. |
| `remove_reporter` | `reporter: Address` | `Result<(), Error>` | `Admin` | Revoke reporter authorization. |
| `record_event` | `reporter: Address, subject: Address, _kind: Symbol, delta: i32, ref_id: String` | `Result<(), Error>` | `reporter` (registered) | Apply non-zero `delta` to `subject` score and emit `score_changed`. |
| `get_score` | `subject: Address` | `i64` | None | Return cumulative score (`0` if none). |
| `get_badges` | `subject: Address` | `Vec<Badge>` | None | Return earned badges based on current score and `Config`. |

### 3.3 Events (`reputation`)

| Topic (`topic[0]`) | SDK Decoded `type` | Payload Fields |
|---|---|---|
| `score_changed` | `reputation.score_changed` | `subject: Address, delta: i32, new_score: i64, ref_id: String` |

### 3.4 Error Codes (`reputation::Error`, `#[repr(u32)]`)

| Code | Name | Meaning |
|---:|---|---|
| `1` | `NotInitialized` | Contract has not been initialized |
| `2` | `AlreadyInitialized` | `initialize` called more than once |
| `3` | `Unauthorized` | Caller is not admin or registered reporter |
| `4` | `InvalidDelta` | `delta == 0` or `i64` score addition overflowed |

---

## 4. `attestation` Contract (`AttestationContract`)

### 4.1 Public Types

#### `Attestation` (`#[contracttype]` struct)
| Field | Rust Type | Description |
|---|---|---|
| `id` | `u64` | Monotonic attestation ID starting at `1` |
| `attestor` | `Address` | Registered attestor that issued the statement |
| `subject` | `Address` | Subject user or contract/request address |
| `kind` | `Symbol` | Category tag (e.g. `verified_need`, `identity`) |
| `content_hash` | `BytesN<32>` | 32-byte digest of off-chain statement/evidence |
| `expires_at` | `Option<u64>` | Optional expiration UNIX timestamp (seconds) |
| `revoked` | `bool` | `true` if revoked by `attestor` |
| `created_at` | `u64` | Creation ledger UNIX timestamp (seconds) |

### 4.2 Public Entrypoints (`#[contractimpl]`)

| Function | Parameters | Return | Auth | Description |
|---|---|---|---|---|
| `initialize` | `admin: Address` | `Result<(), Error>` | `admin` | One-time setup of admin and `NextId = 1`. |
| `add_attestor` | `attestor: Address, meta_hash: BytesN<32>` | `Result<(), Error>` | `Admin` | Approve `attestor` and record metadata digest. |
| `remove_attestor` | `attestor: Address` | `Result<(), Error>` | `Admin` | Remove `attestor` from active registry. |
| `attest` | `attestor: Address, subject: Address, kind: Symbol, content_hash: BytesN<32>, expires_at: Option<u64>` | `Result<u64, Error>` | `attestor` (registered) | Store new attestation (`expires_at > now` if present), emit `attested`, return `id`. |
| `revoke` | `attestor: Address, attestation_id: u64` | `Result<(), Error>` | `attestation.attestor` | Mark attestation `revoked = true` and emit `revoked`. |
| `get_attestations` | `subject: Address` | `Vec<Attestation>` | None | Return all non-revoked, non-expired (`expires_at > now`) attestations for `subject`. |

### 4.3 Events (`attestation`)

| Topic (`topic[0]`) | SDK Decoded `type` | Payload Fields |
|---|---|---|
| `attested` | `attestation.attested` | `attestation_id: u64, attestor: Address, subject: Address` |
| `revoked` | `attestation.revoked` | `attestation_id: u64, attestor: Address, subject: Address` |

### 4.4 Error Codes (`attestation::Error`, `#[repr(u32)]`)

| Code | Name | Meaning |
|---:|---|---|
| `1` | `NotInitialized` | Contract has not been initialized |
| `2` | `AlreadyInitialized` | `initialize` called more than once |
| `3` | `Unauthorized` | Caller is not admin or the issuing/registered attestor |
| `4` | `AttestationNotFound` | Unknown `attestation_id` |
| `5` | `AlreadyRevoked` | Attestation was already revoked |
| `6` | `InvalidExpiry` | `expires_at <= now` or ID counter overflow |
