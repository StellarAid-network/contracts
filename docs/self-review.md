# StellarAid Smart Contracts — Security Self-Review

This document records the comprehensive security review conducted on all state-changing and read entrypoints across `help_request`, `reputation`, and `attestation` against the principles in the repository `README.md` (`Security model`, `Storage and cost considerations`, and `Upgrades and versioning`).

---

## 1. Review Checklist & Principles Verified

Every public entrypoint across all three contracts was audited against five core invariants:

1. **Authorization (`require_auth`):** Every state-changing function verifies caller identity via `Address::require_auth()` or verifies membership in configured role sets (`Admin`, `approvers`, `Reporter`, `Attestor`).
2. **Checks-Effects-Interactions (CEI):** Every function that invokes an external contract (`token::Client::transfer` or cross-contract `reputation` notifications) validates preconditions and commits all storage state mutations **before** external calls.
3. **Checked Arithmetic:** All balance additions, subtractions, proportional refund multiplications, and counter increments use checked arithmetic (`checked_add`, `checked_sub`, `checked_mul`, `CheckedAmount`) with explicit error returns.
4. **Bounded Storage:** Contributor balances use isolated `DataKey::Contribution(request_id, contributor)` persistent keys rather than an unbounded `Vec`. Milestone schedules are capped at `MAX_MILESTONES = 10`.
5. **TTL Extension:** Both instance storage (`Admin`, `Config`) and persistent storage entries (`Request`, `Contribution`, `MilestoneApprover`, `Score`, `Attestation`) extend their TTL on active read and write paths.

---

## 2. Entrypoint Audit Matrix

| Contract | Function | Auth Check | CEI Ordering | Checked Math | Bounded Storage | TTL Extension |
|---|---|---|---|---|---|---|
| `help_request` | `initialize` | `admin.require_auth()` | N/A (no external calls) | N/A | Instance keys | Instance extended |
| `help_request` | `set_config` | `admin.require_auth()` | N/A (no external calls) | N/A | Bounded config vectors | Instance extended |
| `help_request` | `create_request` | `creator.require_auth()` | N/A (no external calls) | `checked_add_amount`, `checked_add(1)` | `MAX_MILESTONES = 10` | Instance + Request extended |
| `help_request` | `contribute` | `contributor.require_auth()` | State saved before `token.transfer` | `checked_add_amount` | Isolated `(id, addr)` key | Instance + Request + Contribution extended |
| `help_request` | `finalize` | Permissionless after deadline | N/A (no external calls) | N/A | Existing Request key | Instance + Request extended |
| `help_request` | `cancel` | `request.creator.require_auth()` | N/A (no external calls) | N/A | Existing Request key | Instance + Request extended |
| `help_request` | `submit_milestone` | `request.creator.require_auth()` | N/A (no external calls) | Bounded loop `0..index` | Bounded milestone vec | Instance + Request extended |
| `help_request` | `approve_milestone` | `approver.require_auth()` + allowlist | N/A (no external calls) | `count.checked_add(1)` | Isolated `(id, idx, addr)` key | Instance + Request + Approver + Count extended |
| `help_request` | `release_milestone` | Permissionless after threshold met | `released = true` & `total_released` saved before `token.transfer` and `try_invoke_contract` | `checked_add_amount` | Bounded milestone vec | Instance + Request extended |
| `help_request` | `dispute_milestone` | `approver.require_auth()` + allowlist | N/A (no external calls) | N/A | Existing Request key | Instance + Request extended |
| `help_request` | `refund` | `contributor.require_auth()` | Contribution zeroed (`0i128`) before `token.transfer` and `try_invoke_contract` | `checked_sub`, `checked_mul` | Isolated `(id, addr)` key | Instance + Request + Contribution extended |
| `reputation` | `initialize` / `set_config` | `admin.require_auth()` | N/A | N/A | Instance keys | Instance extended |
| `reputation` | `add_reporter` / `remove_reporter` | `admin.require_auth()` | N/A | N/A | Isolated `Reporter(addr)` key | Instance + Reporter extended |
| `reputation` | `record_event` | `reporter.require_auth()` + registry check | N/A | `checked_add`, `delta != 0` | Isolated `Score(subject)` key | Instance + Reporter + Score extended |
| `attestation` | `initialize` / `add_attestor` / `remove_attestor` | `admin.require_auth()` | N/A | N/A | Isolated `Attestor(addr)` key | Instance + Attestor extended |
| `attestation` | `attest` / `revoke` | `attestor.require_auth()` + registry/issuer check | N/A | `next_id.checked_add(1)` | Isolated `Attestation(id)` key | Instance + Attestor + Attestation extended |

