#![no_std]
use soroban_sdk::{contracttype, Address, String};

/// Payment status for escrow and routing contracts.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum PaymentStatus {
    Pending,
    Released,
    Refunded,
    Expired,
}

/// A payment record stored on-chain.
#[contracttype]
#[derive(Clone, Debug)]
pub struct PaymentRecord {
    pub id: String,
    pub sender: Address,
    pub recipient: Address,
    pub amount: i128,
    pub asset: Address,
    pub status: PaymentStatus,
    pub created_at: u64,
    pub expires_at: u64,
}

/// Error codes shared across contracts.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    NotFound = 1,
    Unauthorized = 2,
    AlreadyExists = 3,
    Expired = 4,
    InsufficientFunds = 5,
    InvalidAmount = 6,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Error::NotFound => write!(f, "not found"),
            Error::Unauthorized => write!(f, "unauthorized"),
            Error::AlreadyExists => write!(f, "already exists"),
            Error::Expired => write!(f, "expired"),
            Error::InsufficientFunds => write!(f, "insufficient funds"),
            Error::InvalidAmount => write!(f, "invalid amount"),
        }
    }
}

/// Returns true if the payment has passed its expiry timestamp.
pub fn is_expired(record: &PaymentRecord, now: u64) -> bool {
    now >= record.expires_at
}

/// Count payments by status in a slice.
pub fn count_by_status(records: &[PaymentRecord], status: &PaymentStatus) -> usize {
    records.iter().filter(|r| &r.status == status).count()
}

pub const CONTRACT_VERSION: &str = "0.1.0";

// This crate is #![no_std] — no heap allocations outside soroban_sdk types
