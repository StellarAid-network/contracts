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
}
