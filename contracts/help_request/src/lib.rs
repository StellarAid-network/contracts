#![no_std]

//! StellarAid Help Request contract.
//!
//! Funding-goal escrow with a deadline, refunds, and milestone release.
//! This module implements the core flow (no milestones yet).

mod errors;
pub mod events;
mod types;

pub use crate::errors::Error;
pub use crate::types::{Config, DataKey, Milestone, Request, Status};

use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env, Vec};
use shared::CheckedAmount;

/// Minimum instance-storage TTL kept on reads/writes (~1 day at 5s/ledger).
const INSTANCE_TTL_THRESHOLD: u32 = 17_280;
/// Target instance-storage TTL when extending (~30 days at 5s/ledger).
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400;

/// Minimum persistent-storage TTL kept on reads/writes (~7 days at 5s/ledger).
const PERSISTENT_TTL_THRESHOLD: u32 = 120_960;
/// Target persistent-storage TTL when extending (~60 days at 5s/ledger).
const PERSISTENT_TTL_EXTEND_TO: u32 = 1_036_800;

#[contract]
pub struct HelpRequest;

#[contractimpl]
impl HelpRequest {
    // -------------------------------------------------------------------------
    // Initialization and configuration
    // -------------------------------------------------------------------------

    /// One-time setup. Stores the admin and configuration in instance storage.
    ///
    /// `admin` must authorize the call. Calling twice returns [`Error::AlreadyInitialized`].
    pub fn initialize(env: Env, admin: Address, config: Config) -> Result<(), Error> {
        admin.require_auth();

        let storage = env.storage().instance();
        if storage.has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        storage.set(&DataKey::Admin, &admin);
        storage.set(&DataKey::Config, &config);
        storage.set(&DataKey::NextRequestId, &0u64);
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        Ok(())
    }

