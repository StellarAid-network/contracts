#![no_std]

//! StellarAid Attestation contract.
//!
//! An admin-managed registry of attestors that can vouch for requests or users.
//! Attestors sign statements stored as content hashes with optional expiration dates.

pub mod errors;
pub mod events;
pub mod types;

pub use crate::errors::Error;
pub use crate::types::{Attestation, DataKey};

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Symbol, Vec};

const INSTANCE_TTL_THRESHOLD: u32 = 17_280;
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400;

const PERSISTENT_TTL_THRESHOLD: u32 = 120_960;
const PERSISTENT_TTL_EXTEND_TO: u32 = 1_036_800;

#[contract]
pub struct AttestationContract;

#[contractimpl]
impl AttestationContract {
    /// One-time setup. Stores the admin. Calling twice returns `AlreadyInitialized`.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();

        let storage = env.storage().instance();
        if storage.has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        storage.set(&DataKey::Admin, &admin);
        storage.set(&DataKey::NextId, &1u64);
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        Ok(())
    }

    /// Add an approved attestor with metadata hash. Admin-only.
    pub fn add_attestor(env: Env, attestor: Address, meta_hash: BytesN<32>) -> Result<(), Error> {
        let storage = env.storage().instance();
        let admin: Address = storage.get(&DataKey::Admin).ok_or(Error::NotInitialized)?;
        admin.require_auth();
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        env.storage()
            .persistent()
            .set(&DataKey::Attestor(attestor.clone()), &true);
        env.storage()
            .persistent()
            .set(&DataKey::AttestorMeta(attestor.clone()), &meta_hash);

        env.storage().persistent().extend_ttl(
            &DataKey::Attestor(attestor.clone()),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );
        env.storage().persistent().extend_ttl(
            &DataKey::AttestorMeta(attestor),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        Ok(())
    }

    /// Remove an attestor from the registry. Admin-only.
    pub fn remove_attestor(env: Env, attestor: Address) -> Result<(), Error> {
        let storage = env.storage().instance();
        let admin: Address = storage.get(&DataKey::Admin).ok_or(Error::NotInitialized)?;
        admin.require_auth();
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        env.storage()
            .persistent()
            .remove(&DataKey::Attestor(attestor.clone()));
        env.storage()
            .persistent()
            .remove(&DataKey::AttestorMeta(attestor));

        Ok(())
    }

    /// Create an attestation vouching for a subject. Attestor-only.
    pub fn attest(
        env: Env,
        attestor: Address,
        subject: Address,
        kind: Symbol,
        content_hash: BytesN<32>,
        expires_at: Option<u64>,
    ) -> Result<u64, Error> {
        attestor.require_auth();

        let storage = env.storage().instance();
        storage
            .get::<_, Address>(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;

        let is_attestor: bool = env
            .storage()
            .persistent()
            .get(&DataKey::Attestor(attestor.clone()))
            .unwrap_or(false);

        if !is_attestor {
            return Err(Error::Unauthorized);
        }

        env.storage().persistent().extend_ttl(
            &DataKey::Attestor(attestor.clone()),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        let now = env.ledger().timestamp();
        if let Some(exp) = expires_at {
            if exp <= now {
                return Err(Error::InvalidExpiry);
            }
        }

        let next_id: u64 = storage.get(&DataKey::NextId).unwrap_or(1u64);
        let incremented_id = next_id.checked_add(1).ok_or(Error::InvalidExpiry)?;
        storage.set(&DataKey::NextId, &incremented_id);
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        let attestation = Attestation {
            id: next_id,
            attestor: attestor.clone(),
            subject: subject.clone(),
            kind,
            content_hash,
            expires_at,
            revoked: false,
            created_at: now,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Attestation(next_id), &attestation);
        env.storage().persistent().extend_ttl(
            &DataKey::Attestation(next_id),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        let mut subject_list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::SubjectAttestations(subject.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        subject_list.push_back(next_id);

        env.storage().persistent().set(
            &DataKey::SubjectAttestations(subject.clone()),
            &subject_list,
        );
        env.storage().persistent().extend_ttl(
            &DataKey::SubjectAttestations(subject.clone()),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        events::attested(&env, next_id, attestor, subject);

        Ok(next_id)
    }

    /// Revoke an active attestation. Only the issuing attestor may revoke it.
    pub fn revoke(env: Env, attestor: Address, attestation_id: u64) -> Result<(), Error> {
        attestor.require_auth();

        let storage = env.storage().instance();
        storage
            .get::<_, Address>(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        let mut attestation: Attestation = env
            .storage()
            .persistent()
            .get(&DataKey::Attestation(attestation_id))
            .ok_or(Error::AttestationNotFound)?;

        if attestation.attestor != attestor {
            return Err(Error::Unauthorized);
        }

        if attestation.revoked {
            return Err(Error::AlreadyRevoked);
        }

        attestation.revoked = true;
        env.storage()
            .persistent()
            .set(&DataKey::Attestation(attestation_id), &attestation);
        env.storage().persistent().extend_ttl(
            &DataKey::Attestation(attestation_id),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        events::revoked(&env, attestation_id, attestor, attestation.subject);

        Ok(())
    }

    /// Read active, non-expired, non-revoked attestations for a subject.
    pub fn get_attestations(env: Env, subject: Address) -> Vec<Attestation> {
        let mut result = Vec::new(&env);
        let id_list: Vec<u64> = match env
            .storage()
            .persistent()
            .get(&DataKey::SubjectAttestations(subject))
        {
            Some(list) => list,
            None => return result,
        };

        let now = env.ledger().timestamp();
        for i in 0..id_list.len() {
            let id = id_list.get(i).unwrap();
            if let Some(attestation) = env
                .storage()
                .persistent()
                .get::<_, Attestation>(&DataKey::Attestation(id))
            {
                if !attestation.revoked {
                    if let Some(exp) = attestation.expires_at {
                        if exp <= now {
                            continue;
                        }
                    }
                    result.push_back(attestation);
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod test;
