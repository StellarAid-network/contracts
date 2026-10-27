use soroban_sdk::{contracttype, Address, BytesN, Symbol};

/// An on-chain attestation record vouching for a subject.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attestation {
    /// Unique monotonic attestation id.
    pub id: u64,
    /// Authorized attestor address that issued this statement.
    pub attestor: Address,
    /// Subject address (user or help request).
    pub subject: Address,
    /// Attestation category or kind (e.g. "verified_need", "identity").
    pub kind: Symbol,
    /// Content hash of the off-chain evidence or statement.
    pub content_hash: BytesN<32>,
    /// Optional expiration timestamp (ledger seconds).
    pub expires_at: Option<u64>,
    /// Revocation flag.
    pub revoked: bool,
    /// Creation timestamp (ledger seconds).
    pub created_at: u64,
}

/// Storage keys for the Attestation contract.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Contract admin (instance storage).
    Admin,
    /// Authorized attestors (persistent storage).
    Attestor(Address),
    /// Metadata hash for an attestor (persistent storage).
    AttestorMeta(Address),
    /// An attestation record by id (persistent storage).
    Attestation(u64),
    /// List of attestation IDs for a subject (persistent storage).
    SubjectAttestations(Address),
    /// Next attestation ID counter (instance storage).
    NextId,
}
