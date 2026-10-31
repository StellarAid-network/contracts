#![no_std]

//! StellarAid Help Request contract.
//!
//! Non-custodial funding-goal escrow with deadlines, sequential milestone payouts,
//! approver disputes, proportional refunds, and isolated reputation notifications.

mod errors;
pub mod events;
mod types;

pub use crate::errors::Error;
pub use crate::types::{Config, DataKey, Milestone, Request, Status};

use shared::CheckedAmount;
use soroban_sdk::{
    contract, contractimpl, token, Address, BytesN, Env, IntoVal, String, Symbol, Val, Vec,
};

/// Minimum instance-storage TTL kept on reads/writes (~1 day at 5s/ledger).
const INSTANCE_TTL_THRESHOLD: u32 = 17_280;
/// Target instance-storage TTL when extending (~30 days at 5s/ledger).
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400;

/// Minimum persistent-storage TTL kept on reads/writes (~7 days at 5s/ledger).
const PERSISTENT_TTL_THRESHOLD: u32 = 120_960;
/// Target persistent-storage TTL when extending (~60 days at 5s/ledger).
const PERSISTENT_TTL_EXTEND_TO: u32 = 1_036_800;

/// Helper to notify the reputation contract of a verified outcome.
///
/// Calls `record_event` via `try_invoke_contract`.
/// ISOLATION GUARANTEE: If the reputation contract reverts, runs out of gas, or
/// fails for any reason, the error is safely discarded so that fund movements and
/// request state transitions are never blocked or trapped.
fn notify_reputation(
    env: &Env,
    reputation_contract: &Option<Address>,
    subject: &Address,
    kind: Symbol,
    delta: i32,
    ref_id: &String,
) {
    if let Some(target) = reputation_contract {
        let caller = env.current_contract_address();
        let mut args = Vec::new(env);
        args.push_back(caller.to_val());
        args.push_back(subject.to_val());
        args.push_back(kind.to_val());
        args.push_back(delta.into_val(env));
        args.push_back(ref_id.to_val());
        let func = Symbol::new(env, "record_event");
        let _ = env.try_invoke_contract::<Val, soroban_sdk::InvokeError>(target, &func, args);
    }
}

