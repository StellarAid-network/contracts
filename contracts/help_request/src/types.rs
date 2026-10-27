use soroban_sdk::{contracttype, Address, BytesN, Vec};

/// Lifecycle status of a help request.
///
/// Mirrors the state machine documented in the repository README:
/// `Open -> Funded -> Releasing -> Completed`, with `Failed`, `Cancelled`, and
/// `Disputed` reachable as described there.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    /// Accepting contributions.
    Open,
    /// Goal reached; milestones may be released.
    Funded,
    /// At least one milestone released; more may follow.
    Releasing,
    /// All milestones released.
    Completed,
    /// Deadline passed without reaching the goal; refunds are enabled.
    Failed,
    /// Creator or approvers cancelled; unreleased funds are refundable.
    Cancelled,
    /// A milestone was rejected; subject to dispute resolution.
    Disputed,
}

/// A single milestone in a request's schedule.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone {
    /// Position in the schedule.
    pub index: u32,
    /// Amount released when this milestone is approved.
    pub amount: i128,
    /// Hash of the evidence submitted by the creator, once submitted.
    pub evidence_hash: Option<BytesN<32>>,
    /// Whether this milestone has already been paid out.
    pub released: bool,
}

/// A help request and its escrow state.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    /// Monotonic request id assigned at creation.
    pub id: u64,
    /// Address that opened the request.
    pub creator: Address,
    /// SAC token held in escrow.
    pub token: Address,
    /// Funding goal, in the token's smallest unit.
    pub goal: i128,
    /// Total contributed so far.
    pub raised: i128,
    /// Total amount released across milestones.
    pub total_released: i128,
    /// Funding deadline (ledger timestamp, seconds).
    pub deadline: u64,
    /// Current lifecycle status.
    pub status: Status,
    /// IPFS CID or content hash of the off-chain description.
    pub content_hash: BytesN<32>,
    /// Milestone schedule; empty until milestones are configured.
    pub milestones: Vec<Milestone>,
    /// Creation timestamp.
    pub created_at: u64,
}

/// Contract configuration, set at initialization and updatable by the admin.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    /// Tokens accepted for contributions.
    pub allowed_tokens: Vec<Address>,
    /// Minimum accepted contribution, in the token's smallest unit.
    pub min_contribution: i128,
    /// Addresses allowed to approve milestones.
    pub approvers: Vec<Address>,
    /// Number of approvals required to release a milestone.
    pub approval_threshold: u32,
    /// Optional reputation contract notified on lifecycle events.
    pub reputation_contract: Option<Address>,
}

/// Storage keys.
///
/// Requests and per-contributor balances are stored under one key per entry; never a
/// single unbounded `Vec`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Contract admin (instance storage).
    Admin,
    /// Configuration (instance storage).
    Config,
    /// Next request id to assign.
    NextRequestId,
    /// A request by id (persistent storage).
    Request(u64),
    /// A contributor's total for a request (persistent storage).
    Contribution(u64, Address),
    /// Tracks if an approver has approved a milestone: (request_id, index, approver).
    MilestoneApprover(u64, u32, Address),
    /// Tracks the number of approvals for a milestone: (request_id, index).
    MilestoneApprovalCount(u64, u32),
}
