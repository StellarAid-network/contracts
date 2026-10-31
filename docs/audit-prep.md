# StellarAid Smart Contracts — External Audit Preparation Package (`v0.1.0-testnet`)

This document is the onboarding and specification package prepared for independent smart-contract security auditors reviewing the StellarAid Soroban contracts prior to mainnet launch.

---

## 1. Audit Scope

**Repository:** `https://github.com/StellarAid-network/contracts`
**Target Tag:** `v0.1.0-testnet`
**Soroban SDK:** `29.0.1` (`wasm32v1-none`, Rust `1.82+` pinned in `rust-toolchain.toml`)

| Crate | Path | Approximate Source LOC (excl. tests) | Responsibility |
|---|---|---:|---|
| `shared` | `shared/src/{lib.rs,upgrade.rs}` | ~215 | Domain aliases, `CheckedAmount` trait, versioned storage schema, and time-locked WASM upgrade helpers |
| `help_request` | `contracts/help_request/src/{lib.rs,types.rs,errors.rs,events.rs}` | ~1,145 | Non-custodial funding-goal escrow, sequential milestone payouts, disputes, and proportional refunds |
| `reputation` | `contracts/reputation/src/{lib.rs,types.rs,errors.rs,events.rs}` | ~295 | Authorized reporter registry, cumulative `i64` score ledger, and threshold badge computation |
| `attestation` | `contracts/attestation/src/{lib.rs,types.rs,errors.rs,events.rs}` | ~345 | Authorized attestor registry, hash-anchored statements with optional expiry, and revocation |

**Out of Scope:**
- Off-chain services (`api` Fastify backend/indexer and `web` Next.js application).
- Native Stellar operations (claimable-balance giveaways and direct SAC payments built in `sdk` without custom Soroban contracts).
- Generated TypeScript client bindings in `sdk/src/generated/`.

---

## 2. Trust Assumptions & Roles

1. **Contract `Admin` (Multisig on Mainnet):**
   - Controls protocol configuration (`allowed_tokens`, `min_contribution`, `approvers`, `approval_threshold`, `reputation_contract` in `help_request`; badge thresholds and reporters in `reputation`; attestor registry in `attestation`).
   - Controls time-locked WASM upgrades (`DEFAULT_TIMELOCK_DELAY = 86_400` seconds).
   - **Strict Non-Custody:** Even if compromised, `Admin` has **zero** entrypoints capable of transferring, redirecting, or confiscating escrowed tokens. *(Note: On mainnet, WASM upgrade authority must be held by an N-of-M multisig with public time-lock monitoring so users can exit before malicious code could ever execute.)*
2. **Allowlisted Tokens (`allowed_tokens`):**
   - Assumed to be canonical Stellar Asset Contracts (SAC) such as native XLM and Circle USDC, obeying standard `soroban_sdk::token::Client` transfer semantics without fee-on-transfer or rebasing behavior.
3. **Milestone `Approvers`:**
   - Trusted community verifiers or multisig signers configured in `Config.approvers`. At least `approval_threshold` distinct approvers must approve a submitted milestone before payout, and any single approver can halt payouts via `dispute_milestone`.
4. **Reputation `Reporters` & `Attestors`:**
   - `Reporters` (such as `help_request`) can adjust `reputation` scores; `Attestors` can issue/revoke statements in `attestation`. Neither role has any authority over escrowed funds in `help_request`.

---

## 3. Core Security Invariants

Auditors are asked to verify that the following invariants hold across **all** possible transaction sequences and ledger timestamps:

### Invariant 1 — Exact Conservation of Escrowed Funds
For every `request_id` using an allowlisted SAC token:
$$\text{Contract Token Balance Attributable to } R = R.\text{raised} - R.\text{total\_released} - \sum_{c} \text{Refunded}(R, c)$$
- No sequence of `contribute`, `release_milestone`, `cancel`, `finalize`, `dispute_milestone`, or `refund` calls can ever transfer out more tokens than were contributed to `request_id`.
- Floor division in proportional refunds (`(amount * remaining_unreleased) / raised`) guarantees $\sum \text{Refunded} \le R.\text{raised} - R.\text{total\_released}$ (zero value creation).

### Invariant 2 — Non-Custodial Admin Boundary
- Calling `set_config` cannot alter an existing `Request`'s `token`, `creator`, `goal`, `raised`, `total_released`, or `milestones` schedule.
- No admin-only function invokes `token::Client::transfer`.

