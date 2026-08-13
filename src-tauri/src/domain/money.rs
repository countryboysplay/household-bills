use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

/// Money is always stored as integer cents.
///
/// The financial engine must never use floating point values for authoritative
/// calculations. A value of `18_422` represents `$184.22`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Money(pub i64);

impl Money {
    pub const ZERO: Self = Self(0);

    pub const fn cents(cents: i64) -> Self {
        Self(cents)
    }

    pub const fn value(self) -> i64 {
        self.0
    }

    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    pub fn max(self, rhs: Self) -> Self {
        Self(self.0.max(rhs.0))
    }

    pub fn min(self, rhs: Self) -> Self {
        Self(self.0.min(rhs.0))
    }

    pub fn abs(self) -> Self {
        Self(self.0.saturating_abs())
    }

    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub const fn is_positive(self) -> bool {
        self.0 > 0
    }
}

impl Add for Money {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl Sub for Money {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}
impl Neg for Money {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_integer_cents_without_float_error() {
        assert_eq!((Money::cents(18_422) + Money::cents(9_000)).value(), 27_422);
    }

    #[test]
    fn spending_subtracts_cleanly() {
        assert_eq!((Money::cents(100_000) - Money::cents(8_647)).value(), 91_353);
    }

    #[test]
    fn helpers_keep_money_integer_only() {
        assert_eq!(Money::cents(-500).abs(), Money::cents(500));
        assert_eq!(Money::cents(100).min(Money::cents(50)), Money::cents(50));
        assert_eq!(Money::cents(100).max(Money::cents(150)), Money::cents(150));
    }
}
