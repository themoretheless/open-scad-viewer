//! Exact period translations. Rounded enclosures are used only for evaluation;
//! membership uses the exact lifted rational endpoints, never their f64 hulls.
use super::*;
use num_rational::BigRational as Q;
use num_traits::ToPrimitive;
fn q(x: f64) -> Result<Q> {
    Q::from_float(x).ok_or_else(|| crate::input("Nonfinite quotient coordinate"))
}
fn outward(x: &Q) -> Result<[f64; 2]> {
    let f = x
        .to_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| crate::input("Quotient coordinate overflow"))?;
    let exact = q(f)?;
    Ok([
        if exact > *x { f.next_down() } else { f },
        if exact < *x { f.next_up() } else { f },
    ])
}
pub(super) fn natural(s: &Surface, axis: usize) -> [f64; 2] {
    if axis == 0 {
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]]
    } else {
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]]
    }
}
fn periodic(s: &Surface, axis: usize) -> bool {
    if axis == 0 {
        s.periodic_u
    } else {
        s.periodic_v
    }
}
pub(super) fn validate_source(source: &FiberSources) -> Result<()> {
    for s in [source.first(), source.second()] {
        s.validate()?;
        check(
            s.control_points.iter().flatten().all(|p| p.len() == 3)
                && s.control_points
                    .len()
                    .checked_mul(s.control_points[0].len())
                    .is_some_and(|n| n <= 4096),
            "Quotient fibers need bounded original spatial carriers",
        )?;
        // Surface::validate establishes C0 internal knot multiplicities.
        check(
            s.degree_u > 0 && s.degree_v > 0,
            "Quotient source must have positive degrees",
        )?;
        for axis in 0..2 {
            let (knots, p, n) = if axis == 0 {
                (&s.knots_u, s.degree_u, s.control_points.len())
            } else {
                (&s.knots_v, s.degree_v, s.control_points[0].len())
            };
            check(
                knots[p] < knots[p + 1] && (periodic(s, axis) || knots[n - 1] < knots[n]),
                "Original natural endpoint evaluator span is unproved",
            )?;
            if periodic(s, axis) {
                check(
                    crate::periodic_trim::verify_surface_seam_identity(s, axis)?.is_some(),
                    "Original periodic seam identity is unproved",
                )?;
            }
        }
    }
    Ok(())
}
pub(super) fn validate(source: &FiberSources, d: [[f64; 2]; 4], free: usize) -> Result<()> {
    validate_source(source)?;
    for k in 0..4 {
        check(
            d[k].iter().all(|v| v.is_finite())
                && if k == free {
                    d[k][0] <= d[k][1]
                } else {
                    d[k][0] < d[k][1]
                },
            "Invalid quotient chart interval",
        )?;
        let s = if k < 2 {
            source.first()
        } else {
            source.second()
        };
        let n = natural(s, k % 2);
        if periodic(s, k % 2) {
            let period = q(n[1])? - q(n[0])?;
            check(
                q(d[k][1])? - q(d[k][0])? < period
                    && q(d[k][0])? >= q(n[0])? - &period
                    && q(d[k][1])? <= q(n[1])? + period,
                "Quotient chart must cover less than one period within neighboring lifts",
            )?;
        } else {
            let (knots, p, count) = if k % 2 == 0 {
                (&s.knots_u, s.degree_u, s.control_points.len())
            } else {
                (&s.knots_v, s.degree_v, s.control_points[0].len())
            };
            check(
                !((d[k][0] == n[0] && knots[p] == knots[p + 1])
                    || (d[k][1] == n[1] && knots[count - 1] == knots[count])),
                "Original quotient endpoint evaluator span is unproved",
            )?;
            check(
                n[0] <= d[k][0] && d[k][1] <= n[1],
                "Nonperiodic quotient coordinate leaves original domain",
            )?;
        }
    }
    Ok(())
}
pub(super) fn sections(s: &Surface, axis: usize, d: [f64; 2]) -> Result<Vec<[f64; 2]>> {
    let n = natural(s, axis);
    if !periodic(s, axis) {
        return Ok(vec![d]);
    }
    let period = q(n[1])? - q(n[0])?;
    let mut pieces = Vec::new();
    for shift in -1..=1 {
        let delta = &period * Q::from_integer(shift.into());
        let a = (q(d[0])? - &delta).max(q(n[0])?);
        let b = (q(d[1])? - &delta).min(q(n[1])?);
        if a <= b {
            pieces.push([outward(&a)?[0].max(n[0]), outward(&b)?[1].min(n[1])]);
        }
    }
    check(!pieces.is_empty(), "Empty original quotient section")?;
    Ok(pieces)
}
pub(super) fn contains(
    source: &FiberSources,
    lift: [[f64; 2]; 4],
    original: [[f64; 2]; 4],
) -> Result<bool> {
    for k in 0..4 {
        let s = if k < 2 {
            source.first()
        } else {
            source.second()
        };
        let n = natural(s, k % 2);
        let d = original[k];
        check(
            d.iter().all(|v| v.is_finite()) && n[0] <= d[0] && d[0] <= d[1] && d[1] <= n[1],
            "Original membership box outside carrier",
        )?;
        // Finite binary64 values are exact binary rationals. Without a deck
        // translation their order is already exact; no rational arithmetic or
        // rounded period subtraction is needed for this common census path.
        if !periodic(s, k % 2) {
            if lift[k][0] <= d[0] && d[1] <= lift[k][1] {
                continue;
            }
            return Ok(false);
        }
        let period = q(n[1])? - q(n[0])?;
        let mut inside = false;
        for shift in -1..=1 {
            if shift != 0 && !periodic(s, k % 2) {
                continue;
            }
            let delta = &period * Q::from_integer(shift.into());
            inside |= q(lift[k][0])? <= q(d[0])? + &delta && q(d[1])? + delta <= q(lift[k][1])?;
        }
        if !inside {
            return Ok(false);
        }
    }
    Ok(true)
}
