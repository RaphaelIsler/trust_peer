use core::fmt;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize,
)]
pub struct Q32_32(u64);

impl Q32_32 {
    pub const FRAC_BITS: u32 = 32;
    pub const ONE: Self = Self(1u64 << 32);
    pub const ZERO: Self = Self(0);
    pub const LN2: Self = Self(2_977_044_471);

    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub const fn from_u64(value: u64) -> Self {
        match value.checked_shl(Self::FRAC_BITS) {
            Some(v) => Self(v),
            None => Self(u64::MAX),
        }
    }

    pub const fn to_u64_floor(self) -> u64 {
        self.0 >> Self::FRAC_BITS
    }

    pub const fn from_decimal_part(value: u32) -> Self {
        Self(value as u64)
    }

    pub const fn get_decimal_part(&self) -> u32 {
        (self.0 & 0xFFFF_FFFF) as u32
    }

    pub const fn from_f64(value: f64) -> Self {
        if !value.is_finite() || value <= 0.0 {
            return Self::ZERO;
        }
        let scaled = value * (1u64 << Self::FRAC_BITS) as f64;
        if scaled >= u64::MAX as f64 {
            return Self(u64::MAX);
        }
        Self(scaled.round() as u64)
    }

    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    pub fn mul(self, rhs: Self) -> Self {
        let prod = (self.0 as u128) * (rhs.0 as u128);
        let res = (prod + (1u128 << 31)) >> 32;
        if res > u64::MAX as u128 {
            Self(u64::MAX)
        } else {
            Self(res as u64)
        }
    }

    pub fn div(self, rhs: Self) -> Self {
        if rhs.0 == 0 {
            return Self(u64::MAX);
        }
        let res = ((self.0 as u128) << 32) / (rhs.0 as u128);
        if res > u64::MAX as u128 {
            Self(u64::MAX)
        } else {
            Self(res as u64)
        }
    }

    pub fn exp(self) -> Self {
        if self.0 == 0 {
            return Self::ONE;
        }

        let k = self.0 / Self::LN2.0;
        let r = self.0 - k * Self::LN2.0;

        let mut sum = Self::ONE;
        let mut term = Self::ONE;

        for i in 1..=12u64 {
            term = term.mul(Self(r));
            term.0 /= i;
            sum = sum.saturating_add(term);
            if term.0 == 0 {
                break;
            }
        }

        if k >= 32 {
            return Self(u64::MAX);
        }

        Self(sum.0.checked_shl(k as u32).unwrap_or(u64::MAX))
    }
}

impl From<u64> for Q32_32 {
    fn from(value: u64) -> Self {
        Self::from_u64(value)
    }
}

impl From<u32> for Q32_32 {
    fn from(value: u32) -> Self {
        Self::from_u64(value as u64)
    }
}

impl From<f64> for Q32_32 {
    fn from(value: f64) -> Self {
        Self::from_f64(value)
    }
}

impl From<Q32_32> for u64 {
    fn from(value: Q32_32) -> Self {
        value.raw()
    }
}

impl Add for Q32_32 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_add(rhs.0))
    }
}

impl Sub for Q32_32 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_sub(rhs.0))
    }
}

impl Mul for Q32_32 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.mul(rhs)
    }
}

impl Div for Q32_32 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self.div(rhs)
    }
}

impl AddAssign for Q32_32 {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_add(rhs.0);
    }
}

impl SubAssign for Q32_32 {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_sub(rhs.0);
    }
}

impl MulAssign for Q32_32 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = self.mul(rhs);
    }
}

impl DivAssign for Q32_32 {
    fn div_assign(&mut self, rhs: Self) {
        *self = self.div(rhs);
    }
}

impl fmt::Display for Q32_32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let int_part = self.to_u64_floor();
        let frac = self.0 & 0xFFFF_FFFF;
        let frac_dec = (((frac as u128) * 1_000_000u128) + (1u128 << 31)) >> 32;
        write!(f, "{}.{:06}", int_part, frac_dec as u64)
    }
}

#[cfg(feature = "sqlite")]
pub mod sqlite {
    use super::*;
    use sqlx::encode::IsNull;
    use sqlx::sqlite::SqliteArgumentValue;
    use sqlx::{sqlite, Decode, Encode, Sqlite};

    impl Decode<'_, Sqlite> for Q32_32 {
        fn decode(
            value: sqlx::sqlite::SqliteValueRef<'_>,
        ) -> Result<Self, sqlx::error::BoxDynError> {
            let raw = <i64 as Decode<Sqlite>>::decode(value)?;
            Ok(Self(raw as u64))
        }
    }

    impl<'q> Encode<'q, Sqlite> for Q32_32 {
        fn encode_by_ref(
            &self,
            args: &mut Vec<SqliteArgumentValue<'q>>,
        ) -> Result<IsNull, Box<dyn std::error::Error + Send + Sync + 'static>> {
            args.push(SqliteArgumentValue::Int64(self.0 as i64));
            Ok(IsNull::No)
        }
    }

    impl sqlx::Type<Sqlite> for Q32_32 {
        fn type_info() -> sqlite::SqliteTypeInfo {
            <i64 as sqlx::Type<Sqlite>>::type_info()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_q32_32_add_mul_div() {
        let one = Q32_32::ONE;
        let half = Q32_32::from_raw(1u64 << 31);
        let one_half = one + half;
        let two = Q32_32::from_u64(2);
        let three = Q32_32::from_u64(3);

        assert_eq!((one_half * two).raw(), three.raw());
        assert_eq!((three / two).raw(), one_half.raw());
    }

    #[test]
    fn test_q32_32_exp_ln2() {
        let two = Q32_32::from_u64(2);
        let exp_ln2 = Q32_32::LN2.exp();
        let diff = if exp_ln2.raw() > two.raw() {
            exp_ln2.raw() - two.raw()
        } else {
            two.raw() - exp_ln2.raw()
        };

        assert!(diff <= 50_000);
    }

    #[test]
    fn test_q32_32_display() {
        let one = Q32_32::ONE;
        assert_eq!(one.to_string(), "1.000000");
    }

    #[test]
    fn test_q32_32_from_f64() {
        let value = Q32_32::from_f64(0.5);
        assert_eq!(value.raw(), 1u64 << 31);
    }
}
