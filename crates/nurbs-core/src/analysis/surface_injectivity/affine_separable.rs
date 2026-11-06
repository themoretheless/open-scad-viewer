//! Exact fixed linear functionals extend the product-coordinate theorem.
//! A midpoint/chord proposes covectors only. Every projected pole must be
//! represented exactly, and the entire resulting scalar charts must receive
//! the existing source-owned strict monotonicity proof. This does not prove rank.
use super::*;
use crate::exact_curve_segments::{exact_add, exact_mul};
fn charge(work: &mut usize, max: usize) -> bool {
    if *work == max {
        false
    } else {
        *work += 1;
        true
    }
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[(k + 1) % 3] * b[(k + 2) % 3] - a[(k + 2) % 3] * b[(k + 1) % 3])
}
fn proposal(a: [f64; 3], resolution: usize) -> Option<[f64; 3]> {
    let scale = a.iter().map(|x| x.abs()).fold(0., f64::max);
    if !scale.is_finite() || scale == 0. {
        return None;
    }
    let row = a.map(|x| (resolution as f64 * (x / scale)).round());
    row.iter()
        .all(|x| x.is_finite() && x.abs() <= 64.)
        .then_some(row)
}
fn project(s: &Surface, rows: [[f64; 3]; 2], work: &mut usize, max: usize) -> Option<Surface> {
    let mut view = s.clone();
    for (p, q) in s
        .control_points
        .iter()
        .flatten()
        .zip(view.control_points.iter_mut().flatten())
    {
        for r in 0..2 {
            let mut sum = 0.;
            for k in 0..3 {
                if !charge(work, max) {
                    return None;
                }
                sum = exact_add(sum, exact_mul(p[k], rows[r][k])?)?;
            }
            q[r] = sum;
        }
        q[2] = 0.;
    }
    Some(view)
}
pub(super) fn certify(s: &Surface, work: &mut usize, max: usize) -> Result<Option<[[f64; 3]; 2]>> {
    let sizes = [s.control_points.len(), s.control_points[0].len()];
    if s.periodic_u
        || s.periodic_v
        || sizes != [s.degree_u + 1, s.degree_v + 1]
        || !(1..=32).contains(&s.degree_u)
        || !(1..=32).contains(&s.degree_v)
        || s.weights.iter().flatten().any(|w| *w != s.weights[0][0])
    {
        return Ok(None);
    }
    if !charge(work, max) {
        return Ok(None);
    }
    let mid = [
        0.5 * s.knots_u[s.degree_u] + 0.5 * s.knots_u[sizes[0]],
        0.5 * s.knots_v[s.degree_v] + 0.5 * s.knots_v[sizes[1]],
    ];
    // A numerical proposal failure preserves the previous unproved result.
    let Ok(sample) = s.evaluate(mid[0], mid[1]) else {
        return Ok(None);
    };
    let Some((du, dv)) = sample.first_derivatives() else {
        return Ok(None);
    };
    for axis in 0..2 {
        let (along, other) = if axis == 0 { (du, dv) } else { (dv, du) };
        let end = if axis == 0 {
            &s.control_points[sizes[0] - 1][0]
        } else {
            &s.control_points[0][sizes[1] - 1]
        };
        let chord = std::array::from_fn(|k| end[k] - s.control_points[0][0][k]);
        let candidates = [cross(along, other), cross(chord, along)];
        let mut previous = None;
        // Existing integer projection range, not a new geometric tolerance.
        for resolution in 1..=64 {
            if !charge(work, max) {
                return Ok(None);
            }
            let (Some(a), Some(b)) = (
                proposal(candidates[0], resolution),
                proposal(candidates[1], resolution),
            ) else {
                break;
            };
            let rows = if axis == 0 { [a, b] } else { [b, a] };
            if previous == Some(rows) {
                continue;
            }
            previous = Some(rows);
            let Some(view) = project(s, rows, work, max) else {
                if *work == max {
                    return Ok(None);
                }
                continue;
            };
            if let Some(axes) = super::separable_monotone::certify(&view, work, max)? {
                // The third proof-view coordinate is identically zero.
                if axes.iter().all(|&k| k < 2) {
                    return Ok(Some([rows[axes[0]], rows[axes[1]]]));
                }
            }
            if *work == max {
                return Ok(None);
            }
        }
    }
    Ok(None)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Surface {
        let m = [[1., 1., 1.], [1., 2., 1.], [1., 1., 2.]];
        let t = [8., -4., 2.];
        Surface {
            degree_u: 5,
            degree_v: 1,
            knots_u: [vec![0.; 6], vec![1.; 6]].concat(),
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..6)
                .map(|i| {
                    (0..2)
                        .map(|j| {
                            let p = [
                                if i % 2 == 0 { -10. } else { 10. },
                                2. * (i * i) as f64 - 9. * i as f64 + 7.5,
                                2. * j as f64,
                            ];
                            (0..3)
                                .map(|k| t[k] + (0..3).map(|l| m[k][l] * p[l]).sum::<f64>())
                                .collect()
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 6],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn mixed_original_coordinates_require_exact_full_chart_proof_and_shared_work() {
        let s = source();
        let before = s.clone();
        let mut used = 0;
        assert!(
            super::super::separable_monotone::certify(&s, &mut 0, 1000)
                .unwrap()
                .is_none()
        );
        let proof = certify(&s, &mut used, 1000).unwrap();
        assert!(proof.is_some(), "used={used}");
        assert!(used > 0 && used < 1000);
        let mut replay = 0;
        assert_eq!(certify(&s, &mut replay, used).unwrap(), proof);
        assert_eq!(replay, used);
        let mut short = 0;
        assert!(certify(&s, &mut short, used - 1).unwrap().is_none());
        assert_eq!(short, used - 1);
        let chart = certify_chart(&s, 1000).unwrap();
        assert!(chart.proven, "{chart:?}");
        assert_eq!(chart.reason, "global-exact-affine-separable-monotonicity");
        assert!(chart.contraction_upper.is_none());
        assert!(chart.linear_projection.is_some());
        assert!(!certify_contraction(&s, 1000).unwrap().proven);
        assert_eq!(s, before);
        let mut broken = s.clone();
        broken.control_points[2][1][0] = broken.control_points[2][1][0].next_up();
        assert!(certify(&broken, &mut 0, 1000).unwrap().is_none());
        let mut folded = s.clone();
        folded.control_points[5] = folded.control_points[0].clone();
        assert!(certify(&folded, &mut 0, 1000).unwrap().is_none());
    }
}
