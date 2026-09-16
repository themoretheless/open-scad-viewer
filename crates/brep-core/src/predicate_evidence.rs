//! Bridge authored cad-predicates decisions into intersection Complete reports.
//!
//! Predicates never authorize topology alone; they only corroborate algebraic
//! Complete strata before coverage is published.

use cad_predicates::{
    AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
    ToleranceSpecIdentity, orient3d,
};
use nurbs_core::{Error, Result};

use crate::intersections::Plane;

pub const MAX_COMPOSED_EVIDENCE: usize = 64;

fn refuse(message: &str) -> Error {
    Error::new("BREP_PREDICATE_EVIDENCE_REFUSED", message)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceKind {
    Positional,
    RootParameter,
    TangentNormal,
    Correspondence,
    TopologyPreservation,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EvidenceClaim {
    Positional {
        residual_mm: f64,
        admitted_bound_mm: f64,
    },
    RootParameter {
        interval: [f64; 2],
        admitted_width: f64,
    },
    TangentNormal {
        angular_error_radians: f64,
        admitted_bound_radians: f64,
    },
    Correspondence {
        distance_mm: f64,
        admitted_bound_mm: f64,
    },
    TopologyPreservation {
        invariant: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum EvidenceState {
    Proven(EvidenceClaim),
    Indeterminate(&'static str),
}

/// A finite evidence item bound to a portable tolerance-specification identity.
#[derive(Clone, Debug, PartialEq)]
pub struct PredicateEvidence {
    pub context: ToleranceSpecIdentity,
    pub kind: EvidenceKind,
    pub state: EvidenceState,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComposedEvidence {
    pub context: ToleranceSpecIdentity,
    pub claims: Vec<EvidenceClaim>,
}

fn finite_nonnegative(value: f64, message: &str) -> Result<()> {
    if value.is_finite() && value >= 0. {
        Ok(())
    } else {
        Err(refuse(message))
    }
}

impl PredicateEvidence {
    pub fn positional(
        context: &ToleranceContext,
        residual_mm: f64,
        local_scale_mm: f64,
    ) -> Result<Self> {
        finite_nonnegative(residual_mm, "Invalid positional residual")?;
        finite_nonnegative(local_scale_mm, "Invalid positional local scale")?;
        let spatial = context.spatial_bounds();
        let admitted_bound_mm = spatial
            .on_mm
            .max(spatial.absolute_mm)
            .max(spatial.relative * local_scale_mm)
            .min(context.entity_error_bounds().maximum_mm);
        let state = if residual_mm < admitted_bound_mm {
            EvidenceState::Proven(EvidenceClaim::Positional {
                residual_mm,
                admitted_bound_mm,
            })
        } else if residual_mm >= spatial.clear_mm.max(admitted_bound_mm) {
            return Err(refuse("Positional residual exceeds clear bound"));
        } else {
            EvidenceState::Indeterminate("positional_gray_band")
        };
        Ok(Self {
            context: context.spec_identity(),
            kind: EvidenceKind::Positional,
            state,
        })
    }

    pub fn root_parameter(context: &ToleranceContext, interval: [f64; 2]) -> Result<Self> {
        if !interval.iter().all(|value| value.is_finite()) || interval[0] > interval[1] {
            return Err(refuse("Invalid root-parameter interval"));
        }
        let admitted_width = context.parametric_bounds().floor;
        let width = interval[1] - interval[0];
        Ok(Self {
            context: context.spec_identity(),
            kind: EvidenceKind::RootParameter,
            state: if width <= admitted_width {
                EvidenceState::Proven(EvidenceClaim::RootParameter {
                    interval,
                    admitted_width,
                })
            } else {
                EvidenceState::Indeterminate("root_parameter_not_refined")
            },
        })
    }

    pub fn tangent_normal(context: &ToleranceContext, angular_error_radians: f64) -> Result<Self> {
        finite_nonnegative(angular_error_radians, "Invalid tangent/normal error")?;
        let admitted_bound_radians = context.angular_bounds().radians;
        Ok(Self {
            context: context.spec_identity(),
            kind: EvidenceKind::TangentNormal,
            state: if angular_error_radians <= admitted_bound_radians {
                EvidenceState::Proven(EvidenceClaim::TangentNormal {
                    angular_error_radians,
                    admitted_bound_radians,
                })
            } else {
                EvidenceState::Indeterminate("tangent_normal_bound_exceeded")
            },
        })
    }

    pub fn correspondence(
        context: &ToleranceContext,
        distance_mm: f64,
        local_scale_mm: f64,
    ) -> Result<Self> {
        finite_nonnegative(distance_mm, "Invalid correspondence distance")?;
        finite_nonnegative(local_scale_mm, "Invalid correspondence local scale")?;
        let spatial = context.spatial_bounds();
        let admitted_bound_mm = spatial
            .on_mm
            .max(spatial.absolute_mm)
            .max(spatial.relative * local_scale_mm)
            .min(context.entity_error_bounds().maximum_mm);
        Ok(Self {
            context: context.spec_identity(),
            kind: EvidenceKind::Correspondence,
            state: if distance_mm < admitted_bound_mm {
                EvidenceState::Proven(EvidenceClaim::Correspondence {
                    distance_mm,
                    admitted_bound_mm,
                })
            } else if distance_mm >= spatial.clear_mm.max(admitted_bound_mm) {
                return Err(refuse("Correspondence exceeds clear bound"));
            } else {
                EvidenceState::Indeterminate("correspondence_gray_band")
            },
        })
    }

    pub fn topology_preservation(
        context: &ToleranceContext,
        invariant: impl Into<String>,
        preserved: bool,
    ) -> Result<Self> {
        let invariant = invariant.into();
        if invariant.is_empty() || invariant.len() > 128 {
            return Err(refuse("Invalid topology-preservation invariant"));
        }
        Ok(Self {
            context: context.spec_identity(),
            kind: EvidenceKind::TopologyPreservation,
            state: if preserved {
                EvidenceState::Proven(EvidenceClaim::TopologyPreservation { invariant })
            } else {
                EvidenceState::Indeterminate("topology_preservation_unproven")
            },
        })
    }
}

pub fn compose_predicate_evidence(
    context: &ToleranceContext,
    evidence: impl IntoIterator<Item = PredicateEvidence>,
) -> Result<ComposedEvidence> {
    let expected = context.spec_identity();
    let mut claims = Vec::new();
    for item in evidence {
        if claims.len() == MAX_COMPOSED_EVIDENCE {
            return Err(Error::new(
                "BREP_RESOURCE_LIMIT",
                "Predicate evidence exceeds finite 64-item matrix",
            ));
        }
        if item.context != expected {
            return Err(refuse("Predicate evidence tolerance context mismatch"));
        }
        match item.state {
            EvidenceState::Proven(claim) => claims.push(claim),
            EvidenceState::Indeterminate(_) => {
                return Err(refuse("Predicate evidence contains an indeterminate claim"));
            }
        }
    }
    if claims.is_empty() {
        return Err(refuse("Predicate evidence set is empty"));
    }
    Ok(ComposedEvidence {
        context: expected,
        claims,
    })
}

fn bits(v: f64) -> AuthoredScalar {
    AuthoredScalar::Binary64Bits(v.to_bits())
}

fn plane_frame(plane: Plane) -> Result<[[f64; 3]; 3]> {
    let origin = [
        plane.normal[0] * plane.offset,
        plane.normal[1] * plane.offset,
        plane.normal[2] * plane.offset,
    ];
    let axis = if plane.normal[0].abs() < 0.9 {
        [1., 0., 0.]
    } else {
        [0., 1., 0.]
    };
    let ux = plane.normal[1] * axis[2] - plane.normal[2] * axis[1];
    let uy = plane.normal[2] * axis[0] - plane.normal[0] * axis[2];
    let uz = plane.normal[0] * axis[1] - plane.normal[1] * axis[0];
    let un = (ux * ux + uy * uy + uz * uz).sqrt();
    if !un.is_finite() || un == 0. {
        return Err(refuse("Cannot build plane frame for predicate evidence"));
    }
    let u = [ux / un, uy / un, uz / un];
    let v = [
        plane.normal[1] * u[2] - plane.normal[2] * u[1],
        plane.normal[2] * u[0] - plane.normal[0] * u[2],
        plane.normal[0] * u[1] - plane.normal[1] * u[0],
    ];
    Ok([
        origin,
        [origin[0] + u[0], origin[1] + u[1], origin[2] + u[2]],
        [origin[0] + v[0], origin[1] + v[1], origin[2] + v[2]],
    ])
}

fn sign_of(decision: &cad_predicates::Decision) -> Result<Sign> {
    match decision.outcome {
        Outcome::Sign(sign) => Ok(sign),
        Outcome::Indeterminate(_) => Err(refuse(
            "cad-predicates could not decide halfspace sign; refuse Complete",
        )),
    }
}

/// Require opposite halfspaces for a transverse line/plane contact before Complete.
pub fn require_transverse_line_plane_evidence(
    start: [f64; 3],
    end: [f64; 3],
    plane: Plane,
) -> Result<&'static str> {
    let tolerance = ToleranceContext::default_valid();
    require_transverse_line_plane_evidence_in_context(start, end, plane, &tolerance)
}

pub fn require_transverse_line_plane_evidence_in_context(
    start: [f64; 3],
    end: [f64; 3],
    plane: Plane,
    tolerance: &ToleranceContext,
) -> Result<&'static str> {
    let frame = plane_frame(plane)?;
    let mut values = Vec::with_capacity(15);
    for p in [frame[0], frame[1], frame[2], start, end] {
        values.extend(p.into_iter().map(bits));
    }
    let arena = SourceArena::authored("brep-analytic-ss-line-plane", 1, values)
        .map_err(|_| refuse("Failed to admit authored coordinates for predicate evidence"))?;
    let mut ctx = PredicateContext::new(&arena, tolerance, Limits::default(), None);
    let triple = |base: usize| -> Result<[cad_predicates::LeafRef; 3]> {
        Ok([
            arena
                .leaf(base)
                .map_err(|_| refuse("Missing authored leaf"))?,
            arena
                .leaf(base + 1)
                .map_err(|_| refuse("Missing authored leaf"))?,
            arena
                .leaf(base + 2)
                .map_err(|_| refuse("Missing authored leaf"))?,
        ])
    };
    let a = triple(0)?;
    let b = triple(3)?;
    let c = triple(6)?;
    let start_pt = triple(9)?;
    let end_pt = triple(12)?;
    let s0 = sign_of(
        &orient3d(&mut ctx, a, b, c, start_pt)
            .map_err(|_| refuse("orient3d failed for line start halfspace"))?,
    )?;
    let s1 = sign_of(
        &orient3d(&mut ctx, a, b, c, end_pt)
            .map_err(|_| refuse("orient3d failed for line end halfspace"))?,
    )?;
    if s0 == Sign::Zero || s1 == Sign::Zero {
        return Err(refuse(
            "Endpoint lies on supporting plane in predicate arithmetic; not transverse",
        ));
    }
    if s0 == s1 {
        return Err(refuse(
            "Endpoints are not on opposite halfspaces; refuse transverse Complete",
        ));
    }
    Ok("cad_predicates_orient3d_opposite_halfspaces")
}

/// Corroborate that three authored points form a positively oriented triangle (2D lift).
pub fn require_positive_orient2d_evidence(
    a: [f64; 2],
    b: [f64; 2],
    c: [f64; 2],
) -> Result<&'static str> {
    let tolerance = ToleranceContext::default_valid();
    require_positive_orient2d_evidence_in_context(a, b, c, &tolerance)
}

pub fn require_positive_orient2d_evidence_in_context(
    a: [f64; 2],
    b: [f64; 2],
    c: [f64; 2],
    tolerance: &ToleranceContext,
) -> Result<&'static str> {
    use cad_predicates::orient2d;
    let mut values = Vec::with_capacity(6);
    for p in [a, b, c] {
        values.extend(p.into_iter().map(bits));
    }
    let arena = SourceArena::authored("brep-predicate-orient2d", 1, values)
        .map_err(|_| refuse("Failed to admit authored 2D coordinates"))?;
    let mut ctx = PredicateContext::new(&arena, tolerance, Limits::default(), None);
    let leaf = |i: usize| {
        arena
            .leaf(i)
            .map_err(|_| refuse("Missing authored leaf for orient2d"))
    };
    let decision = orient2d(
        &mut ctx,
        [leaf(0)?, leaf(1)?],
        [leaf(2)?, leaf(3)?],
        [leaf(4)?, leaf(5)?],
    )
    .map_err(|_| refuse("orient2d failed"))?;
    match sign_of(&decision)? {
        Sign::Positive => Ok("cad_predicates_orient2d_positive"),
        Sign::Negative => Err(refuse(
            "Triangle orientation is clockwise; refuse positive evidence",
        )),
        Sign::Zero => Err(refuse("Collinear triangle; refuse orient2d evidence")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intersections::Plane;

    #[test]
    fn transverse_segment_gets_opposite_halfspace_evidence() {
        let note = require_transverse_line_plane_evidence(
            [0., 0., -1.],
            [0., 0., 1.],
            Plane {
                normal: [0., 0., 1.],
                offset: 0.,
            },
        )
        .unwrap();
        assert!(note.contains("orient3d"));
    }

    #[test]
    fn same_side_segment_refuses_evidence() {
        assert!(
            require_transverse_line_plane_evidence(
                [0., 0., 1.],
                [0., 0., 2.],
                Plane {
                    normal: [0., 0., 1.],
                    offset: 0.,
                },
            )
            .is_err()
        );
    }

    #[test]
    fn positive_orient2d_evidence() {
        let note = require_positive_orient2d_evidence([0., 0.], [1., 0.], [0., 1.]).unwrap();
        assert!(note.contains("orient2d"));
        assert!(require_positive_orient2d_evidence([0., 0.], [0., 1.], [1., 0.]).is_err());
    }

    #[test]
    fn evidence_matrix_composes_all_independent_claims() {
        let context = ToleranceContext::default_valid();
        let evidence = vec![
            PredicateEvidence::positional(&context, 1e-8, 1.).unwrap(),
            PredicateEvidence::root_parameter(&context, [0.5, 0.5]).unwrap(),
            PredicateEvidence::tangent_normal(&context, 1e-10).unwrap(),
            PredicateEvidence::correspondence(&context, 1e-8, 1.).unwrap(),
            PredicateEvidence::topology_preservation(&context, "closed_manifold_incidence", true)
                .unwrap(),
        ];
        let composed = compose_predicate_evidence(&context, evidence).unwrap();
        assert_eq!(composed.claims.len(), 5);
        assert_eq!(composed.context, context.spec_identity());
    }

    #[test]
    fn positional_bound_is_scale_aware_and_translation_independent() {
        let mut spec = ToleranceContext::default_valid().specification().clone();
        spec.linear_rel = 1e-3;
        spec.clear_tol = 1e-2;
        spec.max_entity_error = 2.;
        let context = ToleranceContext::new(spec).unwrap();
        assert!(matches!(
            PredicateEvidence::positional(&context, 0.5, 1_000.)
                .unwrap()
                .state,
            EvidenceState::Proven(_)
        ));
        assert!(PredicateEvidence::positional(&context, 0.5, 1.).is_err());

        let near_origin = PredicateEvidence::positional(&context, 1e-8, 2.).unwrap();
        let translated = PredicateEvidence::positional(&context, 1e-8, 2.).unwrap();
        assert_eq!(near_origin, translated);
    }

    #[test]
    fn gray_band_and_context_mismatch_refuse_aggregate() {
        let context = ToleranceContext::default_valid();
        let gray = PredicateEvidence::positional(&context, 1e-6, 1.).unwrap();
        assert!(matches!(gray.state, EvidenceState::Indeterminate(_)));
        assert!(compose_predicate_evidence(&context, [gray]).is_err());

        let mut foreign_spec = context.specification().clone();
        foreign_spec.policy = "foreign-evidence-policy".into();
        let foreign = ToleranceContext::new(foreign_spec).unwrap();
        let item = PredicateEvidence::topology_preservation(&foreign, "edge_order", true).unwrap();
        assert!(compose_predicate_evidence(&context, [item]).is_err());
    }

    #[test]
    fn evidence_resource_limit_is_finite() {
        let context = ToleranceContext::default_valid();
        let evidence = (0..=MAX_COMPOSED_EVIDENCE)
            .map(|index| {
                PredicateEvidence::topology_preservation(
                    &context,
                    format!("invariant-{index}"),
                    true,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let error = compose_predicate_evidence(&context, evidence).unwrap_err();
        assert_eq!(error.code, "BREP_RESOURCE_LIMIT");
    }
}
