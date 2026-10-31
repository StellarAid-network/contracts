use soroban_sdk::{contracttype, Address, Symbol};

/// A reputation badge awarded when score reaches a threshold.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Badge {
    /// Badge identifier / name (e.g. "Helper", "Advocate", "Champion").
    pub name: Symbol,
    /// Tier level (1, 2, 3...).
    pub tier: u32,
    /// Minimum score required to earn this badge.
    pub threshold: i64,
}

/// Configuration for badge thresholds.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    /// Minimum score required for Tier 1 (`Helper`).
    pub tier1_threshold: i64,
    /// Minimum score required for Tier 2 (`Advocate`).
    pub tier2_threshold: i64,
    /// Minimum score required for Tier 3 (`Champion`).
    pub tier3_threshold: i64,
}

/// Storage keys for the Reputation contract.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Contract admin (instance storage).
    Admin,
    /// Configuration (instance storage).
    Config,
    /// Authorized reporter addresses (persistent storage).
    Reporter(Address),
    /// Cumulative reputation score for a subject (persistent storage).
    Score(Address),
}
