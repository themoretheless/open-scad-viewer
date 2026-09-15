//! Bridge authored cad-predicates decisions into intersection Complete reports.
//!
//! Predicates never authorize topology alone; they only corroborate algebraic
//! Complete strata before coverage is published.

use cad_predicates::{
    AuthoredScalar, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext, Limits,
    orient3d,
};
use nurbs_core::{Error, Result};

use crate::intersections::Plane;

fn refuse(message: &str) -> Error {
    Error::new("BREP_PREDICATE_EVIDENCE_REFUSED", message)
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
    Ok([origin, [origin[0] + u[0], origin[1] + u[1], origin[2] + u[2]], [
        origin[0] + v[0],
        origin[1] + v[1],
        origin[2] + v[2],
    ]])
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
    let frame = plane_frame(plane)?;
    let mut values = Vec::with_capacity(15);
    for p in [frame[0], frame[1], frame[2], start, end] {
        values.extend(p.into_iter().map(bits));
    }
    let arena = SourceArena::authored("brep-analytic-ss-line-plane", 1, values)
        .map_err(|_| refuse("Failed to admit authored coordinates for predicate evidence"))?;
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
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
    let s0 = sign_of(&orient3d(&mut ctx, a, b, c, start_pt).map_err(|_| {
        refuse("orient3d failed for line start halfspace")
    })?)?;
    let s1 = sign_of(&orient3d(&mut ctx, a, b, c, end_pt).map_err(|_| {
        refuse("orient3d failed for line end halfspace")
    })?)?;
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
}
