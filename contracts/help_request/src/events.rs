//! Event definitions and helpers for the Help Request contract.
//!
//! Event shapes are public API consumed by the `api` indexer; treat changes as breaking.
//! These helpers only format and publish events. Business logic that calls them is not
//! implemented yet.

use soroban_sdk::{contractevent, Address, BytesN, Env};

/// Emitted when a request is created: `request_id, creator, token, goal, deadline`.
#[contractevent(topics = ["created"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Created {
    pub request_id: u64,
    pub creator: Address,
    pub token: Address,
    pub goal: i128,
    pub deadline: u64,
}

/// Emitted on each contribution: `request_id, contributor, amount, total_raised`.
#[contractevent(topics = ["contributed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Contributed {
    pub request_id: u64,
    pub contributor: Address,
    pub amount: i128,
    pub total_raised: i128,
}

/// Emitted when the goal is reached: `request_id, total_raised`.
#[contractevent(topics = ["funded"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Funded {
    pub request_id: u64,
    pub total_raised: i128,
}

/// Emitted when the deadline passes without the goal: `request_id, total_raised`.
#[contractevent(topics = ["failed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Failed {
    pub request_id: u64,
    pub total_raised: i128,
}

/// Emitted when a creator submits milestone evidence: `request_id, index, evidence_hash`.
#[contractevent(topics = ["milestone_submitted"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneSubmitted {
    pub request_id: u64,
    pub index: u32,
    pub evidence_hash: BytesN<32>,
}

/// Emitted when an approver approves a milestone: `request_id, index, approver`.
#[contractevent(topics = ["milestone_approved"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneApproved {
    pub request_id: u64,
    pub index: u32,
    pub approver: Address,
}

/// Emitted when a milestone amount is paid out: `request_id, index, amount`.
#[contractevent(topics = ["milestone_released"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneReleased {
    pub request_id: u64,
    pub index: u32,
    pub amount: i128,
}

/// Emitted when a contributor is refunded: `request_id, contributor, amount`.
#[contractevent(topics = ["refunded"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Refunded {
    pub request_id: u64,
    pub contributor: Address,
    pub amount: i128,
}

/// Emitted when a request is cancelled: `request_id`.
#[contractevent(topics = ["cancelled"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cancelled {
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
