#![no_std]

//! Shared types, constants, and helper traits used across StellarAid contracts.
//!
//! Keep this crate free of business logic. It exists so the contracts agree on the
//! same domain vocabulary and small helpers.

use soroban_sdk::Address;

/// A Stellar account (G...) address.
pub type AccountAddress = Address;

/// The address of a SAC (Stellar Asset Contract) used as an escrow token.
pub type TokenAddress = Address;

/// A ledger UNIX timestamp, in seconds.
pub type Timestamp = u64;

/// Conservative placeholder for the smallest accepted contribution, in the token's
/// smallest unit. The real value is configuration-driven (`set_config`).
pub const MIN_CONTRIBUTION: i128 = 1;

/// Helper methods for combining escrowed amounts without silent overflow.
pub trait CheckedAmount {
    /// Add two amounts, returning `None` on overflow.
    fn checked_add_amount(self, other: Self) -> Option<Self>
    where
        Self: Sized;
}

impl CheckedAmount for i128 {
    fn checked_add_amount(self, other: Self) -> Option<Self> {
        self.checked_add(other)
    }
}

#[cfg(test)]
mod test;
