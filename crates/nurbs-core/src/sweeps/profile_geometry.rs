//! Independent whole-surface guarantees. Deviation never implies these proofs.
//! A periodic V chart is lifted to (radius/height, unwrapped angle). A global
//! contraction on its infinite periodic strip proves injectivity modulo V.
use crate::{Result, check, distance_bounds::Interval as I, surface::Surface};
use value_codec::{Value, json};

fn exact_difference(a: f64, b: f64, expected: f64) -> bool {
    let difference = a - b;
    let virtual_b = a - difference;
    let virtual_a = difference + virtual_b;
    difference.is_finite() && difference == expected && (a - virtual_a) + (virtual_b - b) == 0.
}
pub(super) fn exact_periodic_basis(s: &Surface) -> bool {
    let p = s.degree_v;
    let count = s.control_points[0].len();
    if !s.periodic_v || count <= p || p < 1 {
        return false;
    }
    let n = count - p;
    let a = s.knots_v[p];
    let b = s.knots_v[count];
    let period = b - a;
    if !period.is_finite() || period <= 0. || !exact_difference(b, a, period) {
        return false;
    }
    // Exact subtraction is required; no epsilon establishes periodic jets.
    if (0..s.knots_v.len() - n).any(|i| !exact_difference(s.knots_v[i + n], s.knots_v[i], period)) {
        return false;
    }
    if (p..count).any(|i| s.knots_v[i] >= s.knots_v[i + 1]) {
        return false;
    }
    s.control_points
        .iter()
        .zip(&s.weights)
        .all(|(row, w)| (0..p).all(|i| row[i] == row[n + i] && w[i] == w[n + i]))
}
fn projected_jacobian(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
    axes: [usize; 3],
    origin: [f64; 3],
    radial: bool,
) -> Result<Option<[[I; 2]; 2]>> {
    use crate::surface_injectivity::{
        PolarPoly, polar_poly_add as add, polar_poly_derivative as derivative,
        polar_poly_mul as mul,
    };
    let mut net = crate::curve_surface_composition::surface_net_on(s, span, domain)?;
    for h in net.iter_mut().flatten() {
        for k in 0..3 {
            h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
        }
    }
    let component = |k: usize| -> PolarPoly {
        net.iter()
            .map(|row| row.iter().map(|h| h[k]).collect())
            .collect()
    };
    let [x, y, z] = axes;
    let hx = component(x);
    let hy = component(y);
    let hz = component(z);
    let w = component(3);
    let bounds = |p: &PolarPoly| -> I {
        I {
            lo: p
                .iter()
                .flatten()
                .map(|x| x.lo)
                .fold(f64::INFINITY, f64::min),
            hi: p
                .iter()
                .flatten()
                .map(|x| x.hi)
                .fold(f64::NEG_INFINITY, f64::max),
        }
    };
    let squared = add(&mul(&hx, &hx)?, &mul(&hy, &hy)?, 1.)?;
    let r2 = bounds(&squared);
    let weight = bounds(&w);
    if r2.lo <= 0. || weight.lo <= 0. {
        return Ok(None);
    }
    let mut jac = [[I::point(0.); 2]; 2];
    for axis in 0..2 {
        let dx = derivative(&hx, axis, domain)?;
        let dy = derivative(&hy, axis, domain)?;
        let dz = derivative(&hz, axis, domain)?;
        let dw = derivative(&w, axis, domain)?;
        let angle = add(&mul(&hx, &dy)?, &mul(&hy, &dx)?, -1.)?;
        jac[1][axis] = bounds(&angle).div(r2)?;
        jac[0][axis] = if radial {
            let ds = derivative(&squared, axis, domain)?;
            let numerator = add(&mul(&w, &ds)?, &mul(&squared, &dw)?, -2.)?;
            bounds(&numerator).div(weight.mul(weight)?.mul(weight)?)?
        } else {
            let numerator = add(&mul(&dz, &w)?, &mul(&hz, &dw)?, -1.)?;
            bounds(&numerator).div(weight.mul(weight)?)?
        };
    }
    Ok(Some(jac))
}
fn abs_upper(x: I) -> f64 {
    x.lo.abs().max(x.hi.abs())
}
struct Contraction {
    upper: f64,
    scale: f64,
}
fn contraction(j: [[I; 2]; 2]) -> Result<Option<Contraction>> {
    let m = j.map(|row| row.map(|x| x.lo / 2. + x.hi / 2.));
    let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
    if !det.is_finite() || det == 0. {
        return Ok(None);
    }
    let y = [
        [m[1][1] / det, -m[0][1] / det],
        [-m[1][0] / det, m[0][0] / det],
    ];
    if y.iter().flatten().any(|x| !x.is_finite()) {
        return Ok(None);
    }
    let mut residual = [[0_f64; 2]; 2];
    for r in 0..2 {
        for c in 0..2 {
            let product = I::point(y[r][0])
                .mul(j[0][c])?
                .add(I::point(y[r][1]).mul(j[1][c])?)?;
            residual[r][c] = abs_upper(I::point(if r == c { 1. } else { 0. }).sub(product)?);
        }
    }
    // In the norm max(|x0|,|x1|/t), the two verified row bounds are
    // B00+B01*t and B11+B10/t. A floating Perron vector is only a proposal:
    // both inequalities are checked again by outward-rounded arithmetic.
    let verify = |t: f64| -> Result<f64> {
        Ok(I::point(residual[0][0])
            .add(I::point(residual[0][1]).mul(I::point(t))?)?
            .hi
            .max(
                I::point(residual[1][1])
                    .add(I::point(residual[1][0]).div(I::point(t))?)?
                    .hi,
            ))
    };
    let mut result = Contraction {
        upper: verify(1.)?,
        scale: 1.,
    };
    if result.upper < 1. || residual[0][0] >= 1. || residual[1][1] >= 1. {
        return Ok(Some(result));
    }
    let [a, b] = residual[0];
    let [c, d] = residual[1];
    let off = b.sqrt() * c.sqrt();
    let rho = (a + d + (a - d).hypot(2. * off)) / 2.;
    let proposals = if b == 0. {
        vec![1. + 2. * c / (1. - d)]
    } else if c == 0. {
        vec![(1. - a) / (2. * b)]
    } else {
        vec![c / (rho - d), (rho - a) / b, c.sqrt() / b.sqrt()]
    };
    for t in proposals {
        if !t.is_finite() || t <= 0. {
            continue;
        }
        let Ok(upper) = verify(t) else {
            continue;
        };
        if upper < result.upper {
            result = Contraction { upper, scale: t };
        }
    }
    Ok(Some(result))
}

