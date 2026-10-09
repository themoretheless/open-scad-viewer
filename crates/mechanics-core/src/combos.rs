//! Load combinations and envelopes over solver responses, not a design code.
//!
//! A [`Combination`] is a named row of factors, one per load case; a combined
//! response is the linear combination Σ factorᵢ · responseᵢ, which is exact
//! because every solver here is linear static. An envelope tracks the minimum
//! and maximum of each scalar across combinations together with the governing
//! combination index. The crate supplies no code-mandated factors (1.35G+1.5Q
//! and the like): naming and factoring are the caller's engineering decision.
use crate::{Error, Result};

/// Matches the response budget of `print_strength::screen`.
pub const MAX_LOAD_CASES: usize = 32;
pub const MAX_COMBINATIONS: usize = 64;
const MAX_NAME: usize = 128;

#[derive(Clone, Debug, PartialEq)]
pub struct Combination {
    pub name: String,
    /// One factor per load case, positional. Zero factors are admitted.
    pub factors: Vec<f64>,
}

/// Minimum and maximum of one scalar across combinations.
/// Ties keep the earliest combination (deterministic ordering).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinMax {
    pub min: f64,
    pub max: f64,
    pub min_combination: usize,
    pub max_combination: usize,
}

impl MinMax {
    pub fn of(value: f64, combination: usize) -> Self {
        MinMax {
            min: value,
            max: value,
            min_combination: combination,
            max_combination: combination,
        }
    }

    pub fn absorb(&mut self, value: f64, combination: usize) {
        if value < self.min {
            self.min = value;
            self.min_combination = combination;
        }
        if value > self.max {
            self.max = value;
            self.max_combination = combination;
        }
    }

    /// Range of the envelope; zero when all combinations agree.
    pub fn spread(&self) -> f64 {
        self.max - self.min
    }
}

fn invalid(message: &str) -> Error {
    Error::new("COMBO_INVALID_INPUT", message)
}

/// Validate combination rows against the number of solved load cases.
pub fn validate(combinations: &[Combination], load_cases: usize) -> Result<()> {
    if load_cases == 0 || load_cases > MAX_LOAD_CASES {
        return Err(invalid("Provide 1-32 load cases"));
    }
    if combinations.is_empty() || combinations.len() > MAX_COMBINATIONS {
        return Err(invalid("Provide 1-64 load combinations"));
    }
    for combination in combinations {
        if combination.name.trim().is_empty() || combination.name.len() > MAX_NAME {
            return Err(invalid("Combination names must be 1-128 characters"));
        }
        if combination.factors.len() != load_cases
            || combination.factors.iter().any(|f| !f.is_finite())
        {
            return Err(invalid(
                "Each combination needs one finite factor per load case",
            ));
        }
    }
    Ok(())
}

/// Linear combination of per-case scalar series: out[e] = Σᵢ factorsᵢ · casesᵢ[e].
/// Every series must have the same length as the first.
pub fn combine_series(case_series: &[&[f64]], factors: &[f64]) -> Result<Vec<f64>> {
    if case_series.len() != factors.len() || case_series.is_empty() {
        return Err(invalid("Combine needs one factor per case series"));
    }
    let len = case_series[0].len();
    if case_series.iter().any(|s| s.len() != len) {
        return Err(invalid("Case series must share one length"));
    }
    let mut out = vec![0.; len];
    for (series, &factor) in case_series.iter().zip(factors) {
        if factor == 0. {
            continue;
        }
        for (o, &v) in out.iter_mut().zip(series.iter()) {
            *o += factor * v;
        }
    }
    if out.iter().any(|v| !v.is_finite()) {
        return Err(Error::new(
            "COMBO_NUMERIC_RANGE",
            "Combination exceeds finite numeric range",
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combos() -> Vec<Combination> {
        vec![
            Combination {
                name: "G".into(),
                factors: vec![1., 0.],
            },
            Combination {
                name: "1.35G+1.5Q".into(),
                factors: vec![1.35, 1.5],
            },
        ]
    }

    #[test]
    fn validation_bounds_names_factors_and_counts() {
        assert!(validate(&combos(), 2).is_ok());
        assert!(validate(&combos(), 0).is_err());
        assert!(validate(&combos(), 3).is_err());
        assert!(validate(&[], 2).is_err());
        let mut bad = combos();
        bad[0].factors[1] = f64::NAN;
        assert!(validate(&bad, 2).is_err());
        let mut bad = combos();
        bad[0].name = "  ".into();
        assert!(validate(&bad, 2).is_err());
        let many = vec![
            Combination {
                name: "c".into(),
                factors: vec![1.],
            };
            MAX_COMBINATIONS + 1
        ];
        assert!(validate(&many, 1).is_err());
    }

    #[test]
    fn combine_series_is_linear_and_skips_zero_factors() {
        let a = [1., -2., 3.];
        let b = [10., 20., 30.];
        let out = combine_series(&[&a, &b], &[1.35, 1.5]).unwrap();
        assert_eq!(out, vec![16.35, 27.3, 49.05]);
        let out = combine_series(&[&a, &b], &[0., 2.]).unwrap();
        assert_eq!(out, vec![20., 40., 60.]);
        assert!(combine_series(&[&a], &[1., 1.]).is_err());
        assert!(combine_series(&[&a, &b[..2]], &[1., 1.]).is_err());
        assert!(combine_series(&[&[f64::MAX], &[f64::MAX]], &[1., 1.]).is_err());
    }

    #[test]
    fn minmax_envelope_keeps_governing_combination_and_first_tie() {
        let mut e = MinMax::of(5., 0);
        e.absorb(-3., 1);
        e.absorb(9., 2);
        e.absorb(9., 3);
        assert_eq!(
            e,
            MinMax {
                min: -3.,
                max: 9.,
                min_combination: 1,
                max_combination: 2,
            }
        );
        assert_eq!(e.spread(), 12.);
    }
}
