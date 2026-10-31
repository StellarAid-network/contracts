# Contributor Issue Backlog — `contracts`

Ready-to-claim issues for external contributors. Comment on the corresponding GitHub Issue before starting work, and follow [`docs/CONTRIBUTING-contracts.md`](./CONTRIBUTING-contracts.md).

---

### 1. `[good first issue]` Add boundary timestamp unit tests for `finalize` at `deadline - 1`, `deadline`, and `deadline + 1`
- **Labels:** `good first issue`, `testing`, `help_request`
- **Scope:** `contracts/help_request/src/test.rs`
- **Acceptance Criteria:**
  - Explicit test cases asserting `finalize` returns `DeadlineNotReached` at `deadline - 1` and `deadline`, and succeeds at `deadline + 1` (or exact contract boundary documented in `ABI.md`).
  - `cargo test --workspace` passes with zero warnings.

### 2. `[good first issue]` Add unit tests for `attestation` expiry boundary (`expires_at == ledger.timestamp()`)
- **Labels:** `good first issue`, `testing`, `attestation`
- **Scope:** `contracts/attestation/src/test.rs`
- **Acceptance Criteria:**
  - Verify `get_attestations(subject)` filters out expired attestations at the exact expiry second while retaining `expires_at: None` and future timestamps.
  - Verify revoked attestations remain excluded even when `expires_at` is in the future.

### 3. `[good first issue]` Add rustdoc usage examples to `shared` crate helpers
- **Labels:** `good first issue`, `documentation`
- **Scope:** `shared/src/lib.rs`
- **Acceptance Criteria:**
  - Every public constant and helper in `shared` includes clear rustdoc explanations and storage TTL guidance.
  - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` passes cleanly.

### 4. `[good first issue]` Add shellcheck validation step to `ci.yml` for `scripts/*.sh`
- **Labels:** `good first issue`, `ci`
- **Scope:** `.github/workflows/ci.yml`, `scripts/*.sh`
- **Acceptance Criteria:**
  - Run `shellcheck scripts/*.sh` in CI and resolve any quoting/lint warnings without changing CLI flags.

### 5. `[help wanted]` Automated CPU & memory regression benchmark script (`scripts/bench-costs.sh`)
- **Labels:** `help wanted`, `performance`
- **Scope:** `scripts/bench-costs.sh`, `docs/costs.md`
- **Acceptance Criteria:**
  - Script parses Soroban `env.budget()` output from a dedicated benchmark test and fails if CPU instructions or memory bytes regress by >15% against `docs/costs.md`.

### 6. `[help wanted]` Randomized multi-milestone proportional refund property tests (50+ contributors)
- **Labels:** `help wanted`, `testing`, `security`
- **Scope:** `contracts/help_request/src/test.rs`
- **Acceptance Criteria:**
  - Property-style test simulating partial milestone release (e.g. Milestone 0 released, Milestone 1 disputed) across 50 contributors with prime stroop amounts.
  - Asserts zero over-withdrawal and bounds rounding dust to `< contributor_count` stroops.

### 7. `[help wanted]` Add multi-token allowlist rotation tests in `help_request`
- **Labels:** `help wanted`, `testing`, `help_request`
- **Scope:** `contracts/help_request/src/test.rs`
- **Acceptance Criteria:**
  - Verify that removing a token via `set_config` prevents *new* requests in that token while allowing existing open/failed/funded requests denominated in that token to complete milestones and `refund` normally.

### 8. `[help wanted]` Fuzz target scaffolding with `cargo-fuzz` for `help_request` state transitions
- **Labels:** `help wanted`, `security`, `testing`
- **Scope:** `fuzz/` (excluded from default WASM build)
- **Acceptance Criteria:**
  - Fuzz harness generating arbitrary sequences of `contribute`, `cancel`, `finalize`, `submit_milestone`, `approve_milestone`, `dispute_milestone`, and `refund` calls and asserting fund conservation after every step.

### 9. `[good first issue]` Add badge revocation / negative delta floor tests in `reputation`
- **Labels:** `good first issue`, `testing`, `reputation`
- **Scope:** `contracts/reputation/src/test.rs`
- **Acceptance Criteria:**
  - Test score decreases (`delta < 0`) across badge thresholds (`10`, `50`, `100`) and document whether earned badges are sticky or threshold-computed in `docs/ABI.md`.

### 10. `[help wanted]` Time-locked WASM upgrade design RFC (`post-85` preparation)
- **Labels:** `help wanted`, `rfc`, `post-85`
- **Scope:** Draft RFC for mainnet admin multisig + delay-gated `update_current_contract_wasm`
- **Acceptance Criteria:**
  - Threat analysis of emergency pause vs. immutable escrow guarantees, storage-version migration pattern, and proposed entrypoint specification.