/// Soroban smart contract managing help-request escrow, milestones, and contributor refunds.
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
        shared::init_storage_version(&env, shared::STORAGE_VERSION_V1);
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
        milestones: Vec<Milestone>,
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

        // Validate milestones schedule.
        const MAX_MILESTONES: u32 = 10;
        let final_milestones = if milestones.is_empty() {
            let mut v = Vec::new(&env);
            v.push_back(Milestone {
                index: 0,
                amount: goal,
                evidence_hash: None,
                released: false,
            });
            v
        } else {
            let count = milestones.len();
            if count > MAX_MILESTONES {
                return Err(Error::InvalidMilestoneSchedule);
            }
            let mut sum: i128 = 0;
            for i in 0..count {
                let m = milestones.get(i).unwrap();
                if m.index != i {
                    return Err(Error::MilestoneOutOfOrder);
                }
                if m.amount <= 0 || m.released || m.evidence_hash.is_some() {
                    return Err(Error::InvalidMilestoneSchedule);
                }
                sum = sum
                    .checked_add_amount(m.amount)
                    .ok_or(Error::InvalidAmount)?;
            }
            if sum != goal {
                return Err(Error::InvalidMilestoneSchedule);
            }
            milestones
        };

        // Assign request id.
        let id: u64 = storage.get(&DataKey::NextRequestId).unwrap_or(0u64);
        let next_id = id.checked_add(1).ok_or(Error::InvalidAmount)?;
        storage.set(&DataKey::NextRequestId, &next_id);
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        let request = Request {
            id,
            creator: creator.clone(),
            token: token.clone(),
            goal,
            raised: 0,
            total_released: 0,
            deadline,
            status: Status::Open,
            content_hash,
            milestones: final_milestones,
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
        token::Client::new(&env, &request.token).transfer(&contributor, &contract_address, &amount);

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

    /// Submit evidence for a milestone.
    ///
    /// Only the creator may call this. Request must be Funded or Releasing.
    /// Milestones must be submitted sequentially and only once.
    pub fn submit_milestone(
        env: Env,
        request_id: u64,
        index: u32,
        evidence_hash: BytesN<32>,
    ) -> Result<(), Error> {
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

        if request.status != Status::Funded && request.status != Status::Releasing {
            return Err(Error::NotOpen);
        }

        if index >= request.milestones.len() {
            return Err(Error::MilestoneNotFound);
        }

        // Verify previous milestones are already released.
        for i in 0..index {
            let prev = request.milestones.get(i).unwrap();
            if !prev.released {
                return Err(Error::MilestoneOutOfOrder);
            }
        }

        let mut milestone = request.milestones.get(index).unwrap();
        if milestone.released {
            return Err(Error::MilestoneAlreadyReleased);
        }
        if milestone.evidence_hash.is_some() {
            return Err(Error::MilestoneAlreadySubmitted);
        }

        milestone.evidence_hash = Some(evidence_hash.clone());
        request.milestones.set(index, milestone);

        env.storage()
            .persistent()
            .set(&DataKey::Request(request_id), &request);
        env.storage().persistent().extend_ttl(
            &DataKey::Request(request_id),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        events::milestone_submitted(&env, request_id, index, evidence_hash);

        Ok(())
    }

    /// Approve a submitted milestone.
    ///
    /// Must be an approver configured in Config. An approver cannot approve twice.
    pub fn approve_milestone(
        env: Env,
        request_id: u64,
        index: u32,
        approver: Address,
    ) -> Result<(), Error> {
        approver.require_auth();

        let storage = env.storage().instance();
        let config: Config = storage.get(&DataKey::Config).ok_or(Error::NotInitialized)?;
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        if !config.approvers.contains(&approver) {
            return Err(Error::Unauthorized);
        }

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

        if request.status != Status::Funded && request.status != Status::Releasing {
            return Err(Error::NotOpen);
        }

        if index >= request.milestones.len() {
            return Err(Error::MilestoneNotFound);
        }

        let milestone = request.milestones.get(index).unwrap();
        if milestone.released {
            return Err(Error::MilestoneAlreadyReleased);
        }
        if milestone.evidence_hash.is_none() {
            return Err(Error::MilestoneNotSubmitted);
        }

        let approver_key = DataKey::MilestoneApprover(request_id, index, approver.clone());
        if env.storage().persistent().has(&approver_key) {
            return Err(Error::MilestoneAlreadyApproved);
        }

        let count_key = DataKey::MilestoneApprovalCount(request_id, index);
        let count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0u32);
        let next_count = count.checked_add(1).ok_or(Error::InvalidAmount)?;

        env.storage().persistent().set(&approver_key, &true);
        env.storage().persistent().extend_ttl(
            &approver_key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        env.storage().persistent().set(&count_key, &next_count);
        env.storage().persistent().extend_ttl(
            &count_key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        events::milestone_approved(&env, request_id, index, approver);

        Ok(())
    }

    /// Pay out an approved milestone to the request creator.
    ///
    /// Anyone may call this once sufficient approvals exist.
    /// Checks-effects-interactions: updates contract state and milestone status
    /// before executing the token transfer.
    pub fn release_milestone(env: Env, request_id: u64, index: u32) -> Result<(), Error> {
        let storage = env.storage().instance();
        let config: Config = storage.get(&DataKey::Config).ok_or(Error::NotInitialized)?;
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        let mut request: Request = env
            .storage()
            .persistent()
            .get(&DataKey::Request(request_id))
            .ok_or(Error::RequestNotFound)?;

        if request.status != Status::Funded && request.status != Status::Releasing {
            return Err(Error::NotOpen);
        }

        if index >= request.milestones.len() {
            return Err(Error::MilestoneNotFound);
        }

        // Verify previous milestones are already released.
        for i in 0..index {
            let prev = request.milestones.get(i).unwrap();
            if !prev.released {
                return Err(Error::MilestoneOutOfOrder);
            }
        }

        let mut milestone = request.milestones.get(index).unwrap();
        if milestone.released {
            return Err(Error::MilestoneAlreadyReleased);
        }
        if milestone.evidence_hash.is_none() {
            return Err(Error::MilestoneNotSubmitted);
        }

        let count_key = DataKey::MilestoneApprovalCount(request_id, index);
        let approvals: u32 = env.storage().persistent().get(&count_key).unwrap_or(0u32);
        if approvals < config.approval_threshold {
            return Err(Error::InsufficientApprovals);
        }

        // --- EFFECTS (state update before interaction) ---
        milestone.released = true;
        request.milestones.set(index, milestone.clone());

        request.total_released = request
            .total_released
            .checked_add_amount(milestone.amount)
            .ok_or(Error::InvalidAmount)?;

        let mut all_released = true;
        for i in 0..request.milestones.len() {
            if !request.milestones.get(i).unwrap().released {
                all_released = false;
                break;
            }
        }

        if all_released {
            request.status = Status::Completed;
        } else {
            request.status = Status::Releasing;
        }

        env.storage()
            .persistent()
            .set(&DataKey::Request(request_id), &request);
        env.storage().persistent().extend_ttl(
            &DataKey::Request(request_id),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        // --- INTERACTIONS (transfer tokens to creator) ---
        let contract_address = env.current_contract_address();
        token::Client::new(&env, &request.token).transfer(
            &contract_address,
            &request.creator,
            &milestone.amount,
        );

        events::milestone_released(&env, request_id, index, milestone.amount);

        // Reputation notification (isolated, non-blocking)
        if request.status == Status::Completed {
            notify_reputation(
                &env,
                &config.reputation_contract,
                &request.creator,
                Symbol::new(&env, "completed"),
                10,
                &String::from_str(&env, "completed"),
            );
        } else {
            notify_reputation(
                &env,
                &config.reputation_contract,
                &request.creator,
                Symbol::new(&env, "milestone"),
                5,
                &String::from_str(&env, "milestone"),
            );
        }

        Ok(())
    }

    /// Mark a milestone as disputed/rejected by an authorized approver.
    ///
    /// Transitions the request to Disputed status, freezing further milestone
    /// releases and allowing contributors to claim proportional refunds of unreleased funds.
    pub fn dispute_milestone(
        env: Env,
        request_id: u64,
        index: u32,
        approver: Address,
    ) -> Result<(), Error> {
        approver.require_auth();

        let storage = env.storage().instance();
        let config: Config = storage.get(&DataKey::Config).ok_or(Error::NotInitialized)?;
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);

        if !config.approvers.contains(&approver) {
            return Err(Error::Unauthorized);
        }

        let mut request: Request = env
            .storage()
            .persistent()
            .get(&DataKey::Request(request_id))
            .ok_or(Error::RequestNotFound)?;

        if request.status != Status::Funded && request.status != Status::Releasing {
            return Err(Error::NotOpen);
        }

        if index >= request.milestones.len() {
            return Err(Error::MilestoneNotFound);
        }

        let milestone = request.milestones.get(index).unwrap();
        if milestone.released {
            return Err(Error::MilestoneAlreadyReleased);
        }

        request.status = Status::Disputed;

        env.storage()
            .persistent()
            .set(&DataKey::Request(request_id), &request);
        env.storage().persistent().extend_ttl(
            &DataKey::Request(request_id),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

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

        if request.status != Status::Failed
            && request.status != Status::Cancelled
            && request.status != Status::Disputed
        {
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

        let remaining_unreleased = request
            .raised
            .checked_sub(request.total_released)
            .ok_or(Error::InvalidAmount)?;
        if remaining_unreleased <= 0 {
            return Err(Error::NothingToRefund);
        }

        env.storage().persistent().extend_ttl(
            &DataKey::Request(request_id),
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        // Proportional refund calculation:
        // If no milestones released or Failed, full 100% refund of contributed amount.
        // Otherwise, proportional share of remaining unreleased funds:
        // refund_amount = (amount * remaining_unreleased) / request.raised.
        let refund_amount = if request.total_released == 0 || request.status == Status::Failed {
            amount
        } else {
            let prod = amount
                .checked_mul(remaining_unreleased)
                .ok_or(Error::InvalidAmount)?;
            prod / request.raised
        };

        if refund_amount <= 0 {
            return Err(Error::NothingToRefund);
        }

        // --- EFFECTS (zero out before interaction) ---
        env.storage().persistent().set(&contribution_key, &0i128);
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
            &refund_amount,
        );

        events::refunded(&env, request_id, contributor, refund_amount);

        // Reputation notification (isolated, non-blocking)
        if request.status == Status::Failed {
            let storage = env.storage().instance();
            if let Some(cfg) = storage.get::<_, Config>(&DataKey::Config) {
                notify_reputation(
                    &env,
                    &cfg.reputation_contract,
                    &request.creator,
                    Symbol::new(&env, "refund"),
                    1,
                    &String::from_str(&env, "refund"),
                );
            }
        }

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

impl HelpRequest {
    /// Propose a time-locked WASM upgrade gated by the contract admin (intended for multisig).
    pub fn propose_wasm_upgrade(
        env: &Env,
        wasm_hash: BytesN<32>,
        timelock_delay: u64,
    ) -> Result<shared::PendingUpgrade, Error> {
        let storage = env.storage().instance();
        let admin: Address = storage.get(&DataKey::Admin).ok_or(Error::NotInitialized)?;
        let pending = shared::propose_upgrade(env, &admin, wasm_hash, timelock_delay)
            .map_err(|_| Error::InvalidAmount)?;
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
        Ok(pending)
    }

    /// Execute a previously proposed WASM upgrade after the time-lock has elapsed.
    pub fn execute_wasm_upgrade(env: &Env, apply_wasm: bool) -> Result<BytesN<32>, Error> {
        let storage = env.storage().instance();
        let admin: Address = storage.get(&DataKey::Admin).ok_or(Error::NotInitialized)?;
        let hash = shared::execute_upgrade(env, &admin, apply_wasm).map_err(|e| match e {
            shared::UpgradeError::TimelockNotExpired => Error::DeadlineNotReached,
            _ => Error::Unauthorized,
        })?;
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
        Ok(hash)
    }

    /// Explicit versioned storage migration scaffolding (`from_version -> to_version`).
    pub fn migrate_version(env: &Env, from_version: u32, to_version: u32) -> Result<u32, Error> {
        let storage = env.storage().instance();
        let admin: Address = storage.get(&DataKey::Admin).ok_or(Error::NotInitialized)?;
        let v = shared::migrate_storage(env, &admin, from_version, to_version, |_e| {})
            .map_err(|_| Error::Unauthorized)?;
        storage.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
        Ok(v)
    }
}

#[cfg(test)]
mod test;
