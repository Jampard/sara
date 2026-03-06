//! Fingerprint computation for suspect link detection.

mod compute;
pub mod review;

pub use compute::{
    compute_fingerprint, compute_item_fingerprint, fingerprinted_fields, truncate_fingerprint,
};