    /// Update fee and token settings. Only the admin may call this.
    ///
    /// There is intentionally no way for the admin to access escrowed funds via
    /// config changes—the only fund-moving functions are `contribute`, `refund`,
    /// and `release_milestone`.
    pub fn set_config(env: Env, config: Config) -> Result<(), Error> {
        let storage = env.storage().instance();
        let admin: Address = storage.get(&DataKey::Admin).ok_or(Error::NotInitialized)?;
        admin.require_auth();

        storage.set(&DataKey::Config, &config);
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Request lifecycle
    // -------------------------------------------------------------------------

    /// Open a new help request and return its id.
    ///
    /// `creator` must authorize the call. The token must be on the allowed list.
    /// `deadline` is a ledger UNIX timestamp in seconds; must be in the future.
    pub fn create_request(
        env: Env,
        creator: Address,
        token: Address,
        goal: i128,
        deadline: u64,
        content_hash: BytesN<32>,
    ) -> Result<u64, Error> {
        creator.require_auth();

        let storage = env.storage().instance();
        let config: Config = storage.get(&DataKey::Config).ok_or(Error::NotInitialized)?;

        // Validate goal is positive.
        if goal <= 0 {
            return Err(Error::InvalidAmount);
        }

        // Validate token is allowed.
        if !config.allowed_tokens.contains(&token) {
            return Err(Error::TokenNotAllowed);
        }

        // Validate deadline is in the future.
        let now = env.ledger().timestamp();
        if deadline <= now {
            return Err(Error::DeadlinePassed);
        }

        // Assign request id.
        let id: u64 = storage.get(&DataKey::NextRequestId).unwrap_or(0u64);
        storage.set(&DataKey::NextRequestId, &(id + 1));
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        let request = Request {
            id,
            creator: creator.clone(),
            token: token.clone(),
            goal,
            raised: 0,
            deadline,
            status: Status::Open,
            content_hash,
            milestones: Vec::new(&env),
            created_at: now,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Request(id), &request);
        env.storage().persistent().extend_ttl(
            &DataKey::Request(id),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        events::created(&env, id, creator, token, goal, deadline);

        Ok(id)
    }

    /// Transfer tokens into escrow for an existing request.
    ///
    /// `contributor` must authorize the call. The request must be Open and the
    /// deadline must not have passed. Amount must meet the minimum contribution.
    ///
    /// Uses checks-effects-interactions: state is updated before the token transfer.
    pub fn contribute(
        env: Env,
        request_id: u64,
        contributor: Address,
        amount: i128,
    ) -> Result<(), Error> {
        contributor.require_auth();

        let storage = env.storage().instance();
        let config: Config = storage.get(&DataKey::Config).ok_or(Error::NotInitialized)?;
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        // Load and validate request.
        let mut request: Request = env
            .storage()
            .persistent()
            .get(&DataKey::Request(request_id))
            .ok_or(Error::RequestNotFound)?;

        if request.status != Status::Open {
            return Err(Error::NotOpen);
        }

        let now = env.ledger().timestamp();
        if now >= request.deadline {
            return Err(Error::DeadlinePassed);
        }

        if amount <= 0 || amount < config.min_contribution {
            return Err(Error::InvalidAmount);
        }

        // Load or initialize contributor balance.
        let contribution_key = DataKey::Contribution(request_id, contributor.clone());
        let existing: i128 = env
            .storage()
            .persistent()
            .get(&contribution_key)
            .unwrap_or(0i128);

        let new_contribution = existing
            .checked_add_amount(amount)
            .ok_or(Error::InvalidAmount)?;

        let new_raised = request
            .raised
            .checked_add_amount(amount)
            .ok_or(Error::InvalidAmount)?;

        // --- EFFECTS (update state before interactions) ---
        request.raised = new_raised;

        let funded = new_raised >= request.goal;
        if funded {
            request.status = Status::Funded;
        }

        env.storage()
            .persistent()
            .set(&DataKey::Request(request_id), &request);
        env.storage().persistent().extend_ttl(
            &DataKey::Request(request_id),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        env.storage()
            .persistent()
            .set(&contribution_key, &new_contribution);
        env.storage().persistent().extend_ttl(
            &contribution_key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        // --- INTERACTIONS (token transfer after state is committed) ---
        let contract_address = env.current_contract_address();
        token::Client::new(&env, &request.token).transfer(
            &contributor,
            &contract_address,
            &amount,
        );

        // Emit events.
        events::contributed(&env, request_id, contributor, amount, new_raised);
        if funded {
            events::funded(&env, request_id, new_raised);
        }

        Ok(())
    }

    /// After the deadline, transition an Open request to Funded or Failed.
    ///
    /// Anyone may call this. Emits `funded` or `failed`.
    pub fn finalize(env: Env, request_id: u64) -> Result<(), Error> {
        env.storage()
            .instance()
            .has(&DataKey::Admin)
            .then_some(())
            .ok_or(Error::NotInitialized)?;
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        let mut request: Request = env
            .storage()
            .persistent()
            .get(&DataKey::Request(request_id))
            .ok_or(Error::RequestNotFound)?;

        if request.status != Status::Open {
            return Err(Error::NotOpen);
        }

        let now = env.ledger().timestamp();
        if now < request.deadline {
            return Err(Error::DeadlineNotReached);
        }

        if request.raised >= request.goal {
            request.status = Status::Funded;
            env.storage()
                .persistent()
                .set(&DataKey::Request(request_id), &request);
            env.storage().persistent().extend_ttl(
                &DataKey::Request(request_id),
                PERSISTENT_TTL_THRESHOLD,
                PERSISTENT_TTL_EXTEND_TO,
            );
            events::funded(&env, request_id, request.raised);
        } else {
            request.status = Status::Failed;
            env.storage()
                .persistent()
                .set(&DataKey::Request(request_id), &request);
            env.storage().persistent().extend_ttl(
                &DataKey::Request(request_id),
                PERSISTENT_TTL_THRESHOLD,
                PERSISTENT_TTL_EXTEND_TO,
            );
            events::failed(&env, request_id, request.raised);
        }

        Ok(())
    }

    /// Cancel an Open request, enabling refunds.
    ///
    /// Only the creator may cancel, and only before the request is Funded.
    pub fn cancel(env: Env, request_id: u64) -> Result<(), Error> {
        env.storage()
            .instance()
            .has(&DataKey::Admin)
            .then_some(())
            .ok_or(Error::NotInitialized)?;
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        let mut request: Request = env
            .storage()
            .persistent()
            .get(&DataKey::Request(request_id))
            .ok_or(Error::RequestNotFound)?;

        request.creator.require_auth();

        if request.status != Status::Open {
            return Err(Error::NotOpen);
        }

        request.status = Status::Cancelled;
        env.storage()
            .persistent()
            .set(&DataKey::Request(request_id), &request);
        env.storage().persistent().extend_ttl(
            &DataKey::Request(request_id),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        events::cancelled(&env, request_id);

        Ok(())
    }

    /// Withdraw a contribution from a Failed or Cancelled request.
    ///
    /// `contributor` must authorize the call. Checks-effects-interactions: the stored
    /// balance is zeroed before the token transfer, so double-refund is impossible.
    pub fn refund(env: Env, request_id: u64, contributor: Address) -> Result<(), Error> {
        contributor.require_auth();

        env.storage()
            .instance()
            .has(&DataKey::Admin)
            .then_some(())
            .ok_or(Error::NotInitialized)?;
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        let request: Request = env
            .storage()
            .persistent()
            .get(&DataKey::Request(request_id))
            .ok_or(Error::RequestNotFound)?;

        if request.status != Status::Failed && request.status != Status::Cancelled {
            return Err(Error::NotOpen);
        }

        let contribution_key = DataKey::Contribution(request_id, contributor.clone());
        let amount: i128 = env
            .storage()
            .persistent()
            .get(&contribution_key)
            .unwrap_or(0i128);

        if amount <= 0 {
            return Err(Error::NothingToRefund);
        }

        // --- EFFECTS (zero out before interaction) ---
        env.storage()
            .persistent()
            .set(&contribution_key, &0i128);
        env.storage().persistent().extend_ttl(
            &contribution_key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        // --- INTERACTIONS ---
        let contract_address = env.current_contract_address();
        token::Client::new(&env, &request.token).transfer(
            &contract_address,
            &contributor,
            &amount,
        );

        events::refunded(&env, request_id, contributor, amount);

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Read-only accessors
    // -------------------------------------------------------------------------

    /// Read a request by id.
    pub fn get_request(env: Env, request_id: u64) -> Result<Request, Error> {
        env.storage()
            .instance()
            .has(&DataKey::Admin)
            .then_some(())
            .ok_or(Error::NotInitialized)?;
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        let request: Request = env
            .storage()
            .persistent()
            .get(&DataKey::Request(request_id))
            .ok_or(Error::RequestNotFound)?;

        env.storage().persistent().extend_ttl(
            &DataKey::Request(request_id),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        Ok(request)
    }

    /// Read a contributor's total contribution to a request.
    pub fn get_contribution(env: Env, request_id: u64, contributor: Address) -> i128 {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        let contribution_key = DataKey::Contribution(request_id, contributor);
        let amount: i128 = env
            .storage()
            .persistent()
            .get(&contribution_key)
            .unwrap_or(0i128);

        if amount > 0 {
            env.storage().persistent().extend_ttl(
                &contribution_key,
                PERSISTENT_TTL_THRESHOLD,
                PERSISTENT_TTL_EXTEND_TO,
            );
        }

        amount
    }
}

#[cfg(test)]
mod test;
