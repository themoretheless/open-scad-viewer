//! Canonical exact endpoint ownership, preserving every certified flank pole.
use super::*;
use nurbs_core::interval_eval::Interval as I;
/// A positive rational basis is a partition of unity, so the largest changed
/// pole displacement bounds the entire original curve. Use outward L1 distance
/// as a conservative Euclidean bound without trusting platform hypot accuracy.
fn movement(a: &[f64], b: &[f64]) -> Result<f64> {
    if a == b {
        return Ok(0.);
    }
    let mut distance = I::point(0.);
    for (&a, &b) in a.iter().zip(b) {
        let d = I::point(a).sub(I::point(b))?;
        distance = distance.add(I::point(d.lo.abs().max(d.hi.abs())))?;
    }
    Ok(distance.hi)
}
pub(super) fn close(wires: &mut [Vec<Curve>], budget: f64) -> Result<f64> {
    let mut maximum = 0_f64;
    for wire in wires {
        let original = wire.clone();
        let n = wire.len();
        for i in 0..n {
            let next = (i + 1) % n;
            let a = original[i].control_points.last().unwrap();
            let b = &original[next].control_points[0];
            // Flanks already carry an inclusive nominal certificate. They own
            // both joins; change only the neighboring line or rational arc.
            if original[i].degree == 5 && original[next].degree == 5 && a != b {
                return Err(invalid(
                    "Distinct certified flank endpoints cannot be moved by profile closure",
                ));
            }
            let canonical = if original[i].degree == 5 { a } else { b };
            let da = movement(a, canonical)?;
            let db = movement(b, canonical)?;
            maximum = maximum.max(da).max(db);
            if maximum > budget {
                return Err(invalid(
                    "Gear endpoint ownership exceeds its continuous profile adjustment budget",
                ));
            }
            *wire[i].control_points.last_mut().unwrap() = canonical.clone();
            wire[next].control_points[0] = canonical.clone();
        }
    }
    Ok(maximum)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_flank_pole_is_unchanged_and_movement_bounds_complete_curves() {
        let flank = Curve {
            degree: 5,
            knots: vec![0., 0., 0., 0., 0., 0., 1., 1., 1., 1., 1., 1.],
            control_points: (0..=5).map(|i| vec![i as f64 / 5., 0.]).collect(),
            weights: vec![1.; 6],
            periodic: false,
        };
        let epsilon = 1e-9;
        let mut wires = vec![vec![
            flank,
            line2([1. + epsilon, 0.], [1., 1.]),
            line2([1., 1. + epsilon], [0., 1.]),
            line2([0., 1.], [0., epsilon]),
        ]];
        // Nonuniform positive rational line parameterization must not weaken
        // the whole-image movement bound or change original weights.
        wires[0][1].weights = vec![1., 2.];
        let original = wires.clone();
        let bound = close(&mut wires, 1e-7).unwrap();
        assert_eq!(wires[0][0], original[0][0]);
        assert!(bound >= epsilon && bound < 2. * epsilon);
        for (before, after) in original[0].iter().zip(&wires[0]) {
            assert_eq!(before.weights, after.weights);
            assert_eq!(before.knots, after.knots);
            for j in 0..=100 {
                let u = j as f64 / 100.;
                let a = before.evaluate(u).unwrap().point;
                let b = after.evaluate(u).unwrap().point;
                assert!((a[0] - b[0]).hypot(a[1] - b[1]) <= bound + 1e-15);
            }
        }
        assert!(close(&mut original.clone(), 1e-12).is_err());
    }
}
