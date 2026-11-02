//! Bounded original-curve point witnesses. Search is never a unique-root or
//! exact-intersection certificate; saved complete child/lift gates remain next.
use super::*;
use nurbs_core::{
    curve_distance,
    interval_eval::{self, Interval as I},
};
fn spend(max: usize, cells: &mut usize) -> Result<()> {
    allowance(max, *cells)?;
    *cells += 1;
    Ok(())
}
fn witness_upper(
    curve: &Curve,
    parameter: f64,
    point: &[f64],
    max: usize,
    cells: &mut usize,
) -> Result<f64> {
    spend(max, cells)?;
    let enclosed = interval_eval::evaluate_interval(curve, I::point(parameter))?;
    let mut upper = I::point(0.);
    for (coordinate, &value) in enclosed.iter().zip(point) {
        if coordinate.lo == value && coordinate.hi == value {
            continue;
        }
        let delta = coordinate.sub(I::point(value))?;
        upper = upper.add(I::point(delta.lo.abs().max(delta.hi.abs())))?;
    }
    Ok(upper.hi)
}
pub(super) fn cuts(
    sources: &[Surface],
    patch: usize,
    side: usize,
    curve: &Curve,
    tolerance: f64,
    max: usize,
    cells: &mut usize,
) -> Result<Vec<Cut>> {
    let d = curve.domain();
    let first = &curve.control_points[0];
    let last = curve.control_points.last().unwrap();
    let mut points = BTreeMap::<[u64; 3], (usize, usize)>::new();
    for (other, s) in sources.iter().enumerate() {
        if other == patch {
            continue;
        }
        for i in 0..4 {
            let p = corner(s, i);
            if p == first.as_slice() || p == last.as_slice() {
                continue;
            }
            let key = std::array::from_fn(|k| if p[k] == 0. { 0 } else { p[k].to_bits() });
            points.entry(key).or_insert((other, i));
        }
    }
    let mut candidates = BTreeMap::<Q, Cut>::new();
    for (_, (other, i)) in points {
        let p = corner(&sources[other], i);
        let start = *cells;
        spend(max, cells)?;
        // Positive rational basis supplies an original control-hull exclusion.
        let mut outside = false;
        for k in 0..3 {
            let lo = curve
                .control_points
                .iter()
                .map(|v| v[k])
                .fold(f64::INFINITY, f64::min);
            let hi = curve
                .control_points
                .iter()
                .map(|v| v[k])
                .fold(f64::NEG_INFINITY, f64::max);
            if p[k] < I::point(lo).sub(I::point(tolerance))?.lo
                || p[k] > I::point(hi).add(I::point(tolerance))?.hi
            {
                outside = true;
                break;
            }
        }
        if outside {
            continue;
        }
        let point = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![p.to_vec(); 2],
            weights: vec![1.; 2],
            periodic: false,
        };
        let separation = curve_distance::prove_clearance(
            curve,
            &point,
            tolerance * 2.,
            allowance(max, *cells)?,
        )?;
        *cells += separation.cells;
        if separation.distance_bounds[0] > tolerance {
            continue;
        }
        if separation.status == curve_distance::ClearanceStatus::Unresolved {
            return Err(refused(
                "Original curved T candidate clearance remains unresolved",
            ));
        }
        let nearest =
            curve_distance::nearest_point(curve, p, tolerance * 0.125, allowance(max, *cells)?)?;
        *cells += nearest.cells;
        if nearest.distance_interval[0] > tolerance {
            continue;
        }
        let parameter = nearest.parameter;
        if parameter <= d[0] || parameter >= d[1] {
            return Err(refused(
                "Curved T witness does not isolate an interior stored parameter",
            ));
        }
        let upper = witness_upper(curve, parameter, p, max, cells)?;
        if upper > tolerance {
            return Err(refused(
                "Original curved T parameter witness remains unproven",
            ));
        }
        let exact = q(parameter);
        if let Some(previous) = candidates.get(&exact) {
            if corner(&sources[previous.witness_patch], previous.witness_corner) != p {
                return Err(refused(
                    "Distinct curved T vertices collide in a stored parameter",
                ));
            }
        } else {
            candidates.insert(
                exact.clone(),
                Cut {
                    kind: CutKind::CurvedWitness,
                    patch,
                    side,
                    parameter: exact,
                    stored_parameter: parameter,
                    witness_patch: other,
                    witness_corner: i,
                    source_distance_upper: upper,
                    discovery_cells: *cells - start,
                },
            );
        }
    }
    if candidates.len() > 63 {
        return Err(refused(
            "Original curved boundary needs more than 63 T cuts",
        ));
    }
    Ok(candidates.into_values().collect())
}
