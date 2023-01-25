mod into_shares;
pub mod replicated;
mod scheme;
#[cfg(any(test, feature = "test-fixture", feature = "cli"))]
pub use into_shares::IntoShares;

pub use scheme::{Arithmetic, Boolean, SecretSharing};

use crate::bits::{BooleanOps, Serializable};
use crate::ff::ArithmeticOps;
use std::fmt::Debug;

pub trait SharedValue: Clone + Copy + PartialEq + Debug + Send + Sync + Sized + Serializable + 'static {
    /// Number of bits stored in this data type.
    const BITS: u32;

    const ZERO: Self;
}

pub trait ArithmeticShare: SharedValue + ArithmeticOps {}

pub trait BooleanShare: SharedValue + BooleanOps {}

impl<T> ArithmeticShare for T where T: SharedValue + ArithmeticOps {}

impl<T> BooleanShare for T where T: SharedValue + BooleanOps + Serializable {}
