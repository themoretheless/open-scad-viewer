//! Exact original-clock restriction across every common rational knot cell.
use crate::{Result, check, curve::Curve};
use cad_predicates::{
    AuthoredScalar, BezierIdentityDecision, Limits, PredicateContext, SourceArena, ToleranceContext,
};

/// Compare a stored child with its original continuous rational B-spline on the
/// child's entire natural interval. Both keep ORIGINAL parameter coordinates;
/// no knot insertion, rounded reclocking or sampling provides proof authority.
/// Different degrees/knots/positive global weight gauges are supported by the
/// predicate. None is unsupported; non-equality and limits are explicit in the
/// returned decision. This does not grant shared incidence or solid admission.
pub fn verify_exact_bspline_restriction(
    original: &Curve,
    child: &Curve,
    max_work: u64,
) -> Result<Option<BezierIdentityDecision>> {
    original.validate()?;
    child.validate()?;
    check(
        max_work > 0 && max_work <= cad_predicates::MAX_WORK,
        "Invalid original B-spline proof work allowance",
    )?;
    let dim = original.control_points[0].len();
    if original.periodic
        || child.periodic
        || !(2..=3).contains(&dim)
        || original
            .control_points
            .iter()
            .chain(&child.control_points)
            .any(|p| p.len() != dim)
        || [original, child]
            .iter()
            .any(|c| !(1..=32).contains(&c.degree) || c.control_points.len() > 4096)
    {
        return Ok(None);
    }
    let mut values = Vec::new();
    for c in [original, child] {
        for (p, w) in c.control_points.iter().zip(&c.weights) {
            values.extend(p.iter().copied());
            if dim == 2 {
                values.push(0.);
            }
            values.push(*w);
        }
    }
    let offset = values.len();
    values.extend(&original.knots);
    values.extend(&child.knots);
    values.extend([0.; 3]);
    let arena = SourceArena::authored(
        "original-world-bspline-restriction",
        1,
        values
            .iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| crate::input("Invalid original B-spline proof source"))?;
    let rows = |start: usize, n: usize| {
        (start..start + n)
            .map(|i| std::array::from_fn(|k| arena.leaf(4 * i + k).unwrap()))
            .collect::<Vec<_>>()
    };
    let refs = |start: usize, n: usize| {
        (start..start + n)
            .map(|i| arena.leaf(i).unwrap())
            .collect::<Vec<_>>()
    };
    let shift = offset + original.knots.len() + child.knots.len();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work,
            ..Limits::default()
        },
        None,
    );
    cad_predicates::rational_bspline_restriction_identity(
        &mut ctx,
        original.degree,
        &rows(0, original.control_points.len()),
        &refs(offset, original.knots.len()),
        child.degree,
        &rows(original.control_points.len(), child.control_points.len()),
        &refs(offset + original.knots.len(), child.knots.len()),
        std::array::from_fn(|k| arena.leaf(shift + k).unwrap()),
    )
    .map(Some)
    .map_err(|_| crate::input("Invalid original B-spline restriction request"))
}

/// Owned exact functional partition on one original root interval. This binds
/// actual definitions, not topology IDs or a claim supplied by the caller.
pub struct ExactBsplineRootPartition {
    source: Curve,
    pieces: Vec<Curve>,
    range: [f64; 2],
    work: u64,
}
impl ExactBsplineRootPartition {
    pub fn source(&self) -> &Curve {
        &self.source
    }
    pub fn pieces(&self) -> &[Curve] {
        &self.pieces
    }
    pub fn range(&self) -> [f64; 2] {
        self.range
    }
    pub fn work_used(&self) -> u64 {
        self.work
    }
    pub fn reverify(&self, max_work: u64) -> Result<()> {
        let fresh =
            verify_exact_bspline_root_partition(&self.source, &self.pieces, self.range, max_work)?;
        check(
            fresh.is_some_and(|v| v.work == self.work),
            "Exact original root partition remains unproved",
        )
    }
}
/// Every child must keep its original clock and cover the requested interval
/// exactly once in increasing order. Original continuity plus whole-cell
/// identity proves the adjacent root endpoints; no spatial welding is used.
/// None is unproved (including budget limits), never partial success.
pub fn verify_exact_bspline_root_partition(
    source: &Curve,
    pieces: &[Curve],
    range: [f64; 2],
    max_work: u64,
) -> Result<Option<ExactBsplineRootPartition>> {
    source.validate()?;
    check(
        (1..=256).contains(&pieces.len())
            && max_work > 0
            && max_work <= cad_predicates::MAX_WORK
            && range.iter().all(|v| v.is_finite())
            && range[0] < range[1]
            && source.domain()[0] <= range[0]
            && range[1] <= source.domain()[1],
        "Invalid original root partition scope",
    )?;
    let mut cursor = range[0];
    for p in pieces {
        p.validate()?;
        let d = p.domain();
        check(
            d[0] == cursor && d[0] < d[1],
            "Original root partition has a gap, overlap or reversed interval",
        )?;
        cursor = d[1];
    }
    check(
        cursor == range[1],
        "Original root partition omits or exceeds requested end",
    )?;
    let mut work = 0;
    for p in pieces {
        if work >= max_work {
            return Ok(None);
        }
        let Some(r) = verify_exact_bspline_restriction(source, p, max_work - work)? else {
            return Ok(None);
        };
        work += r.work_used;
        if r.outcome != cad_predicates::BezierIdentity::Equal {
            return Ok(None);
        }
    }
    Ok(Some(ExactBsplineRootPartition {
        source: source.clone(),
        pieces: pieces.to_vec(),
        range,
        work,
    }))
}
