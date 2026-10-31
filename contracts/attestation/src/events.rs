//! Event definitions and helpers for the Attestation contract.

use soroban_sdk::{contractevent, Address, Env};

/// Emitted when an attestation is created: `attestation_id, attestor, subject`.
#[contractevent(topics = ["attested"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attested {
    /// Monotonic identifier of the created attestation.
    pub attestation_id: u64,
    /// Authorized attestor address that issued the statement.
    pub attestor: Address,
    /// Subject user or help-request address.
    pub subject: Address,
}

/// Emitted when an attestation is revoked: `attestation_id, attestor, subject`.
#[contractevent(topics = ["revoked"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Revoked {
    /// Identifier of the revoked attestation.
    pub attestation_id: u64,
    /// Issuing attestor address that revoked the statement.
    pub attestor: Address,
    /// Subject user or help-request address.
    pub subject: Address,
}

/// Publish an `attested` event.
pub fn attested(env: &Env, attestation_id: u64, attestor: Address, subject: Address) {
    Attested {
        attestation_id,
        attestor,
        subject,
    }
    .publish(env);
}

/// Publish a `revoked` event.
pub fn revoked(env: &Env, attestation_id: u64, attestor: Address, subject: Address) {
    Revoked {
        attestation_id,
        attestor,
        subject,
    }
    .publish(env);
}
