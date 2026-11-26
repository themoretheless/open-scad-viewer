//! Exact threshold roots on a one-sided UV edge of an original knot cell.
use super::{curvature_gap_threshold::threshold_residual, umbilic_subdivision::split, *};
use num_traits::Signed;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurvatureGapEdge {
    UMin,
    UMax,
    VMin,
    VMax,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeThresholdBracket {
    /// Open interval in the unit parameter of the varying axis.
    pub unit_interval: [f64; 2],
    /// Bernstein sign variation bounds the number of roots with multiplicity.
    /// A value of one certifies exactly one simple root. Zero is omitted.
    pub variation: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EdgeThresholdRoots {
    Coincident,
    /// All roots lie at an exact dyadic point or inside an open bracket.
    Isolated {
        exact_points: Vec<f64>,
        brackets: Vec<EdgeThresholdBracket>,
    },
    UnresolvedDegree,
}

#[derive(Clone, Debug)]
pub struct EdgeThresholdAudit {
    /// Absolute parameter endpoints along the varying axis.
    pub varying_domain: [f64; 2],
    pub roots: Option<EdgeThresholdRoots>,
    pub refusal: Option<Refusal>,
    pub work_used: u64,
}

/// Enclose all points where the squared principal curvature gap equals a
/// positive level on the specified one-sided knot-cell edge. `span` uses
/// original expanded knot indices. Each depth-capped bracket covers all
/// remaining roots in its interior, with an exact variation upper bound.
/// This operation certifies roots on the edge, without tracing UV curves.
pub fn curvature_gap_edge_roots(
    source: &Surface,
    span: [usize; 2],
    edge: CurvatureGapEdge,
    level: f64,
    max_depth: u8,
    max_work: u64,
) -> Result<EdgeThresholdAudit> {
    source.validate()?;
    check(
        level.is_finite() && level > 0.,
        "Curvature gap level must be finite and positive",
    )?;
    check(
        max_depth <= 16,
        "Curvature gap edge depth must be in [0,16]",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Curvature gap edge work must be in [1,1000000]",
    )?;
    let [ku, kv] = span;
    check(
        (source.degree_u..source.control_points.len()).contains(&ku)
            && (source.degree_v..source.control_points[0].len()).contains(&kv),
        "Curvature gap edge requires active knot indices",
    )?;
    let u = [source.knots_u[ku], source.knots_u[ku + 1]];
    let v = [source.knots_v[kv], source.knots_v[kv + 1]];
    check(
        u[0] < u[1] && v[0] < v[1],
        "Curvature gap edge requires a nonempty original cell",
    )?;
    let (fixed, upper) = match edge {
        CurvatureGapEdge::UMin => (0, false),
        CurvatureGapEdge::UMax => (0, true),
        CurvatureGapEdge::VMin => (1, false),
        CurvatureGapEdge::VMax => (1, true),
    };
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| {
        let level = work.lift(level)?;
        let residual = threshold_residual(source, ku, kv, &level, &mut work)?;
        let mut restriction = Poly::new();
        for ((i, j), value) in residual {
            let powers = [i, j];
            if upper || powers[fixed] == 0 {
                polynomial::accumulate(
                    &mut restriction,
                    (powers[1 - fixed], 0),
                    &value,
                    &mut work,
                )?;
            }
        }
        if restriction.is_empty() {
            return Ok(EdgeThresholdRoots::Coincident);
        }
        let degree = storage::degrees(std::slice::from_ref(&restriction)).0;
        if degree > 25 {
            return Ok(EdgeThresholdRoots::UnresolvedDegree);
        }
        let power = (0..=degree)
            .map(|i| restriction.remove(&(i, 0)).unwrap_or_else(Q::zero))
            .collect::<Vec<_>>();
        isolate(
            uni::bernstein(&power, degree, &mut work)?,
            max_depth,
            &mut work,
        )
    })();
    let (roots, refusal) = match result {
        Ok(roots) => (Some(roots), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(EdgeThresholdAudit {
        varying_domain: [u, v][1 - fixed],
        roots,
        refusal,
        work_used: work.used,
    })
}

fn isolate(net: Vec<Q>, max_depth: u8, work: &mut Work) -> PResult<EdgeThresholdRoots> {
    let size = [net.len(), 1];
    let mut pending = vec![([0., 1.], net, 0u8)];
    let mut exact_points = Vec::new();
    let mut brackets = Vec::new();
    while let Some(([lo, hi], net, depth)) = pending.pop() {
        if net[0].is_zero() {
            exact_points.push(lo);
        }
        if net[net.len() - 1].is_zero() {
            exact_points.push(hi);
        }
        let mut previous = None;
        let mut variation = 0;
        for q in net.iter().filter(|q| !q.is_zero()) {
            let positive = q.is_positive();
            variation += usize::from(previous.is_some_and(|old| old != positive));
            previous = Some(positive);
        }
        if variation == 0 {
            continue;
        }
        if depth == max_depth {
            brackets.push(EdgeThresholdBracket {
                unit_interval: [lo, hi],
                variation,
            });
            continue;
        }
        let mid = (lo + hi) * 0.5;
        let [left, right] = split(&net, size, 0, work)?;
        pending.push(([mid, hi], right, depth + 1));
        pending.push(([lo, mid], left, depth + 1));
    }
    exact_points.sort_by(f64::total_cmp);
    exact_points.dedup();
    Ok(EdgeThresholdRoots::Isolated {
        exact_points,
        brackets,
    })
}
