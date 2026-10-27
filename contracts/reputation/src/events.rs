use soroban_sdk::{contractevent, Address, Env, String};

/// Emitted when an address reputation score changes: `subject, delta, new_score, ref_id`.
#[contractevent(topics = ["score_changed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScoreChanged {
    pub subject: Address,
    pub delta: i32,
    pub new_score: i64,
    pub ref_id: String,
}

pub fn score_changed(env: &Env, subject: Address, delta: i32, new_score: i64, ref_id: String) {
    ScoreChanged {
        subject,
        delta,
        new_score,
        ref_id,
    }
    .publish(env);
}
