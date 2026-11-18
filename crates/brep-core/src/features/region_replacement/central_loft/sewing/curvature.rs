//! Fresh curvature coverage of the actual trimmed sewn model, not its proposal.
use super::*;
use crate::face_curvature::{self, Decision, Limits, Report};
/// Absolute principal curvature in inverse millimetres. Not a radius-law test.
pub const SEWN_CORNER_CURVATURE_LIMIT: f64 = 10.;
pub struct CornerCurvatureFace {
    pub face: usize,
    pub report: Report,
}
pub struct CornerCurvatureAudit {
    pub faces: Vec<CornerCurvatureFace>,
    pub next_face: Option<usize>,
    pub cells: usize,
    pub domain_cells: usize,
    pub shape_cells: usize,
    pub continuity_cells: usize,
    pub exact_work: u64,
    pub continuity_faces: Vec<usize>,
    pub continuity_decisions: Vec<(usize, cad_predicates::BezierIdentity)>,
    pub complete: bool,
    pub within_limit: bool,
    pub one_sided_fields: bool,
    pub internal_g2_tolerances: Option<[f64; 2]>,
}
fn needs_exact_c2(s: &Surface) -> bool {
    [
        (s.degree_u, &s.knots_u, s.control_points.len()),
        (s.degree_v, &s.knots_v, s.control_points[0].len()),
    ]
    .iter()
    .any(|(p, k, n)| {
        k.iter()
            .any(|&u| u > k[*p] && u < k[*n] && k.iter().filter(|&&v| v == u).count() + 2 > *p)
    })
}
fn inspect(model: &Model, tolerance_uv: f64, limits: Limits) -> Result<CornerCurvatureAudit> {
    inspect_exact(model, tolerance_uv, limits, 0, 0)
}
fn inspect_exact(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    continuity_cells: usize,
    exact_work: u64,
) -> Result<CornerCurvatureAudit> {
    inspect_fields(
        model,
        tolerance_uv,
        limits,
        continuity_cells,
        exact_work,
        false,
    )
}
pub(super) fn inspect_one_sided_model(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
) -> Result<CornerCurvatureAudit> {
    inspect_fields(model, tolerance_uv, limits, 0, 0, true)
}
fn inspect_fields(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    continuity_cells: usize,
    exact_work: u64,
    one_sided: bool,
) -> Result<CornerCurvatureAudit> {
    require(
        continuity_cells <= 1000000 && exact_work <= 1000000,
        "Bound original sewn C2 coefficient work",
    )?;
    require(
        tolerance_uv.is_finite()
            && tolerance_uv > 0.
            && limits.cells <= 50000
            && limits.domain_cells <= 1000000
            && limits.shape_cells <= 1000000,
        "Bound sewn corner curvature work and UV tolerance",
    )?;
    model.validate()?;
    let mut out = CornerCurvatureAudit {
        faces: vec![],
        next_face: None,
        cells: 0,
        domain_cells: 0,
        shape_cells: 0,
        continuity_cells: 0,
        exact_work: 0,
        continuity_faces: vec![],
        continuity_decisions: vec![],
        complete: false,
        within_limit: false,
        one_sided_fields: one_sided,
        internal_g2_tolerances: None,
    };
    for face in 0..model.faces.len() {
        if out.cells >= limits.cells
            || out.domain_cells >= limits.domain_cells
            || out.shape_cells >= limits.shape_cells
        {
            out.next_face = Some(face);
            break;
        }
        let share = if one_sided {
            model.faces.len() - face
        } else {
            1
        };
        let budget = Limits {
            cells: (limits.cells - out.cells) / share,
            domain_cells: (limits.domain_cells - out.domain_cells) / share,
            shape_cells: (limits.shape_cells - out.shape_cells) / share,
        };
        let mut report = if one_sided {
            face_curvature::inspect_one_sided(
                model,
                face,
                SEWN_CORNER_CURVATURE_LIMIT,
                tolerance_uv,
                budget,
            )?
            .field
        } else {
            face_curvature::inspect(
                model,
                face,
                SEWN_CORNER_CURVATURE_LIMIT,
                tolerance_uv,
                budget,
                &[],
            )?
        };
        out.cells += report.cells;
        out.domain_cells += report.domain_cells;
        out.shape_cells += report.shape_cells;
        if report.reason == "continuity_not_proven"
            && out.continuity_cells < continuity_cells
            && out.exact_work < exact_work
        {
            // Reserve a share for every remaining low-continuity support;
            // the first difficult face must not consume all later proof work.
            let remaining = model.faces[face..]
                .iter()
                .filter(|f| needs_exact_c2(&f.surface))
                .count()
                .max(1);
            let continuity = nurbs_core::analysis::surface_continuity::qualify(
                &model.faces[face].surface,
                2,
                (continuity_cells - out.continuity_cells) / remaining,
                (exact_work - out.exact_work) / remaining as u64,
            )?;
            out.continuity_decisions.push((face, continuity.outcome));
            out.continuity_cells += continuity.cells;
            out.exact_work += continuity.exact_work;
            if let Some(proof) = continuity.certificate {
                out.continuity_faces.push(face);
                if out.cells < limits.cells
                    && out.domain_cells < limits.domain_cells
                    && out.shape_cells < limits.shape_cells
                {
                    report = face_curvature::inspect_with_continuity(
                        model,
                        face,
                        SEWN_CORNER_CURVATURE_LIMIT,
                        tolerance_uv,
                        Limits {
                            cells: limits.cells - out.cells,
                            domain_cells: limits.domain_cells - out.domain_cells,
                            shape_cells: limits.shape_cells - out.shape_cells,
                        },
                        &[],
                        &proof,
                    )?;
                    out.cells += report.cells;
                    out.domain_cells += report.domain_cells;
                    out.shape_cells += report.shape_cells;
                }
            }
        }
        out.faces.push(CornerCurvatureFace { face, report });
    }
    // The first pass reserves work for every face. Reuse its unspent shared
    // allowance on unresolved faces once; never promote a partial cover or
    // discard earlier cost. Already decisive faces are not recalculated.
    if one_sided && out.faces.len() == model.faces.len() {
        let pending: Vec<_> = out
            .faces
            .iter()
            .enumerate()
            .filter(|(_, f)| f.report.decision == Decision::Unresolved)
            .map(|(i, _)| i)
            .collect();
        for (attempt, &index) in pending.iter().enumerate() {
            if out.cells >= limits.cells
                || out.domain_cells >= limits.domain_cells
                || out.shape_cells >= limits.shape_cells
            {
                break;
            }
            let share = pending.len() - attempt;
            let mut report = face_curvature::inspect_one_sided(
                model,
                out.faces[index].face,
                SEWN_CORNER_CURVATURE_LIMIT,
                tolerance_uv,
                Limits {
                    cells: (limits.cells - out.cells) / share,
                    domain_cells: (limits.domain_cells - out.domain_cells) / share,
                    shape_cells: (limits.shape_cells - out.shape_cells) / share,
                },
            )?
            .field;
            out.cells += report.cells;
            out.domain_cells += report.domain_cells;
            out.shape_cells += report.shape_cells;
            // Only the new full attempt describes coverage; costs include both.
            report.cells += out.faces[index].report.cells;
            report.domain_cells += out.faces[index].report.domain_cells;
            report.shape_cells += out.faces[index].report.shape_cells;
            out.faces[index].report = report;
        }
    }
    out.complete = !model.faces.is_empty()
        && out.faces.len() == model.faces.len()
        && out
            .faces
            .iter()
            .all(|f| f.report.decision != Decision::Unresolved);
    out.within_limit = out.complete
        && out.faces.iter().all(|f| {
            f.report.decision == Decision::Within
                && f.report
                    .absolute_curvature_upper
                    .is_some_and(|k| k.is_finite() && k <= SEWN_CORNER_CURVATURE_LIMIT)
        });
    Ok(out)
}
impl SewnCornerCandidate {
    /// Covers saved face domains (including retained stock), with one shared budget.
    /// Does not grant G2, embedding, wall thickness, or a radius-law certificate.
    pub fn inspect_curvature_with_continuity(
        &self,
        tolerance_uv: f64,
        limits: Limits,
        continuity_cells: usize,
        exact_work: u64,
    ) -> Result<CornerCurvatureAudit> {
        inspect_exact(
            &self.model,
            tolerance_uv,
            limits,
            continuity_cells,
            exact_work,
        )
    }
    /// Conservative basis-only entry point without additional exact C2 work.
    pub fn inspect_curvature(
        &self,
        tolerance_uv: f64,
        limits: Limits,
    ) -> Result<CornerCurvatureAudit> {
        inspect(&self.model, tolerance_uv, limits)
    }
}
impl SewnCornerNetworkCandidate {
    /// Covers all saved network, strip, cap and retained stock face domains with
    /// one shared budget. Does not grant embedding, volume or radius-law admission.
    pub fn inspect_curvature_with_continuity(
        &self,
        tolerance_uv: f64,
        limits: Limits,
        continuity_cells: usize,
        exact_work: u64,
    ) -> Result<CornerCurvatureAudit> {
        inspect_exact(
            self.model(),
            tolerance_uv,
            limits,
            continuity_cells,
            exact_work,
        )
    }
    /// Conservative basis-only entry point without additional exact C2 work.
    pub fn inspect_curvature(
        &self,
        tolerance_uv: f64,
        limits: Limits,
    ) -> Result<CornerCurvatureAudit> {
        inspect(self.model(), tolerance_uv, limits)
    }
    /// One-sided fair-share curvature inspection across all 17 network, strip, cap, and stock faces.
    pub fn inspect_one_sided_curvature(
        &self,
        tolerance_uv: f64,
        limits: Limits,
    ) -> Result<CornerCurvatureAudit> {
        inspect_one_sided_model(self.model(), tolerance_uv, limits)
    }
}
/// Immutable body admission combining constructed-seam G2, embedding/material/volume
/// and full trimmed-face curvature coverage. Original sharp stock edges are retained.
/// Partial coverage cannot construct this admission.
pub struct QualifiedG2SewnCorner<'a> {
    body: QualifiedSewnCorner<'a>,
    curvature: CornerCurvatureAudit,
}
impl SewnCornerCandidate {
    pub fn qualify_g2(
        &self,
        tolerance_uv: f64,
        body_limits: CornerAuditLimits,
        curvature_limits: Limits,
    ) -> Result<QualifiedG2SewnCorner<'_>> {
        let curvature = self.inspect_curvature(tolerance_uv, curvature_limits)?;
        self.admit_g2_fields(tolerance_uv, body_limits, curvature)
    }
    /// Same body admission with separately bounded fresh original C2 proof work.
    pub fn qualify_g2_with_continuity(
        &self,
        tolerance_uv: f64,
        body_limits: CornerAuditLimits,
        curvature_limits: Limits,
        continuity_cells: usize,
        exact_work: u64,
    ) -> Result<QualifiedG2SewnCorner<'_>> {
        let curvature = self.inspect_curvature_with_continuity(
            tolerance_uv,
            curvature_limits,
            continuity_cells,
            exact_work,
        )?;
        self.admit_g2_fields(tolerance_uv, body_limits, curvature)
    }
    fn admit_g2_fields(
        &self,
        tolerance_uv: f64,
        body_limits: CornerAuditLimits,
        curvature: CornerCurvatureAudit,
    ) -> Result<QualifiedG2SewnCorner<'_>> {
        require(
            self.final_jets.len() >= 12
                && self.final_jets.iter().all(|r| {
                    r.decision == nurbs_core::surface_join::Decision::WithinTolerance
                        && r.regularity_proven
                        && r.pending == 0
                }),
            "Actual sewn G2 remains unproven",
        )?;
        if !curvature.within_limit {
            let exceeds = curvature
                .faces
                .iter()
                .any(|f| f.report.decision == Decision::Exceeds);
            return Err(nurbs_core::Error::new(
                if exceeds {
                    "BREP_SEWN_CURVATURE_LIMIT"
                } else {
                    "BREP_SEWN_CURVATURE_UNPROVEN"
                },
                if exceeds {
                    "Actual sewn face exceeds 10/mm"
                } else {
                    "Whole sewn face curvature coverage remains unproven"
                },
            ));
        }
        let body = self.qualify(tolerance_uv, body_limits)?;
        Ok(QualifiedG2SewnCorner { body, curvature })
    }
}
impl QualifiedG2SewnCorner<'_> {
    pub fn body(&self) -> &QualifiedSewnCorner<'_> {
        &self.body
    }
    pub fn curvature(&self) -> &CornerCurvatureAudit {
        &self.curvature
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> Limits {
        Limits {
            cells: 5000,
            domain_cells: 100000,
            shape_cells: 100000,
        }
    }
    #[test]
    fn all_saved_faces_covered_without_modifying_source() {
        let m = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let before = m.clone();
        let r = inspect(&m, 1e-8, limits()).unwrap();
        assert!(r.complete && r.within_limit);
        assert_eq!(r.faces.len(), 6);
        assert!(r.next_face.is_none());
        assert_eq!(m, before);
    }
    #[test]
    fn a_small_rational_cylinder_witness_cannot_be_admitted() {
        // This is a field diagnostic input, not a sewn-solid claim.
        let r = 1. / 64.;
        let w = 0.5f64.sqrt();
        let surface = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![r, 0., 0.], vec![r, 0., 1.]],
                vec![vec![r, r, 0.], vec![r, r, 1.]],
                vec![vec![0., r, 0.], vec![0., r, 1.]],
            ],
            weights: vec![vec![1.; 2], vec![w; 2], vec![1.; 2]],
            periodic_u: false,
            periodic_v: false,
        };
        let mut b = crate::region_replacement::corner_patch_body::Build::empty(1e-7);
        use nurbs_core::surface::Axis;
        let curves = [
            surface.iso(Axis::V, 0.).unwrap(),
            surface.iso(Axis::U, 1.).unwrap(),
            surface.iso(Axis::V, 1.).unwrap().reverse().unwrap(),
            surface.iso(Axis::U, 0.).unwrap().reverse().unwrap(),
        ];
        let nodes = curves
            .each_ref()
            .map(|c| b.node(c.control_points[0].clone().try_into().unwrap()));
        let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        let segments = curves
            .into_iter()
            .enumerate()
            .map(|(i, c)| {
                (
                    nodes[i],
                    nodes[(i + 1) % 4],
                    c,
                    Curve::from_polyline(vec![uv[i].to_vec(), uv[(i + 1) % 4].to_vec()]).unwrap(),
                )
            })
            .collect();
        let face = b.face(surface, segments).unwrap();
        b.m.shells.push(crate::Shell {
            faces: vec![crate::FaceUse {
                face,
                reversed: false,
            }],
            closed: false,
        });
        b.m.rebuild_topology_ids();
        let out = inspect(&b.m, 1e-8, limits()).unwrap();
        assert!(!out.within_limit);
        assert_eq!(out.faces[0].report.decision, Decision::Exceeds);
        assert!(out.faces[0].report.witness_curvature.unwrap()[0] > 10.);
    }
    #[test]
    fn exhausted_shared_budget_never_inherits_a_positive_face_decision() {
        let m = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let zero = inspect(
            &m,
            1e-8,
            Limits {
                cells: 0,
                ..limits()
            },
        )
        .unwrap();
        assert!(!zero.complete && !zero.within_limit);
        assert_eq!(zero.next_face, Some(0));
        assert!(zero.faces.is_empty());
        let first = inspect(&m, 1e-8, limits())
            .unwrap()
            .faces
            .remove(0)
            .report
            .cells;
        let partial = inspect(
            &m,
            1e-8,
            Limits {
                cells: first,
                ..limits()
            },
        )
        .unwrap();
        assert!(!partial.complete && !partial.within_limit);
        assert_eq!(partial.next_face, Some(1));
        assert_eq!(partial.cells, first);
        assert!(inspect(&m, f64::NAN, limits()).is_err());
    }
}
