//! A NEW patch-network proposal, never a certificate for the input surface.
//! Equal denominators make the V blend Cartesian. Separate original U cells
//! retain independent homogeneous gauges; welding and G2 remain fresh gates.
use super::*;

fn choose(n: usize, k: usize) -> f64 {
    (0..k).fold(1u64, |v, i| v * (n - i) as u64 / (i + 1) as u64) as f64
}
fn product(a: &[f64], b: &[f64]) -> Vec<f64> {
    let p = a.len() - 1;
    let q = b.len() - 1;
    (0..=p + q)
        .map(|k| {
            (0..=p)
                .filter(|&i| i <= k && k - i <= q)
                .map(|i| choose(p, i) * choose(q, k - i) / choose(p + q, k) * a[i] * b[k - i])
                .sum()
        })
        .collect()
}

#[cfg(test)]
/// Construct only from saturated, continuous original source cells. No knot
/// insertion, sampling, or fitted source surrogate enters this proposal.
fn propose(source: &Surface, operations: usize) -> Result<Vec<Surface>> {
    propose_authored(source, operations, None)
}
fn propose_authored(
    source: &Surface,
    operations: usize,
    authored: Option<&layers::AuthoredRows>,
) -> Result<Vec<Surface>> {
    source.validate()?;
    if let Some(a) = authored {
        for c in a.base.iter().chain(&a.field) {
            c.validate()?;
            require(
                c.degree == source.degree_u
                    && c.knots == source.knots_u
                    && c.control_points.len() == source.control_points.len(),
                "Authored endpoint field lost its source basis",
            )?;
        }
    }
    let p = source.degree_u;
    let n = source.control_points.len();
    let d = [source.knots_u[p], source.knots_u[n]];
    require(
        !source.periodic_u
            && !source.periodic_v
            && p > 0
            && p * 2 <= 16
            && source.degree_v == 5
            && source.control_points[0].len() == 6
            && source.control_points[0][0].len() == 3
            && source.knots_v == [vec![0.; 6], vec![1.; 6]].concat()
            && source.knots_u[..=p].iter().all(|&x| x == d[0])
            && source.knots_u[n..].iter().all(|&x| x == d[1]),
        "Common-denominator network requires clamped six-row original cells",
    )?;
    let mut work = Work {
        used: 0,
        max: operations,
    };
    for &t in source.knots_u.iter().filter(|&&t| d[0] < t && t < d[1]) {
        work.charge(1)?;
        require(
            source.knots_u.iter().filter(|&&x| x == t).count() == p,
            "Common-denominator network requires saturated original knots",
        )?;
    }
    for w in &source.weights {
        work.charge(4)?;
        require(
            w[0] == w[1] && w[1] == w[2] && w[3] == w[4] && w[4] == w[5],
            "Common-denominator network requires two literal row gauges",
        )?;
    }
    let mut patches = Vec::new();
    for span in p..n {
        let a = source.knots_u[span];
        let b = source.knots_u[span + 1];
        if a == b {
            continue;
        }
        let start = span - p;
        let wa: Vec<_> = (start..=span).map(|i| source.weights[i][0]).collect();
        let wb: Vec<_> = (start..=span).map(|i| source.weights[i][5]).collect();
        let weights = product(&wa, &wb);
        let top = weights.iter().copied().fold(0., f64::max);
        let exponent = ((top.to_bits() >> 52) & 0x7ff) as i32 - 1023;
        let gauge = 2f64.powi(-exponent);
        require(
            top.is_finite() && top > 0. && gauge.is_finite() && gauge > 0.,
            "Common-denominator network gauge underflow",
        )?;
        let q = p * 2;
        let mut points = vec![vec![vec![0.; 3]; 6]; q + 1];
        for row in 0..6 {
            let (own, other) = if row < 3 { (&wa, &wb) } else { (&wb, &wa) };
            // Center original numerators to avoid multiplying world offsets
            // by tiny endpoint weights. The actual rounded result is NEW.
            let origin = &source.control_points[start][row];
            for axis in 0..3 {
                let h: Vec<_> = (0..=p)
                    .map(|i| (source.control_points[start + i][row][axis] - origin[axis]) * own[i])
                    .collect();
                let numerator = product(&h, other);
                for k in 0..=q {
                    points[k][row][axis] = origin[axis] + numerator[k] / weights[k];
                }
            }
            // Literal shared source roots remain owned, not recomputed by a
            // multiply/divide round trip. All subsequent proofs use this net.
            points[0][row] = source.control_points[start][row].clone();
            points[q][row] = source.control_points[span][row].clone();
            // A Bernstein product preserves literal constant coordinate jets.
            // Keep these authored support-plane constraints instead of a
            // multiply/divide roundoff amplified by tiny endpoint weights.
            for axis in 0..3 {
                for (ids, output) in [
                    ([start, start + 1, start + 2], [0, 1, 2]),
                    ([span, span - 1, span - 2], [q, q - 1, q - 2]),
                ] {
                    let value = source.control_points[ids[0]][row][axis];
                    if ids
                        .iter()
                        .all(|&i| source.control_points[i][row][axis] == value)
                    {
                        for i in output {
                            points[i][row][axis] = value;
                        }
                    }
                }
            }
            work.charge((p + 1) * (p + 1) * 4 + (q + 1) * 3)?;
        }
        endpoint_jets::restore_authored(
            source,
            start,
            span,
            &mut points,
            &weights,
            &mut work,
            authored,
        )?;
        let patch = Surface {
            degree_u: q,
            degree_v: 5,
            knots_u: [vec![a; q + 1], vec![b; q + 1]].concat(),
            knots_v: source.knots_v.clone(),
            control_points: points,
            weights: weights.iter().map(|w| vec![w * gauge; 6]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        patch.validate()?;
        patches.push(patch);
    }
    require(!patches.is_empty(), "Empty common-denominator network")?;
    Ok(patches)
}
impl CentralLoftCandidate<'_, '_, '_> {
    /// Unqualified alternative geometry. This does NOT preserve the original
    /// surface interior and conveys no radius, G2, curvature or sewing receipt.
    pub fn propose_common_denominator_patches(&self, operations: usize) -> Result<Vec<Surface>> {
        propose_authored(&self.surface, operations, Some(&self.authored))
    }
    /// A separate unqualified coupled-jet candidate. The operation budget is
    /// shared conservatively between construction and coupling. No source/G2
    /// or closed-body authority is inherited from either input proposal.
    pub fn propose_fair_patches(&self, operations: usize, relaxation: f64) -> Result<Vec<Surface>> {
        let part = operations / 4;
        let mut base = propose_authored(&self.surface, part, Some(&self.authored))?;
        endpoint_jets::reconcile(
            &mut base,
            self.options.tolerances[0] / 4.,
            &mut Work { used: 0, max: part },
        )?;
        let mut patches = interior_fairing::propose(
            &base,
            relaxation,
            self.stock,
            &mut Work { used: 0, max: part },
        )?;
        endpoint_jets::reconcile(
            &mut patches,
            self.options.tolerances[0] / 4.,
            &mut Work {
                used: 0,
                max: operations - 3 * part,
            },
        )?;
        for s in &patches {
            for p in s.control_points.iter().flatten() {
                for k in 0..3 {
                    require(
                        self.stock[k][0] <= p[k] && p[k] <= self.stock[k][1],
                        "Coupled free interior leaves original stock",
                    )?;
                }
            }
        }
        Ok(patches)
    }
    /// Sampled two-parameter bending proposal with literal boundary two-jets.
    /// No sampled score or iteration count grants curvature or body admission.
    pub fn propose_bivariate_patches(
        &self,
        operations: usize,
        relaxation: f64,
    ) -> Result<Vec<Surface>> {
        let base = self.propose_fair_patches(operations / 2, 1.)?;
        bivariate_fairing::propose(
            &base,
            relaxation,
            self.stock,
            &mut Work {
                used: 0,
                max: operations - operations / 2,
            },
        )
    }
    /// Approximate sampled-curvature search. No score admits a surface or body.
    /// `evaluations` is a shared source-evaluation allowance across all patches.
    pub fn propose_curvature_search_patches(
        &self,
        operations: usize,
        evaluations: usize,
        sweeps: usize,
    ) -> Result<Vec<Surface>> {
        require(
            evaluations <= 100000 && operations >= evaluations,
            "Bound curvature proposal work",
        )?;
        let base = self.propose_bivariate_patches(operations - evaluations, 1.)?;
        curvature_search::propose(
            &base,
            sweeps,
            self.stock,
            &mut Work {
                used: 0,
                max: evaluations,
            },
        )
    }
    /// Joint internal curve/neighbour proposal; original exterior two-jet poles
    /// remain fixed. Only independent original field checks can admit its G2/cap.
    pub fn propose_joint_seam_patches(
        &self,
        operations: usize,
        evaluations: usize,
        sweeps: usize,
    ) -> Result<Vec<Surface>> {
        let construction = operations / 2;
        require(
            evaluations <= 100000 && evaluations <= operations - construction,
            "Bound joint seam work",
        )?;
        let base = self.propose_fair_patches(construction, 1.)?;
        joint_seam_search::propose(
            &base,
            sweeps,
            self.stock,
            &mut Work {
                used: 0,
                max: evaluations,
            },
            &mut Work {
                used: 0,
                max: operations - construction - evaluations,
            },
        )
    }
    /// Degree-nine joint proposal with four free V rows. Fresh original G2,
    /// field and solid checks remain mandatory; no admission is implied.
    pub fn propose_joint_seam_degree9_patches(
        &self,
        operations: usize,
        evaluations: usize,
        sweeps: usize,
    ) -> Result<Vec<Surface>> {
        let construction = operations / 2;
        require(
            evaluations <= 100000 && evaluations <= operations - construction,
            "Bound degree-nine joint work",
        )?;
        let base = self.propose_fair_patches(construction / 2, 1.)?;
        let mut work = Work {
            used: 0,
            max: construction - construction / 2,
        };
        let mut base = interior_fairing::elevate_v(&base, 9, &mut work)?;
        endpoint_jets::reconcile(&mut base, self.options.tolerances[0] / 4., &mut work)?;
        joint_seam_search::propose(
            &base,
            sweeps,
            self.stock,
            &mut Work {
                used: 0,
                max: evaluations,
            },
            &mut Work {
                used: 0,
                max: operations - construction - evaluations,
            },
        )
    }
    /// Separate degree-5 coupled proposal; no body or curvature authority.
    pub fn propose_reconciled_patches(&self, operations: usize) -> Result<Vec<Surface>> {
        let mut patches = propose_authored(&self.surface, operations / 2, Some(&self.authored))?;
        endpoint_jets::reconcile(
            &mut patches,
            self.options.tolerances[0] / 4.,
            &mut Work {
                used: 0,
                max: operations - operations / 2,
            },
        )?;
        Ok(patches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 5,
            knots_u: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            knots_v: [vec![0.; 6], vec![1.; 6]].concat(),
            control_points: (0..5)
                .map(|i| {
                    (0..6)
                        .map(|j| {
                            vec![
                                i as f64 / 4.,
                                j as f64 / 5.,
                                if j < 3 { i as f64 / 8. } else { 0. },
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: (0..5)
                .map(|i| vec![1. + i as f64, 1. + i as f64, 1. + i as f64, 3., 3., 3.])
                .collect(),
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn new_network_preserves_owned_roots_and_each_boundary_row_image() {
        let s = source();
        let before = s.clone();
        let patches = propose(&s, 100000).unwrap();
        assert_eq!(patches.len(), 2);
        assert_eq!(
            patches[0].control_points.last(),
            patches[1].control_points.first()
        );
        for (cell, p) in patches.iter().enumerate() {
            assert!(p.weights.iter().all(|w| w.iter().all(|x| *x == w[0])));
            for row in 0..6 {
                let original = Curve {
                    degree: 2,
                    knots: vec![0., 0., 0., 1., 1., 1.],
                    control_points: (0..3)
                        .map(|i| s.control_points[cell * 2 + i][row].clone())
                        .collect(),
                    weights: (0..3).map(|i| s.weights[cell * 2 + i][row]).collect(),
                    periodic: false,
                };
                let c = Curve {
                    degree: p.degree_u,
                    knots: p.knots_u.clone(),
                    control_points: p.control_points.iter().map(|v| v[row].clone()).collect(),
                    weights: p.weights.iter().map(|v| v[row]).collect(),
                    periodic: false,
                };
                // Regression only; sample agreement is not a whole-curve proof.
                for t in [0., 0.125, 0.5, 0.875, 1.] {
                    let a = original.evaluate(t).unwrap();
                    let b = c.evaluate((cell as f64 + t) * 0.5).unwrap();
                    for k in 0..3 {
                        assert!((a.point[k] - b.point[k]).abs() < 2e-14);
                    }
                }
            }
        }
        assert_eq!(s, before);
    }
    #[test]
    fn compensated_endpoint_product_retains_full_original_boundary_images() {
        let poles = [[0.1, 0.1], [0.1 - 1e-10, 0.1], [0., 0.05], [0., 0.]];
        let w = [2f64.powi(-30), 0.25, 0.125, 2f64.powi(-28)];
        let s = Surface {
            degree_u: 3,
            degree_v: 5,
            knots_u: [vec![0.; 4], vec![1.; 4]].concat(),
            knots_v: [vec![0.; 6], vec![1.; 6]].concat(),
            control_points: poles
                .iter()
                .map(|p| (0..6).map(|j| vec![p[0], p[1], j as f64 / 5.]).collect())
                .collect(),
            weights: w.iter().map(|w| vec![*w, *w, *w, 1., 1., 1.]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let before = s.clone();
        let net = propose(&s, 1000000).unwrap();
        assert_eq!(net.len(), 1);
        let p = &net[0];
        assert_eq!(p.degree_u, 6);
        for row in [0, 5] {
            let c = Curve {
                degree: p.degree_u,
                knots: p.knots_u.clone(),
                control_points: p.control_points.iter().map(|p| p[row].clone()).collect(),
                weights: p.weights.iter().map(|w| w[row]).collect(),
                periodic: false,
            };
            let uv =
                Curve::from_polyline(vec![vec![0., row as f64 / 5.], vec![1., row as f64 / 5.]])
                    .unwrap();
            let r = agreement::verify(&c, &uv, &s, false, 1e-7, 10000).unwrap();
            assert_eq!(r.status, agreement::Status::WithinTolerance, "{r:?}");
            assert_eq!(p.control_points[0][row], s.control_points[0][row]);
            assert_eq!(p.control_points[6][row], s.control_points[3][row]);
        }
        assert_eq!(s, before);
    }
    #[test]
    fn source_and_budget_guards_cannot_return_a_partial_network() {
        let s = source();
        assert!(propose(&s, 0).is_err());
        assert!(propose(&s, 100).is_err());
        let mut flat = s.clone();
        for row in &mut flat.control_points {
            for point in row {
                point.pop();
            }
        }
        assert!(propose(&flat, 100000).is_err());
        let mut bad = s.clone();
        bad.weights[1][1] += 1e-12;
        assert!(propose(&bad, 100000).is_err());
        bad = s.clone();
        bad.knots_u.remove(3);
        bad.control_points.remove(1);
        bad.weights.remove(1);
        assert!(propose(&bad, 100000).is_err());
    }
}
