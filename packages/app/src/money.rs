use core_types::{Timestamp, Q32_32};
#[cfg(any(feature = "desktop", feature = "mobile"))]
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(any(feature = "desktop", feature = "mobile"))]
use tokio::time::{sleep, Duration};

///80% after 3 months
const DECAY_RATE_Q32_32: Q32_32 = Q32_32::from_f64(0.00075);

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    amount: Q32_32,
    timestamp: Timestamp,
    decay_rate: u32,  // the _32 of Q32_32
    income_rate: u32, // the 32_ of Q32_32 for income
}

impl Default for Money {
    fn default() -> Self {
        Self {
            amount: Q32_32::from_u64(0), // Default initial amount
            timestamp: Timestamp::now(), // Default to current time
            decay_rate: DECAY_RATE_Q32_32.get_decimal_part(), // Default decay rate
            income_rate: 200,            // No linear income by default
        }
    }
}

/// Calculate wealth using Q32.32 fixed-point (u64) and no_std-friendly math.
/// - time is in seconds (Q32.32)
/// - linear_income_rate is income per second (Q32.32)
/// - interest_rate is continuous rate per second (Q32.32)
impl Money {
    pub fn timestamp(self) -> Timestamp {
        self.timestamp
    }

    pub fn is_projection_of(self, previous: Money) -> bool {
        let expected = previous.on_time(self.timestamp);
        self == expected
    }

    pub fn on_time(self, target_time: Timestamp) -> Money {
        if target_time <= self.timestamp {
            return self.clone();
        }
        let diff = target_time - self.timestamp;
        let delta_t = Q32_32::from_u64(diff) / Q32_32::from_u64(3600);
        let decay_rate = Q32_32::from_decimal_part(self.decay_rate);
        let linear_income_rate = Q32_32::from_u64(self.income_rate as u64);

        if decay_rate.raw() == 0 {
            let linear_income = linear_income_rate * delta_t;
            return Self {
                amount: self.amount.saturating_add(linear_income),
                timestamp: target_time,
                decay_rate: self.decay_rate,
                income_rate: self.income_rate,
            };
        }

        let r_dt = decay_rate * delta_t;
        let exp_term = r_dt.exp();

        let part1 = self.amount * exp_term;
        let linear_over_r = linear_income_rate / decay_rate;
        let exp_minus_one = exp_term.saturating_sub(Q32_32::ONE);
        let part2 = linear_over_r * exp_minus_one;

        Self {
            amount: part1.saturating_add(part2),
            timestamp: target_time,
            decay_rate: self.decay_rate,
            income_rate: self.income_rate,
        }
    }
}

impl core::ops::Add for Money {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        if self.timestamp == rhs.timestamp {
            Self {
                amount: self.amount.saturating_add(rhs.amount),
                timestamp: self.timestamp,
                decay_rate: self.decay_rate,
                income_rate: self.income_rate + rhs.income_rate,
            }
        } else if self.timestamp > rhs.timestamp {
            let same_time_rhs = rhs.on_time(self.timestamp);
            Self {
                amount: self.amount.saturating_add(same_time_rhs.amount),
                timestamp: self.timestamp,
                decay_rate: self.decay_rate,
                income_rate: self.income_rate + same_time_rhs.income_rate,
            }
        } else {
            let same_time_self = self.on_time(rhs.timestamp);
            Self {
                amount: same_time_self.amount.saturating_add(rhs.amount),
                timestamp: rhs.timestamp,
                decay_rate: self.decay_rate,
                income_rate: self.income_rate + rhs.income_rate,
            }
        }
    }
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MoneyViewMode {
    Current,
    Stored,
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
#[component]
pub fn MoneyView(money: Money, shown_amount: MoneyViewMode) -> Element {
    let mut mode = use_signal(move || shown_amount);
    let mut current_amount = use_signal(move || money.on_time(Timestamp::now()).amount);

    use_effect(move || {
        mode.set(shown_amount);
    });

    use_future(move || async move {
        loop {
            sleep(Duration::from_secs(9)).await;

            if mode() == MoneyViewMode::Current {
                current_amount.set(money.on_time(Timestamp::now()).amount);
            }
        }
    });

    let shown_amount = match mode() {
        MoneyViewMode::Current => current_amount().to_u64_floor().to_string(),
        MoneyViewMode::Stored => money.amount.to_string(),
    };

    rsx! {
        span {
            class: "money-view",
            style: "display: inline-flex; gap: 8px; align-items: center;",
            select {
                value: if mode() == MoneyViewMode::Current { "current" } else { "stored" },
                onchange: move |event| {
                    if event.value() == "current" {
                        mode.set(MoneyViewMode::Current);
                        current_amount.set(money.on_time(Timestamp::now()).amount);
                    } else {
                        mode.set(MoneyViewMode::Stored);
                    }
                },
                option { value: "current", "aktuell" }
                option { value: "stored", "gespeichert" }
            }
            span { "{shown_amount}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_wealth_q32_32_no_std_linear_only() {
        let start_time = Q32_32::from_u64(0).raw();
        let initial_wealth = Q32_32::from_u64(10).raw();
        let linear_income_rate = Q32_32::from_u64(2).raw();
        let interest_rate = Q32_32::ZERO.raw();
        let target_time = Q32_32::from_u64(3).raw();

        let result = calculate_wealth_q32_32_no_std(
            start_time,
            initial_wealth,
            linear_income_rate,
            interest_rate,
            target_time,
        );

        let expected = Q32_32::from_u64(16).raw();
        assert_eq!(result, expected);
    }
}
