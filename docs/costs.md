# StellarAid Smart Contracts — WASM Sizes & Resource Cost Profile

This document records the optimized WASM binary sizes, SHA-256 hashes, storage footprints, and simulated Soroban execution budgets for the StellarAid contracts (`help_request`, `reputation`, and `attestation`).

---

## 1. Optimized WASM Binary Sizes

Built via `make optimize` (`stellar contract build` with spec shaking v2 and `wasm32v1-none` release optimizations):

| Contract | Crate | Unoptimized Size | Optimized Size | Reduction | Budget Limit (64 KiB) | SHA-256 Hash (`wasmHash`) |
|---|---|---|---|---|---|---|
| **Help Request** | `contracts/help_request` | 28,359 bytes | **25,183 bytes** (24.6 KiB) | -11.2% | 38.4% of limit | `43e6a0ae6e934788787654d0ef7460acc68c261ad85d6918cc898c43d2e2768f` |
| **Reputation** | `contracts/reputation` | 6,939 bytes | **6,076 bytes** (5.9 KiB) | -12.4% | 9.3% of limit | `610058f5840c3bbd4f9bb56dddf8a62ed3f6959d02fed7683a0d28cd0014524b` |
| **Attestation** | `contracts/attestation` | 8,730 bytes | **7,624 bytes** (7.4 KiB) | -12.7% | 11.6% of limit | `bb2114eadc35f4045e4d6a2d2f85163a7791496c019448a1e5687b0e3056694b` |

---

## 2. Simulated Execution Resource Costs

Measured via Soroban host test environment (`soroban-env-host 29.0.1`, protocol 29) across representative lifecycle operations:

| Contract & Operation | CPU Instructions (approx.) | Memory Bytes (approx.) | Ledger Read Entries | Ledger Write Entries | Notes |
|---|---|---|---|---|---|
| `help_request::initialize` | ~310,000 | ~64 KiB | 1 (instance) | 1 (instance) | One-time setup |
| `help_request::create_request` (3 milestones) | ~520,000 | ~96 KiB | 1 (instance) | 2 (instance + `Request(id)`) | Bounded to max 10 milestones |
| `help_request::contribute` | ~1,180,000 | ~160 KiB | 4 (instance, `Request`, `Contribution`, SAC balances) | 4 (`Request`, `Contribution`, 2 SAC balances) | O(1) per contributor key |
| `help_request::finalize` | ~420,000 | ~80 KiB | 2 (instance, `Request`) | 1 (`Request`) | Permissionless after deadline |
| `help_request::submit_milestone` | ~460,000 | ~88 KiB | 2 (instance, `Request`) | 1 (`Request`) | Sequential check bounded by `index < 10` |
| `help_request::approve_milestone` | ~510,000 | ~92 KiB | 4 (instance, `Request`, `MilestoneApprover`, `MilestoneApprovalCount`) | 2 (`MilestoneApprover`, `MilestoneApprovalCount`) | Isolated per-approver key |
| `help_request::release_milestone` (+ reputation) | ~1,650,000 | ~220 KiB | 5 (instance, `Request`, `MilestoneApprovalCount`, SAC, `Reputation`) | 4 (`Request`, 2 SAC balances, `Reputation::Score`) | Includes isolated `try_invoke_contract` |
| `help_request::refund` (proportional) | ~1,240,000 | ~172 KiB | 4 (instance, `Request`, `Contribution`, SAC) | 3 (`Contribution`, 2 SAC balances) | Zeroes contributor balance before transfer |
| `reputation::record_event` | ~380,000 | ~72 KiB | 3 (instance, `Reporter`, `Score`) | 1 (`Score`) | Emits `score_changed` |
| `attestation::attest` | ~490,000 | ~88 KiB | 3 (instance, `Attestor`, `SubjectAttestations`) | 3 (instance, `Attestation(id)`, `SubjectAttestations`) | Emits `attested` |

---

## 3. Storage Footprint & Rent Optimization Design

- **Instance Storage (`Admin`, `Config`, `NextRequestId`, `UpgradeKey`):** Shared across all calls to a contract instance; extended to 30 days (`518,400` ledgers) whenever remaining TTL drops below 1 day (`17,280` ledgers).
- **Persistent Storage (`Request(id)`, `Contribution(id, addr)`, `MilestoneApprover`, `Score(addr)`, `Attestation(id)`):** Isolated keys ensure each transaction only loads the ledger entries it touches, avoiding `O(N)` deserialization of contributor lists. Extended to 60 days (`1,036,800` ledgers) when remaining TTL drops below 7 days (`120,960` ledgers).
- **Hash-Only Content References:** Descriptions, evidence attachments, and identity metadata are stored off-chain (IPFS) and committed on-chain strictly as 32-byte `BytesN<32>` digests.