fn periodic(s: &Surface, max_cells: usize) -> Result<Value> {
    let mut work = 0usize;
    let failed = |work, reason| {
        json!({"certified":false,"cells":work,"reason":reason,
        "scope":"surface-modulo-periodic-v","method":"periodic-unwrapped-projection-contraction"})
    };
    if s.degree_u > 8 || s.degree_v > 8 || s.periodic_u || !exact_periodic_basis(s) {
        return Ok(failed(work, "exact-periodic-basis-unproved"));
    }
    // The lift must be C1 across interior U knots. V simple knots and exact
    // repeated data above guarantee C^(degreeV-1), including the closing seam.
    if s.degree_v < 2 {
        return Ok(failed(work, "periodic-c1-unproved"));
    }
    let count = s.control_points.len();
    let [ua, ub] = [s.knots_u[s.degree_u], s.knots_u[count]];
    if s.knots_u
        .iter()
        .filter(|&&t| ua < t && t < ub)
        .any(|t| s.knots_u.iter().filter(|k| *k == t).count() >= s.degree_u)
    {
        return Ok(failed(work, "interior-u-c1-unproved"));
    }
    let origin: [f64; 3] = std::array::from_fn(|k| {
        let lo = s
            .control_points
            .iter()
            .flatten()
            .map(|p| p[k])
            .fold(f64::INFINITY, f64::min);
        let hi = s
            .control_points
            .iter()
            .flatten()
            .map(|p| p[k])
            .fold(f64::NEG_INFINITY, f64::max);
        lo / 2. + hi / 2.
    });
    for [x, y, z] in [[0, 1, 2], [1, 2, 0], [2, 0, 1]] {
        for radial in [true, false] {
            for parts in [1usize, 2, 4, 8, 16, 32] {
                let mut global: Option<[[I; 2]; 2]> = None;
                let mut invalid = false;
                for u in s.degree_u..count {
                    if s.knots_u[u] == s.knots_u[u + 1] {
                        continue;
                    }
                    for v in s.degree_v..s.control_points[0].len() {
                        if s.knots_v[v] == s.knots_v[v + 1] {
                            continue;
                        }
                        for i in 0..parts {
                            for j in 0..parts {
                                if work == max_cells {
                                    return Ok(failed(work, "work-limit"));
                                }
                                work += 1;
                                let domain = std::array::from_fn(|axis| {
                                    let (k, span, index) = if axis == 0 {
                                        (&s.knots_u, u, i)
                                    } else {
                                        (&s.knots_v, v, j)
                                    };
                                    let lo = k[span];
                                    let hi = k[span + 1];
                                    [
                                        if index == 0 {
                                            lo
                                        } else {
                                            lo + (hi - lo) * index as f64 / parts as f64
                                        },
                                        if index + 1 == parts {
                                            hi
                                        } else {
                                            lo + (hi - lo) * (index + 1) as f64 / parts as f64
                                        },
                                    ]
                                });
                                let Some(jac) = projected_jacobian(
                                    s,
                                    [u, v],
                                    domain,
                                    [x, y, z],
                                    origin,
                                    radial,
                                )?
                                else {
                                    invalid = true;
                                    continue;
                                };
                                global = Some(match global {
                                    None => jac,
                                    Some(g) => std::array::from_fn(|r| {
                                        std::array::from_fn(|c| I {
                                            lo: g[r][c].lo.min(jac[r][c].lo),
                                            hi: g[r][c].hi.max(jac[r][c].hi),
                                        })
                                    }),
                                });
                            }
                        }
                    }
                }
                if invalid {
                    continue;
                }
                let Some(g) = global else {
                    continue;
                };
                let Some(q) = contraction(g)? else {
                    break;
                };
                let angular = g[1][1];
                let period = s.knots_v[s.control_points[0].len()] - s.knots_v[s.degree_v];
                let rotation_upper = I::point(abs_upper(angular)).mul(I::point(period))?.hi;
                // The lower literal bounds pi from below. Exact closed, nonzero
                // projections have integer winding; fixed derivative sign and
                // rotation < 4*pi force exactly one turn, with either orientation.
                if q.upper < 1.
                    && (angular.lo > 0. || angular.hi < 0.)
                    && rotation_upper < 4. * 3.141592653589793
                {
                    return Ok(
                        json!({"certified":true,"cells":work,"reason":"global-periodic-lift-contraction",
                        "scope":"surface-modulo-periodic-v","method":"periodic-unwrapped-projection-contraction",
                        "projectionAxes":[x,y],"firstCoordinate":if radial {"squared-radius"}else{"height"},
                        "origin":origin,"contractionUpper":q.upper,"normWeights":[1.,q.scale],"angularDerivative":[angular.lo,angular.hi],
                        "rotationUpper":rotation_upper,"absoluteWinding":1}),
                    );
                }
            }
        }
    }
    Ok(failed(work, "global-periodic-projection-unproved"))
}
pub(super) fn inspect(s: &Surface, max_cells: usize) -> Result<Value> {
    check(
        (1..=100000).contains(&max_cells),
        "Geometry needs 1..100000 cells per predicate",
    )?;
    let r = crate::surface_regularity::inspect(s, max_cells)?;
    let regularity = json!({"certified":r.spanwise_regular,"interiorBasisC1":r.interior_basis_c1,
        "cells":r.cells,"unresolvedCells":r.unresolved.len(),"scope":"all-original-knot-rectangles"});
    let embedding = if s.periodic_v {
        periodic(s, max_cells)?
    } else {
        let r = crate::surface_injectivity::certify(s, max_cells)?;
        json!({"certified":r.proven,"cells":r.spans,"reason":r.reason,
            "scope":"complete-untrimmed-surface","method":"global-projection-contraction",
            "contractionUpper":r.contraction_upper})
    };
    let certified = r.spanwise_regular && embedding["certified"] == json!(true);
    Ok(
        json!({"certified":certified,"regularity":regularity,"embedding":embedding,
        "maxCellsPerPredicate":max_cells,"solidTopologyCertified":false,
        "scope":"single-untrimmed-surface","pairwiseFaceContactsCertified":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Curve;
    fn ring(vertical: bool) -> Surface {
        let mut sections: Vec<_> = (0..16)
            .map(|i| {
                let t = std::f64::consts::TAU * i as f64 / 16.;
                let [x, y] = [t.cos(), t.sin()];
                Curve {
                    degree: 1,
                    knots: vec![0., 0., 1., 1.],
                    control_points: if vertical {
                        vec![vec![x, y, -0.1], vec![x, y, 0.1]]
                    } else {
                        vec![vec![x, y, 0.], vec![1.2 * x, 1.2 * y, 0.]]
                    },
                    weights: vec![1., 1.],
                    periodic: false,
                }
            })
            .collect();
        sections.push(sections[0].clone());
        super::super::profile_seam::build(&sections).unwrap().0
    }
    #[test]
    fn periodic_radial_and_height_projections_certify_complete_strips() {
        for vertical in [false, true] {
            let s = ring(vertical);
            let r = inspect(&s, 16384).unwrap();
            assert_eq!(r["certified"], true, "{r}");
            assert_eq!(r["embedding"]["absoluteWinding"], 1);
            assert_eq!(r["solidTopologyCertified"], false);
            let mut reflected = s.clone();
            for p in reflected.control_points.iter_mut().flatten() {
                p[0] *= -1.;
            }
            assert_eq!(inspect(&reflected, 16384).unwrap()["certified"], true);
        }
    }
    #[test]
    fn exact_two_cover_is_regular_but_must_not_be_globally_admitted() {
        let mut s = ring(false);
        let n = s.control_points[0].len() - s.degree_v;
        for row in &mut s.control_points {
            let period = row[..n].to_vec();
            *row = period.iter().cycle().take(2 * n).cloned().collect();
            row.extend_from_slice(&period[..3]);
        }
        for row in &mut s.weights {
            *row = vec![1.; 2 * n + 3];
        }
        s.knots_v = (0..2 * n + 7)
            .map(|i| (i as f64 - 3.) / (2 * n) as f64)
            .collect();
        s.validate().unwrap();
        assert!(
            crate::surface_regularity::inspect(&s, 16384)
                .unwrap()
                .spanwise_regular
        );
        assert_eq!(periodic(&s, 1024).unwrap()["certified"], false);
        let a = s.evaluate(0.37, 0.).unwrap().point;
        let b = s.evaluate(0.37, 0.5).unwrap().point;
        assert_eq!(a, b);
    }
    #[test]
    fn exhausted_work_and_one_ulp_periodic_knot_tampering_refuse() {
        let mut s = ring(false);
        let r = periodic(&s, 1).unwrap();
        assert_eq!(r["certified"], false);
        assert_eq!(r["cells"], 1);
        s.knots_v[0] = s.knots_v[0].next_up();
        assert!(!exact_periodic_basis(&s));
        assert_eq!(periodic(&s, 16384).unwrap()["certified"], false);
    }
}

#[cfg(test)]
mod weighted_norm_tests {
    use super::*;
    #[test]
    fn weighted_norm_proves_skewed_jacobian_without_relaxing_the_bound() {
        let j = [
            [I::new(0.9, 1.1).unwrap(), I::new(-10., 10.).unwrap()],
            [I::new(-0.001, 0.001).unwrap(), I::new(0.9, 1.1).unwrap()],
        ];
        let proof = contraction(j).unwrap().unwrap();
        assert!(proof.upper < 0.21);
        assert!(proof.scale > 0. && proof.scale < 1.);
        let unsafe_j = [
            [I::new(0.5, 1.5).unwrap(), I::new(-1., 1.).unwrap()],
            [I::new(-1., 1.).unwrap(), I::new(0.5, 1.5).unwrap()],
        ];
        assert!(contraction(unsafe_j).unwrap().unwrap().upper >= 1.);
        assert!(
            contraction([[I::new(-1., 1.).unwrap(); 2]; 2])
                .unwrap()
                .is_none()
        );
    }
}
