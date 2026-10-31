//! Event definitions and helpers for the Help Request contract.
//!
//! Event shapes are public API consumed by the `api` indexer (`@stellar-aid/sdk/events`);
//! treat any topic or payload field change as a breaking ABI change.

use soroban_sdk::{contractevent, Address, BytesN, Env};

/// Emitted when a request is created: `request_id, creator, token, goal, deadline`.
#[contractevent(topics = ["created"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Created {
    /// Monotonic request identifier assigned at creation.
    pub request_id: u64,
    /// Account that opened the help request.
    pub creator: Address,
    /// Escrowed SAC token contract address.
    pub token: Address,
    /// Target funding goal in the token's smallest unit.
    pub goal: i128,
    /// Funding deadline as a ledger UNIX timestamp in seconds.
    pub deadline: u64,
}

/// Emitted on each contribution: `request_id, contributor, amount, total_raised`.
#[contractevent(topics = ["contributed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Contributed {
    /// Target help request identifier.
    pub request_id: u64,
    /// Account transferring tokens into escrow.
    pub contributor: Address,
    /// Amount contributed in this call.
    pub amount: i128,
    /// Updated cumulative amount raised for the request.
    pub total_raised: i128,
}

/// Emitted when the goal is reached: `request_id, total_raised`.
#[contractevent(topics = ["funded"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Funded {
    /// Help request identifier that reached its funding goal.
    pub request_id: u64,
    /// Total escrowed amount when funded.
    pub total_raised: i128,
}

/// Emitted when the deadline passes without the goal: `request_id, total_raised`.
#[contractevent(topics = ["failed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Failed {
    /// Help request identifier that missed its deadline.
    pub request_id: u64,
    /// Total amount raised prior to expiration.
    pub total_raised: i128,
}

/// Emitted when a creator submits milestone evidence: `request_id, index, evidence_hash`.
#[contractevent(topics = ["milestone_submitted"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneSubmitted {
    /// Help request identifier.
    pub request_id: u64,
    /// Zero-based sequential milestone index.
    pub index: u32,
    /// 32-byte digest of off-chain delivery proof.
    pub evidence_hash: BytesN<32>,
}

/// Emitted when an approver approves a milestone: `request_id, index, approver`.
#[contractevent(topics = ["milestone_approved"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneApproved {
    /// Help request identifier.
    pub request_id: u64,
    /// Zero-based sequential milestone index.
    pub index: u32,
    /// Authorized approver address that recorded approval.
    pub approver: Address,
}

/// Emitted when a milestone amount is paid out: `request_id, index, amount`.
#[contractevent(topics = ["milestone_released"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneReleased {
    /// Help request identifier.
    pub request_id: u64,
    /// Zero-based sequential milestone index.
    pub index: u32,
    /// Token amount transferred to the request creator.
    pub amount: i128,
}

/// Emitted when a contributor is refunded: `request_id, contributor, amount`.
#[contractevent(topics = ["refunded"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Refunded {
    /// Help request identifier.
    pub request_id: u64,
    /// Contributor receiving the refund.
    pub contributor: Address,
    /// Token amount returned to the contributor.
    pub amount: i128,
}

/// Emitted when a request is cancelled: `request_id`.
#[contractevent(topics = ["cancelled"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cancelled {
    /// Help request identifier cancelled by its creator.
    pub request_id: u64,
}

/// Publish a `created` event.
pub fn created(
    env: &Env,
    request_id: u64,
    creator: Address,
    token: Address,
    goal: i128,
    deadline: u64,
) {
    Created {
        request_id,
        creator,
        token,
        goal,
        deadline,
    }
    .publish(env);
}

/// Publish a `contributed` event.
pub fn contributed(
    env: &Env,
    request_id: u64,
    contributor: Address,
    amount: i128,
    total_raised: i128,
) {
    Contributed {
        request_id,
        contributor,
        amount,
        total_raised,
    }
    .publish(env);
}

/// Publish a `funded` event.
pub fn funded(env: &Env, request_id: u64, total_raised: i128) {
    Funded {
        request_id,
        total_raised,
    }
    .publish(env);
}

/// Publish a `failed` event.
pub fn failed(env: &Env, request_id: u64, total_raised: i128) {
    Failed {
        request_id,
        total_raised,
    }
    .publish(env);
}

/// Publish a `milestone_submitted` event.
pub fn milestone_submitted(env: &Env, request_id: u64, index: u32, evidence_hash: BytesN<32>) {
    MilestoneSubmitted {
        request_id,
        index,
        evidence_hash,
    }
    .publish(env);
}

/// Publish a `milestone_approved` event.
pub fn milestone_approved(env: &Env, request_id: u64, index: u32, approver: Address) {
    MilestoneApproved {
        request_id,
        index,
        approver,
    }
    .publish(env);
}

/// Publish a `milestone_released` event.
pub fn milestone_released(env: &Env, request_id: u64, index: u32, amount: i128) {
    MilestoneReleased {
        request_id,
        index,
        amount,
    }
    .publish(env);
}

/// Publish a `refunded` event.
pub fn refunded(env: &Env, request_id: u64, contributor: Address, amount: i128) {
    Refunded {
        request_id,
        contributor,
        amount,
    }
    .publish(env);
}

/// Publish a `cancelled` event.
pub fn cancelled(env: &Env, request_id: u64) {
    Cancelled { request_id }.publish(env);
}
