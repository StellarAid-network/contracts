# Threat model

This document is expanded during the security-review work. It starts from the threat
table in the repository README.

## Assets

- Escrowed funds held by the Help Request contract.
- Reputation scores and badges.
- Attestations issued by trusted parties.
- Admin and approver authority.

## Threats and mitigations

| Threat | Mitigation |
|---|---|
| Malicious token contract | Allowlist of tokens |
| Creator withdraws and disappears | Escrow plus milestone approvals |
| Fake milestone evidence | Approver set, attestations, dispute path |
| Admin key compromise | Multisig admin, no fund-withdrawal path |
| Griefing with dust contributions | Minimum contribution amount |
| Storage exhaustion | Per-contribution rent paid through fees, bounded structures |
| Reentrancy via token callbacks | State updated before transfers |

## Trust assumptions

- The admin is a multisig account on mainnet, never a single hot key.
- The token allowlist contains only reviewed Stellar Asset Contracts.
- Approvers and attestors are reviewed out of band before being added.
- Stellar has deterministic finality; there are no chain reorganizations to unwind.