### Invariant 3 — Single Refund & Checks-Effects-Interactions (CEI)
- Every external call (`token::Client::transfer` and `env.try_invoke_contract`) occurs **strictly after** persistent storage mutations (`request.raised`, `milestone.released`, `request.total_released`, `Contribution(id, addr) = 0`) are written.
- Once `refund(request_id, contributor)` succeeds, `DataKey::Contribution(request_id, contributor)` is `0`, preventing double-refunds.

### Invariant 4 — Sequential & Bounded Milestone Execution
- At `create_request`, `1 <= milestones.len() <= 10`, indices are strictly `0..len`, every `milestone.amount > 0`, and $\sum \text{milestone.amount} == \text{goal}$.
- Milestone `k` cannot be submitted or released unless all milestones `0..k` have `released == true`.
- Milestone `k` cannot be released unless `MilestoneApprovalCount(request_id, k) >= config.approval_threshold` and `request.status` is `Funded` or `Releasing`.

### Invariant 5 — Cross-Contract Failure Isolation
- `help_request` invokes `reputation::record_event` exclusively via `env.try_invoke_contract::<Val, InvokeError>` and discards any `Err`.
- A paused, reverted, uninitialized, or malicious `reputation_contract` can **never** revert or block `release_milestone` or `refund`.

---

## 4. Deterministic Build & Test Reproduction

### Prerequisites
- Rust toolchain matching `rust-toolchain.toml` (`stable` with `wasm32v1-none`, `rustfmt`, `clippy`)
- Stellar CLI (`stellar 22+` / Soroban protocol 29)

### Step-by-Step Commands
```bash
# 1. Clone and checkout the audited tag
git clone https://github.com/StellarAid-network/contracts.git
cd contracts
git checkout v0.1.0-testnet

# 2. Verify formatting and lints
make lint
cargo fmt --all -- --check

# 3. Run full unit, lifecycle, and randomized property tests
make test

# 4. Build release WASM binaries and verify SHA-256 digests
make optimize
sha256sum target/wasm32v1-none/release/{help_request,reputation,attestation}.wasm

# 5. Generate rustdoc with warnings denied
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Compare the output of `sha256sum` against `deployments/testnet.json` and `docs/costs.md`.

---

## 5. Known Limitations & Design Trade-offs

1. **Proportional Refund Rounding Dust (`help_request::refund`):**
   - When a request enters `Disputed` after partial milestone payouts, integer floor division `(contribution * remaining_unreleased) / raised` may leave up to `1` stroop per contributor (`< N` stroops total, i.e., `< 0.0000001` USDC per contributor) unclaimable inside the contract. This intentionally errs on the side of solvency so the last contributor to refund never encounters an insufficient contract balance.
2. **Config Changes Affect In-Flight Milestone Thresholds:**
   - `Config.approvers` and `Config.approval_threshold` are global instance settings rather than snapshotted per request. This allows the admin multisig to rotate a lost/compromised approver key so in-flight `Funded` requests are not permanently bricked.
3. **Per-Subject Attestation Index Vector (`attestation`):**
   - `DataKey::SubjectAttestations(subject)` stores a `Vec<u64>` of attestation IDs issued to `subject`. Because only admin-vetted `Attestor` organizations can call `attest`, untrusted users cannot bloat this vector; pagination/archival is tracked in `docs/backlog.md` for post-MVP scale.
4. **Upgrade Entrypoints Exposed via Shared Helpers:**
   - `propose_wasm_upgrade`, `execute_wasm_upgrade`, and `migrate_version` are implemented on `HelpRequest` (Rust `impl`) backed by `shared::upgrade`. Exposing them as `#[contractimpl]` Soroban entrypoints across all three contracts is intentionally deferred until mainnet multisig governance parameters are finalized (tracked under `post-85`).

---

## 6. Reference Documentation

- [`README.md`](../README.md) — Architecture and operational guide
- [`docs/ABI.md`](./ABI.md) — Complete function, event, and numeric error specification
- [`docs/threat-model.md`](./threat-model.md) — Threat matrix and mitigations
- [`docs/self-review.md`](./self-review.md) — Internal Day-5/Day-6 security audit log
- [`docs/costs.md`](./costs.md) — WASM binary sizes and Soroban CPU/memory resource measurements
