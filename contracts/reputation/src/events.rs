//! Event definitions and helpers for the Reputation contract.

use soroban_sdk::{contractevent, Address, Env, String};

/// Emitted when an address reputation score changes: `subject, delta, new_score, ref_id`.
#[contractevent(topics = ["score_changed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScoreChanged {
    /// Subject address whose reputation score changed.
    pub subject: Address,
    /// Signed score delta applied in this event.
    pub delta: i32,
    /// Updated cumulative reputation score after applying `delta`.
    pub new_score: i64,
    /// Off-chain or cross-contract outcome reference identifier.
    pub ref_id: String,
}

/// Publish a `score_changed` event.
pub fn score_changed(env: &Env, subject: Address, delta: i32, new_score: i64, ref_id: String) {
    ScoreChanged {
        subject,
        delta,
        new_score,
        ref_id,
    }
    .publish(env);
}
