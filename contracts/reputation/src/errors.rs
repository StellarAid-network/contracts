use soroban_sdk::contracterror;

/// Errors returned by the Reputation contract.
///
/// Numeric codes are stable public ABI; append new codes without renumbering.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract has not been initialized.
    NotInitialized = 1,
    /// Initialization attempted twice.
    AlreadyInitialized = 2,
    /// Caller is not authorized.
    Unauthorized = 3,
    /// Invalid delta parameter.
    InvalidDelta = 4,
}
