//! Heckbert's "Nice Numbers" algorithm for graph axis labeling.
//!
//! Given a data range [data_min, data_max] and a desired number of tick marks,
//! computes a "nice" axis range and tick interval where ticks fall on round numbers
//! (multiples of 1, 2, or 5 × 10^n).

/// Result of the nice numbers calculation.
#[derive(Debug, Clone, PartialEq)]
pub struct NiceScale {
    /// Nice minimum (≤ data_min)
    pub min: f64,
    /// Nice maximum (≥ data_max)
    pub max: f64,
    /// Tick interval (a "nice" number)
    pub tick_spacing: f64,
    /// Number of ticks (inclusive of min and max)
    pub tick_count: usize,
}

impl NiceScale {
    /// Generate tick values from min to max.
    pub fn tick_values(&self) -> Vec<f64> {
        let mut ticks = Vec::with_capacity(self.tick_count);
        let mut val = self.min;
        for _ in 0..self.tick_count {
            ticks.push(val);
            val += self.tick_spacing;
        }
        ticks
    }
}

/// Compute a nice scale for the given data range.
///
/// `max_ticks` is the desired maximum number of ticks (typically 5-7).
/// Returns a NiceScale with rounded min, max, and tick spacing.
pub fn nice_scale(data_min: f64, data_max: f64, max_ticks: usize) -> NiceScale {
    // Handle edge cases
    if data_min >= data_max || max_ticks < 2 {
        // Single value or invalid range
        let center = if data_min.is_finite() { data_min } else { 0.0 };
        return NiceScale {
            min: center - 1.0,
            max: center + 1.0,
            tick_spacing: 1.0,
            tick_count: 3,
        };
    }

    let range = nice_num(data_max - data_min, false);
    let tick_spacing = nice_num(range / (max_ticks - 1) as f64, true);

    let nice_min = (data_min / tick_spacing).floor() * tick_spacing;
    let nice_max = (data_max / tick_spacing).ceil() * tick_spacing;

    let tick_count = ((nice_max - nice_min) / tick_spacing).round() as usize + 1;

    NiceScale {
        min: nice_min,
        max: nice_max,
        tick_spacing,
        tick_count,
    }
}

