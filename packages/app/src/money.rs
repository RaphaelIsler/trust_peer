pub fn calculate_wealth(
    start_time: f64,
    initial_wealth: f64,
    linear_income_rate: f64,   // income per unit time
    interest_rate: f64,        // e.g. 0.05 for 5%
    target_time: f64,
) -> f64 {
    let delta_t = target_time - start_time;

    if delta_t <= 0.0 {
        return initial_wealth;
    }

    // Handle zero interest separately to avoid division by zero
    if interest_rate.abs() < f64::EPSILON {
        return initial_wealth + linear_income_rate * delta_t;
    }

    let exp_term = (interest_rate * delta_t).exp();

    initial_wealth * exp_term
        + (linear_income_rate / interest_rate) * (exp_term - 1.0)
}

pub fn required_interest_rate(
    initial_wealth: f64,
    linear_income_rate: f64,
    time: f64,
    zins_faktor: f64, // k = Faktor wie stark Zins den Zufluss ergänzt
) -> Option<f64> {
    if time <= 0.0 || linear_income_rate <= 0.0 {
        return None;
    }

    // Zielvermögen: linearer Zufluss + Faktor*linearer Zufluss
    let target = initial_wealth + linear_income_rate * time * (1.0 + zins_faktor);

    // Funktion f(r) = calculate_wealth(...) - target
    let f = |r: f64| -> f64 {
        calculate_wealth(
            0.0,                  // start_time = 0, wir arbeiten mit delta_t = time
            initial_wealth,
            linear_income_rate,
            r,
            time
        ) - target
    };

    let mut low = 0.0;
    let mut high = 0.1;

    // Obergrenze hochskalieren bis f(high) > 0
    while f(high) < 0.0 {
        high *= 2.0;
        if high > 1000.0 {
            return None; // keine Lösung in realistischem Bereich
        }
    }

    // Binary Search
    for _ in 0..100 {
        let mid = (low + high) / 2.0;
        if f(mid) > 0.0 {
            high = mid;
        } else {
            low = mid;
        }
    }

    Some((low + high) / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_wealth() {
        let start_time = 0.0;
        let initial_wealth = 1000.0;
        let linear_income_rate = 50.0;
        let interest_rate = 0.05;
        let target_time = 10.0;

        let wealth = calculate_wealth(start_time, initial_wealth, linear_income_rate, interest_rate, target_time);
        assert!(wealth > initial_wealth);
    }

    #[test]
    fn test_required_interest_rate() {
        let initial_wealth = 1.0;
        let linear_income_rate = 4000.0 / 3600.0;
        let time = 60.0*60.0*24.0*90.0; // 3 Month

        let rate = required_interest_rate(initial_wealth, linear_income_rate, time, 1.0).unwrap();
        let sum = calculate_wealth(0.0, 0.0, linear_income_rate, rate, time);
        let mal_rate = sum * rate;
        println!("Required interest rate: {:.9}%, summe bis dann; {}, mal rate: {}, income {}", rate * 100.0, sum, mal_rate, linear_income_rate);
        assert!(rate > 0.0);

    }
}