---

## 3. Findings & Resolutions (All Closed & Verified — Day 6 Final Sign-off)

| ID | Severity | Status | Location | Finding Description | Resolution & Verification |
|---|---|---|---|---|---|
| **SR-01** | Low | **CLOSED** | `help_request::refund` | `request.raised - request.total_released` used unchecked subtraction instead of `checked_sub`, and did not extend the `Request` persistent entry TTL during refund waves. | Replaced with `request.raised.checked_sub(request.total_released).ok_or(Error::InvalidAmount)?` and added `extend_ttl` on `DataKey::Request(request_id)`. Verified in `prop_proportional_refunds_conserve_funds_across_random_contributors`. |
| **SR-02** | Low | **CLOSED** | `help_request::submit_milestone`, `approve_milestone`, `release_milestone`, `dispute_milestone` | Instance storage TTL was not extended during milestone phase operations (`submit`, `approve`, `release`, `dispute`), which could allow instance config TTL to lag during long milestone delivery schedules. | Added `storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO)` to all four milestone functions and extended `Request` TTL in `approve_milestone`. Verified across milestone lifecycle tests. |
| **SR-03** | Low | **CLOSED** | `help_request::create_request`, `approve_milestone` & `attestation::attest` | Counter increments (`id + 1`, `count + 1`, `next_id + 1`) used standard `+` operator instead of `checked_add(1)`. | Replaced all counter increments with `.checked_add(1)` returning a typed contract error on overflow. Verified in crate unit test suites. |
| **SR-04** | Low | **CLOSED** | `reputation::record_event` | `Error::InvalidDelta` (code 4) was defined in `reputation/src/errors.rs` but `record_event` accepted `delta == 0` and used `saturating_add` instead of `checked_add`. | Added explicit `if delta == 0 { return Err(Error::InvalidDelta); }` validation, replaced `saturating_add` with `checked_add(delta as i64).ok_or(Error::InvalidDelta)?`, and extended TTLs on `Instance`, `Reporter`, and `Score`. Verified in `reputation::test::record_event_zero_delta_fails`. |
| **SR-05** | Informational | **VERIFIED** | `help_request` Admin Custody Check | Verified that `Admin` only controls `Config` (`allowed_tokens`, `min_contribution`, `approvers`, `approval_threshold`, `reputation_contract`) and cannot withdraw, redirect, or freeze escrowed tokens. | Confirmed by code inspection, `docs/audit-prep.md` Invariant 2, and `admin_cannot_withdraw_funds`. |

---

## 4. Day 6 Final Verification Summary

- **Workspace Tests:** All unit, integration, upgrade/migration, and randomized conservation property tests pass (`cargo test --workspace`).
- **Static Analysis & Documentation:** `cargo clippy --workspace --all-targets -- -D warnings` and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` pass with zero warnings.
- **Dependency Hygiene:** `deny.toml` configured for advisories, licenses, bans, and crates.io source verification in `audit.yml`.
- **Open High/Medium/Low Findings:** `0` open findings. Ready for `v0.1.0-testnet` tag and external audit handoff (`docs/audit-prep.md`).
