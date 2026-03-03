use core::fmt;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(any(feature = "desktop", feature = "mobile"))]
use chrono::{Local, TimeZone};
#[cfg(any(feature = "desktop", feature = "mobile"))]
use dioxus::prelude::*;

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct Timestamp(u64);

impl Timestamp {
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn now() -> Self {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self(secs)
    }

    #[cfg(any(feature = "desktop", feature = "mobile"))]
    pub fn to_local_string(self) -> String {
        Local
            .timestamp_opt(self.0 as i64, 0)
            .single()
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| self.0.to_string())
    }
}

impl From<u64> for Timestamp {
    fn from(value: u64) -> Self {
        Self::from_raw(value)
    }
}

impl From<Timestamp> for u64 {
    fn from(value: Timestamp) -> Self {
        value.raw()
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add<u64> for Timestamp {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0.saturating_add(rhs))
    }
}

impl Sub<u64> for Timestamp {
    type Output = Self;

    fn sub(self, rhs: u64) -> Self::Output {
        Self(self.0.saturating_sub(rhs))
    }
}

impl Sub for Timestamp {
    type Output = u64;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0.saturating_sub(rhs.0)
    }
}

impl Mul<u64> for Timestamp {
    type Output = Self;

    fn mul(self, rhs: u64) -> Self::Output {
        Self(self.0.saturating_mul(rhs))
    }
}

impl Div<u64> for Timestamp {
    type Output = Self;

    fn div(self, rhs: u64) -> Self::Output {
        if rhs == 0 {
            return Self(0);
        }
        Self(self.0 / rhs)
    }
}

impl AddAssign<u64> for Timestamp {
    fn add_assign(&mut self, rhs: u64) {
        self.0 = self.0.saturating_add(rhs);
    }
}

impl SubAssign<u64> for Timestamp {
    fn sub_assign(&mut self, rhs: u64) {
        self.0 = self.0.saturating_sub(rhs);
    }
}

impl MulAssign<u64> for Timestamp {
    fn mul_assign(&mut self, rhs: u64) {
        self.0 = self.0.saturating_mul(rhs);
    }
}

impl DivAssign<u64> for Timestamp {
    fn div_assign(&mut self, rhs: u64) {
        if rhs == 0 {
            self.0 = 0;
        } else {
            self.0 /= rhs;
        }
    }
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
#[component]
pub fn TimestampView(timestamp: Timestamp) -> Element {
    let text = timestamp.to_local_string();
    rsx! {
        span { "{text}" }
    }
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
#[component]
pub fn TimestampU64View(value: u64) -> Element {
    let text = Timestamp::from_raw(value).to_local_string();
    rsx! {
        span { "{text}" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_roundtrip() {
        let ts = Timestamp::from_raw(1_700_000_000);
        assert_eq!(ts.raw(), 1_700_000_000);
        let raw: u64 = ts.into();
        assert_eq!(raw, 1_700_000_000);
    }

    #[test]
    fn test_timestamp_now() {
        let now = Timestamp::now();
        assert!(now.raw() > 0);
    }

    #[test]
    fn test_timestamp_ops() {
        let ts = Timestamp::from_raw(10);
        assert_eq!((ts + 5).raw(), 15);
        assert_eq!((ts - 3).raw(), 7);
        assert_eq!((ts * 2).raw(), 20);
        assert_eq!((ts / 2).raw(), 5);
        assert_eq!(ts - Timestamp::from_raw(4), 6);
    }
}
