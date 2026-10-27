use soroban_sdk::contracterror;

/// Errors returned by the Help Request contract.
///
/// Numeric codes are part of the public API and must stay stable. Append new codes;
/// never renumber existing ones.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract has not been initialized.
    NotInitialized = 1,
    /// Initialization attempted twice.
    AlreadyInitialized = 2,
    /// Unknown request id.
    RequestNotFound = 3,
    /// Amount is zero or negative.
    InvalidAmount = 4,
    /// Action not allowed after the deadline.
    DeadlinePassed = 5,
    /// Finalize attempted before the deadline.
    DeadlineNotReached = 6,
    /// Request is not accepting contributions.
    NotOpen = 7,
    /// Caller lacks permission.
    Unauthorized = 8,
    /// Token is not on the allowed list.
    TokenNotAllowed = 9,
    /// No refundable balance.
    NothingToRefund = 10,
    /// Milestone index out of bounds.
    MilestoneNotFound = 11,
    /// Milestone already released.
    MilestoneAlreadyReleased = 12,
    /// Milestone evidence not yet submitted.
    MilestoneNotSubmitted = 13,
    /// Milestone evidence already submitted.
    MilestoneAlreadySubmitted = 14,
    /// Caller has already approved this milestone.
    MilestoneAlreadyApproved = 15,
    /// Milestone has not reached required approval threshold.
    InsufficientApprovals = 16,
    /// Milestone schedule amounts do not sum to goal or invalid count.
    InvalidMilestoneSchedule = 17,
    /// Milestones must be submitted and released in sequential order.
    MilestoneOutOfOrder = 18,
    /// Request is not in a disputed or refundable state.
    NotDisputedOrCancelled = 19,
}
