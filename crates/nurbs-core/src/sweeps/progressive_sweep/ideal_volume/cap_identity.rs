//! Identity of ORIGINAL tensor carriers in normalized UV coordinates.
//!
//! A positive affine change of each complete knot vector preserves its basis
//! in normalized coordinates. One common positive homogeneous weight factor
//! cancels from the rational quotient. With the same Cartesian controls this
//! proves equality of the entire maps and every existing normalized jet,
//! including one-sided jets at knots. A second exact owner also compares
//! different degrees/nets on the complete common original knot partition.
//! No rounded normalized knots, quotients,
//! inserted controls, sampled values or supplied equality flags enter this proof.
use super::*;
use cad_predicates::{
    AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
};

pub(super) struct Identity<'a> {
    caps: [&'a Surface; 2],
}
impl Identity<'_> {
    pub(super) fn matches(&self, caps: [&Surface; 2]) -> bool {
        std::ptr::eq(self.caps[0], caps[0]) && std::ptr::eq(self.caps[1], caps[1])
    }
}

mod tensor;
pub(super) fn certify<'a>(
    caps: [&'a Surface; 2],
    out: &mut IdealVolumeReport,
    max_cells: usize,
    max_work: u64,
) -> Option<Identity<'a>> {
    if let Some(proof) = affine(caps, out, max_cells, max_work) {
        return Some(proof);
    }
    tensor::certify(caps, out, max_cells, max_work).then_some(Identity { caps })
}

/// Inputs are the already validated immutable original carriers. A failed
/// trial retains all spent work in the owning volume ledger; open moving-cap
/// proofs may still use the remaining budget without assuming any identity.
fn affine<'a>(
    caps: [&'a Surface; 2],
    out: &mut IdealVolumeReport,
    max_cells: usize,
    max_work: u64,
) -> Option<Identity<'a>> {
    let [a, b] = caps;
    // Retain the pre-existing literal-source route, including its zero work.
    if a == b {
        return Some(Identity { caps });
    }
    if a.periodic_u
        || a.periodic_v
        || b.periodic_u
        || b.periodic_v
        || a.degree_u != b.degree_u
        || a.degree_v != b.degree_v
        || a.control_points.len() != b.control_points.len()
        || a.control_points[0].len() != b.control_points[0].len()
        || a.knots_u.len() != b.knots_u.len()
        || a.knots_v.len() != b.knots_v.len()
    {
        return None;
    }
    // Pay for the full original net/knot inspection before constructing any
    // predicate arena. One source stage never acquires an implicit larger cap.
    let charge = a
        .control_points
        .len()
        .checked_mul(a.control_points[0].len())?
        .checked_add(a.knots_u.len())?
        .checked_add(a.knots_v.len())?;
    if charge > 100000 || charge > max_cells.checked_sub(out.cells)? {
        return None;
    }
    out.cells += charge;
    if a.control_points != b.control_points {
        return None;
    }
    let mut points = Vec::new();
    let mut checks = Vec::new();
    if a.weights != b.weights {
        points.extend([[0., 0.], [a.weights[0][0], b.weights[0][0]]]);
        for (&x, &y) in a.weights.iter().flatten().zip(b.weights.iter().flatten()) {
            let i = points.len();
            points.push([x, y]);
            checks.push([0, 1, i]);
        }
    }
    for (ak, bk, degree, end) in [
        (&a.knots_u, &b.knots_u, a.degree_u, a.control_points.len()),
        (
            &a.knots_v,
            &b.knots_v,
            a.degree_v,
            a.control_points[0].len(),
        ),
    ] {
        if ak == bk {
            continue;
        }
        let lo = points.len();
        points.extend([[ak[degree], bk[degree]], [ak[end], bk[end]]]);
        for (&x, &y) in ak.iter().zip(bk) {
            let i = points.len();
            points.push([x, y]);
            checks.push([lo, lo + 1, i]);
        }
    }
    if checks.is_empty() {
        return Some(Identity { caps });
    }
    if points.len() > 16384 || out.exact_work >= max_work {
        return None;
    }
    let arena = SourceArena::authored(
        "original-normalized-filled-cap-identity",
        1,
        points
            .iter()
            .flatten()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .ok()?;
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: max_work - out.exact_work,
            ..Limits::default()
        },
        None,
    );
    let refs = |i| [arena.leaf(2 * i).unwrap(), arena.leaf(2 * i + 1).unwrap()];
    let proven = checks.into_iter().all(|[a, b, c]| {
        matches!(
            cad_predicates::orient2d(&mut ctx, refs(a), refs(b), refs(c)).map(|r| r.outcome),
            Ok(Outcome::Sign(Sign::Zero))
        )
    });
    out.exact_work += ctx.work_used();
    proven.then_some(Identity { caps })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod closed_tests;

#[cfg(test)]
mod tensor_tests;
