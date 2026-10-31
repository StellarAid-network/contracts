# Contributing to `stellar-aid/contracts`

Thank you for helping secure and improve the Soroban smart contracts that power StellarAid! Because these contracts hold community funds in escrow and enforce public reputation/attestation rules, we hold every change to strict safety and review standards.

---

## 1. Golden Rules

1. **No admin custody:** Admin functions may update configuration (`allowed_tokens`, `approvers`, `min_contribution`, badge thresholds, attestor registry), and **must never** introduce any code path that withdraws, redirects, or freezes escrowed contributor funds.
2. **Checks-Effects-Interactions:** Always validate inputs and persist updated storage state *before* invoking external token transfers (`token::Client::transfer`) or cross-contract calls.
3. **Checked arithmetic everywhere:** Use `checked_add`, `checked_sub`, `checked_mul`, `checked_div` and return `Error::InvalidAmount` on overflow/underflow. Never rely on wrapping or panicking arithmetic for user-supplied values.
4. **Explicit authorization:** Every state-mutating entrypoint must call `require_auth()` on the acting `Address` at the top of the function.
5. **Bounded storage & loops:** Never iterate over an unbounded contributor list. Store per-contributor balances under `(request_id, contributor)` persistent keys and bump TTLs (`extend_ttl`) on active reads/writes.

---

## 2. How to Add or Modify a Contract Function Safely

1. **Check whether an RFC is required:**
   - Any change to a public function signature, `#[contracttype]` struct/enum layout, `#[contracterror]` variant, or emitted event topic/payload is an **ABI change** and **requires an RFC** in [`StellarAid-network/.github/rfcs/`](https://github.com/StellarAid-network/.github/tree/main/rfcs) before implementation.
   - Never renumber existing `#[contracterror]` codes—append new variants at the end only.
2. **Write tests first (`src/test.rs`):**
   - Success path (verifying storage state, token balances, and emitted events).
   - Every reachable `Error` variant.
   - Unauthorized caller rejection (`mock_all_auths` omitted or verified via `auths()`).
   - Ledger timestamp boundary conditions (`deadline - 1`, `deadline`, `deadline + 1`).
   - Fund-conservation invariant (`escrow_balance == total_contributed - total_released - total_refunded`).
3. **Document rustdoc & `docs/ABI.md`:**
   - Add `///` rustdoc comments documenting parameters, authorization requirements, error codes, and events.
   - Note: Soroban embeds rustdoc strings into `contractspecv0` WASM custom sections, which changes the compiled WASM SHA-256 hash. Whenever `.rs` files change, regenerate bindings (`./scripts/bindings.sh`) and coordinate a matching `@stellar-aid/sdk` PR.

---

## 3. Pre-PR Verification Checklist

Run every check locally before opening your pull request:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo test --workspace
make optimize
```

Verify that optimized `.wasm` sizes remain comfortably under Soroban's 64 KiB ceiling (see [`docs/costs.md`](./costs.md)).
