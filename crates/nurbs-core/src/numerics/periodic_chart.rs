//! Conservative chart images for a periodic parameter interval. Integer
//! translations are evaluated as intervals, never as rounded new UV data.
use crate::{Result, check, distance_bounds::Interval};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Chart {
    pub range: Interval,
    /// Translation for this chart image; None means an aggregate of many wraps.
    pub shift: Option<Interval>,
}

pub(crate) fn charts(
    range: Interval,
    domain: [f64; 2],
    periodic: bool,
) -> Result<Option<Vec<Chart>>> {
    check(
        domain[0].is_finite() && domain[1].is_finite() && domain[0] < domain[1],
        "Periodic chart domain must be finite and increasing",
    )?;
    if range.lo >= domain[0] && range.hi <= domain[1] {
        return Ok(Some(vec![Chart {
            range,
            shift: Some(Interval::point(0.)),
        }]));
    }
    if !periodic {
        return Ok(None);
    }
    let period = Interval::point(domain[1]).sub(Interval::point(domain[0]))?;
    let quotient = range.sub(Interval::point(domain[0]))?.div(period)?;
    let first = quotient.lo.floor();
    let last = quotient.hi.floor();
    if first.abs() > 1_000_000. || last.abs() > 1_000_000. || last - first > 8. {
        // Many wraps cover at most the whole chart. Preserve all possible
        // images, but do not claim a common translation for composition.
        return Ok(Some(vec![Chart {
            range: Interval::new(domain[0], domain[1])?,
            shift: None,
        }]));
    }
    let mut out = Vec::new();
    for k in first as i64..=last as i64 {
        let shift = if k == 0 {
            Interval::point(0.)
        } else {
            period.mul(Interval::point(k as f64))?
        };
        let translated = if k == 0 { range } else { range.sub(shift)? };
        let lo = translated.lo.max(domain[0]);
        let hi = translated.hi.min(domain[1]);
        if lo <= hi {
            out.push(Chart {
                range: Interval::new(lo, hi)?,
                shift: Some(shift),
            });
        }
    }
    // Failure to cover a numerically marginal interval must never mean that
    // there is no geometry. Fall back to the entire periodic chart.
    if out.is_empty() {
        out.push(Chart {
            range: Interval::new(domain[0], domain[1])?,
            shift: None,
        });
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_domain() {
        let iv = Interval::new(0., 1.).unwrap();
        assert!(charts(iv, [10., 0.], true).is_err());
        assert!(charts(iv, [5., 5.], true).is_err());
        assert!(charts(iv, [f64::NAN, 1.], true).is_err());
    }

    #[test]
    fn wraps_correctly_around_periodic_domain() {
        let iv = Interval::new(5., 12.).unwrap();
        let res = charts(iv, [0., 10.], true).unwrap().unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].range, Interval::new(5., 10.).unwrap());
        assert_eq!(res[1].range.lo, 0.);
        assert!(res[1].range.hi >= 2. && res[1].range.hi <= 2. + 32. * f64::EPSILON);
        let shift = res[1].shift.unwrap();
        assert!(shift.lo <= 10. && shift.hi >= 10.);
    }
}
