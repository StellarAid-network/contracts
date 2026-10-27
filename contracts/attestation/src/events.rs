use soroban_sdk::{contractevent, Address, Env};

/// Emitted when an attestation is created: `attestation_id, attestor, subject`.
#[contractevent(topics = ["attested"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attested {
    pub attestation_id: u64,
    pub attestor: Address,
    pub subject: Address,
}

/// Emitted when an attestation is revoked: `attestation_id, attestor, subject`.
#[contractevent(topics = ["revoked"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Revoked {
    pub attestation_id: u64,
    pub attestor: Address,
    pub subject: Address,
}

pub fn attested(env: &Env, attestation_id: u64, attestor: Address, subject: Address) {
    Attested {
        attestation_id,
        attestor,
        subject,
    }
    .publish(env);
}

pub fn revoked(env: &Env, attestation_id: u64, attestor: Address, subject: Address) {
    Revoked {
        attestation_id,
        attestor,
        subject,
    }
    .publish(env);
}