/// Find a "nice" number approximately equal to `x`.
///
/// If `round` is true, round to the nearest nice number.
/// If `round` is false, take the ceiling (for range calculation).
fn nice_num(x: f64, round: bool) -> f64 {
    if x <= 0.0 || !x.is_finite() {
        return 1.0;
    }

    let exponent = x.log10().floor();
    let fraction = x / 10.0_f64.powf(exponent);

    let nice_fraction = if round {
        // Round to nearest nice number
        if fraction < 1.5 {
            1.0
        } else if fraction < 3.0 {
            2.0
        } else if fraction < 7.0 {
            5.0
        } else {
            10.0
        }
    } else {
        // Ceiling nice number (for range)
        if fraction <= 1.0 {
            1.0
        } else if fraction <= 2.0 {
            2.0
        } else if fraction <= 5.0 {
            5.0
        } else {
            10.0
        }
    };

    nice_fraction * 10.0_f64.powf(exponent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nice_scale_basic() {
        // revenue data: 800 to 2000
        let scale = nice_scale(800.0, 2000.0, 5);
        // min should be ≤ 800 and a round number
        assert!(scale.min <= 800.0, "min {} should be ≤ 800", scale.min);
        // max should be ≥ 2000 and a round number
        assert!(scale.max >= 2000.0, "max {} should be ≥ 2000", scale.max);
        // tick_spacing should be a nice number (multiple of 1, 2, or 5)
        assert!(is_nice_spacing(scale.tick_spacing));
        // All ticks should be round numbers
        for tick in scale.tick_values() {
            assert!(is_round_number(tick), "tick {} is not a round number", tick);
        }
    }

    #[test]
    fn test_nice_scale_small_range() {
        // e.g., scores 78 to 95
        let scale = nice_scale(78.0, 95.0, 5);
        assert!(scale.min <= 78.0);
        assert!(scale.max >= 95.0);
        assert!(is_nice_spacing(scale.tick_spacing));
    }

    #[test]
    fn test_nice_scale_large_numbers() {
        // population: 800_000 to 14_000_000
        let scale = nice_scale(800_000.0, 14_000_000.0, 5);
        assert!(scale.min <= 800_000.0);
        assert!(scale.max >= 14_000_000.0);
        assert!(is_nice_spacing(scale.tick_spacing));
        for tick in scale.tick_values() {
            assert!(is_round_number(tick), "tick {} is not round", tick);
        }
    }

    #[test]
    fn test_nice_scale_zero_crossing() {
        // data: -15 to 25
        let scale = nice_scale(-15.0, 25.0, 5);
        assert!(scale.min <= -15.0);
        assert!(scale.max >= 25.0);
        // Zero should be one of the ticks
        let ticks = scale.tick_values();
        let has_zero = ticks.iter().any(|t| t.abs() < f64::EPSILON);
        assert!(has_zero, "Zero-crossing axis should include 0: {:?}", ticks);
    }

    #[test]
    fn test_nice_scale_tiny_range() {
        // e.g., percentages: 0.12 to 0.18
        let scale = nice_scale(0.12, 0.18, 5);
        assert!(scale.min <= 0.12);
        assert!(scale.max >= 0.18);
        assert!(scale.tick_spacing > 0.0);
    }

    #[test]
    fn test_nice_scale_same_values() {
        // All data is 5.0
        let scale = nice_scale(5.0, 5.0, 5);
        // Should still produce a valid range
        assert!(scale.min < scale.max);
        assert!(scale.tick_count >= 2);
    }

    #[test]
    fn test_nice_scale_from_zero() {
        // Bar chart: 0 to 4200
        let scale = nice_scale(0.0, 4200.0, 5);
        // min should be exactly 0 for data starting at 0
        assert!(
            scale.min.abs() < f64::EPSILON,
            "min should be 0, got {}",
            scale.min
        );
        assert!(scale.max >= 4200.0);
        assert!(is_nice_spacing(scale.tick_spacing));
    }

    #[test]
    fn test_nice_scale_tick_count_reasonable() {
        let scale = nice_scale(0.0, 100.0, 5);
        // Should produce between 3 and 8 ticks (close to requested 5)
        assert!(
            scale.tick_count >= 3 && scale.tick_count <= 8,
            "tick_count {} out of reasonable range",
            scale.tick_count
        );
    }

    #[test]
    fn test_nice_num_round() {
        // 12 → should round to 10
        assert_eq!(nice_num(12.0, true), 10.0);
        // 17 → should round to 20
        assert_eq!(nice_num(17.0, true), 20.0);
        // 35 → should round to 50
        assert_eq!(nice_num(35.0, true), 50.0);
        // 75 → should round to 100
        assert_eq!(nice_num(75.0, true), 100.0);
    }

    #[test]
    fn test_nice_num_ceil() {
        // 12 → should ceil to 20
        assert_eq!(nice_num(12.0, false), 20.0);
        // 35 → should ceil to 50
        assert_eq!(nice_num(35.0, false), 50.0);
        // 5.1 → should ceil to 10
        assert_eq!(nice_num(5.1, false), 10.0);
    }

    #[test]
    fn test_nice_scale_sales_data() {
        // The actual sales.csv data: revenue 800, 1500, 1200, 800, 2000, 1800
        let scale = nice_scale(800.0, 2000.0, 5);
        let ticks = scale.tick_values();
        // Ticks should be something like 500, 1000, 1500, 2000 or 800, 1000, 1200, ...
        // The important thing: all ticks are multiples of tick_spacing
        for (i, tick) in ticks.iter().enumerate() {
            let expected = scale.min + i as f64 * scale.tick_spacing;
            assert!(
                (tick - expected).abs() < 1e-10,
                "tick[{}] = {}, expected {}",
                i,
                tick,
                expected
            );
        }
    }

    /// Helper: check if spacing is a nice number (1, 2, or 5 × 10^n)
    fn is_nice_spacing(spacing: f64) -> bool {
        if spacing <= 0.0 {
            return false;
        }
        let exponent = spacing.log10().floor();
        let mantissa = spacing / 10.0_f64.powf(exponent);
        // Should be approximately 1, 2, or 5
        let nice_mantissas = [1.0, 2.0, 5.0, 10.0];
        nice_mantissas.iter().any(|&n| (mantissa - n).abs() < 1e-10)
    }

    /// Helper: check if a value is "round" (no messy trailing decimals)
    fn is_round_number(val: f64) -> bool {
        if val == 0.0 {
            return true;
        }
        // A round number: when formatted, shouldn't have more than 2 significant digits
        // after removing trailing zeros
        let abs = val.abs();
        if abs >= 1.0 {
            // Integer or clean decimal
            let formatted = format!("{}", val);
            // Should not have long decimal tails
            !formatted.contains('.') || formatted.split('.').nth(1).unwrap_or("").len() <= 2
        } else {
            // Small number: just check it's not irrational-looking
            let scaled = abs / 10.0_f64.powf(abs.log10().floor());
            (scaled - scaled.round()).abs() < 1e-10
        }
    }
}
