# Threat Model & Security Architecture

This document provides a comprehensive threat analysis and security architecture review for the StellarAid smart contracts (`help_request`, `reputation`, and `attestation`).

---

## 1. System Overview & Critical Assets

StellarAid contracts govern mutual-aid escrow, milestone payouts, community reputation, and verified attestations on Stellar (Soroban).

### Critical Assets:
1. **Escrowed Funds:** SAC tokens (e.g. USDC, native XLM) locked in `help_request` awaiting milestone delivery or refund.
2. **Accounting Invariants:** The exact conservation of funds: $\sum \text{Contributions} = \sum \text{Released Milestones} + \sum \text{Refunds} + \text{Escrow Balance}$.
3. **Reputation Scores & Badges:** Tamper-proof, verifiable scores reflecting honest mutual aid outcomes.
4. **Attestation Registry:** Trust vouches and verified-need credentials from authorized organizations.
5. **Administrative Authority:** Configurations, allowlists, and approver sets.

---

## 2. Threat Analysis & Mitigations

| Threat | Attack Vector / Scenario | Mitigation in StellarAid Contracts |
|---|---|---|
| **Malicious Token Contract** | Attacker deploys a malicious custom token contract with reentrancy hooks or fake balances. | Strict **Token Allowlist** (`allowed_tokens` in `Config`). Only verified Stellar Asset Contracts (SAC) such as canonical USDC or native XLM are permitted. |
| **Escrow Run / Creator Abandonment** | Creator receives full funding upfront, abandons the cause, and disappears with donor funds. | **Milestone Schedule Escrow:** Funds are released incrementally only after independent approver confirmation. Max milestones bounded to 10. |
| **Fake Milestone Evidence** | Creator uploads fraudulent or empty progress reports. | **Approver Threshold & Content Hashes:** Cryptographic content hashes (IPFS CID / SHA-256) committed on-chain. Approvers review out-of-band; configurable approval threshold. |
| **Milestone Dispute & Freeze** | Milestone evidence is contested or rejected. | **Dispute Path & Proportional Refunds:** Approvers can dispute milestones, enabling contributors to claim exact proportional refunds of unreleased funds. |
| **Admin Key Compromise / Rug Pull** | Compromised admin account attempts to drain escrowed funds. | **No Admin Custody Principle:** Admin can modify configuration (allowed tokens, minimum contributions, approver set) but **has zero fund withdrawal or transfer capabilities**. Admin on mainnet is enforced as a multisig account. |
| **Dust Griefing & Sybil Attacks** | Attacker floods contract with micro-contributions (1 stroop) to exhaust storage and ledger space. | **Minimum Contribution Enforced:** `min_contribution` enforced on every `contribute` call. |
| **Storage Exhaustion & Rent Expiration** | Contract entries expire from ledger storage or bloat unbounded collections. | **Isolated Storage Keys & TTL Extension:** Entries use discrete keys `(request_id, contributor)`; no single unbounded `Vec` in storage. Every read/write extends instance and persistent TTL. |
| **Reentrancy via Token Callbacks** | Reentrancy into `refund` or `release_milestone` during external token transfer calls. | **Checks-Effects-Interactions:** State is zeroed or marked `released = true` BEFORE initiating external `token::Client::transfer`. |
| **Cross-Contract Fund Trapping** | Malfunctioning, malicious, or reverted `reputation` contract traps funds in `help_request`. | **Cross-Contract Failure Isolation:** Calls to `reputation_contract` use `env.try_invoke_contract`. Any failure, out-of-gas, or revert in reputation is safely swallowed, ensuring token transfers and state transitions always succeed. |
| **Rounding Leakage in Refunds** | Rounding errors during integer division create or destroy value during proportional refunds. | **Floor Division Conservation Invariant:** Refund math uses integer multiplication followed by division: `(amount * remaining_unreleased) / raised`. Tested to assert zero value created or lost. |
| **Attestation Spoofing & Expiry Bypass** | Attacker fabricates an attestation or presents an expired statement as valid. | **Attestor Registry & On-Chain Expiry:** Admin manages authorized attestors with identity metadata hashes. `get_attestations` checks current ledger timestamp against `expires_at` and verifies `!revoked`. |
| **Reputation Sybil Farming** | User spins up dummy requests to farm reputation scores. | **Reporter Authorization:** Only authorized smart contracts (like `help_request`) can invoke `record_event`. Unverified external callers are rejected with `Unauthorized`. |

---

## 3. Trust Assumptions & Operational Security

1. **Multisig Governance:** On mainnet, the contract admin must be a multi-signature Stellar account or a time-locked DAO governor.
2. **Deterministic Finality:** Stellar consensus offers deterministic transaction finality (~5 seconds), eliminating the threat of blockchain reorganizations or double-spend forks.
3. **SAC Compliance:** Escrow tokens are assumed to follow the official Stellar Asset Contract specifications.
4. **Out-of-band Verification:** Attestor and approver identities are verified prior to on-chain authorization.
