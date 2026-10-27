use soroban_sdk::contracterror;

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
    /// Attestation does not exist.
    AttestationNotFound = 4,
    /// Attestation was already revoked.
    AlreadyRevoked = 5,
    /// Expiration timestamp must be in the future.
    InvalidExpiry = 6,
}
