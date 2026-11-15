//! Certify a whole-cell curvature threshold contour as one coordinate graph.
use super::{curvature_gap_threshold::threshold_residual, *};
use num_traits::Signed;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurvatureGapGraph {
    /// Exactly one simple crossing for each coordinate on the other axis.
    /// The flag gives the sign of the residual slope in the chosen axis.
    Unique {
        increasing: bool,
    },
    EverywhereAtLevel,
    NoContour,
    Unresolved,
}

#[derive(Clone, Debug)]
pub struct CurvatureGapGraphAudit {
    pub domain: [[f64; 2]; 2],
    pub graph: Option<CurvatureGapGraph>,
    pub refusal: Option<Refusal>,
    pub work_used: u64,
}

/// Certify the squared-gap contour on an original UV cell as a graph of
/// the chosen axis over the other axis. Requires uniformly opposite signs
/// on the two fixed-coordinate faces and a strictly monotone residual in
/// the chosen axis. The certificate is local to this cell and does not
/// join traces across source knots.
pub fn certify_curvature_gap_graph(
    source: &Surface,
    span: [usize; 2],
    axis: Axis,
    level: f64,
    max_work: u64,
) -> Result<CurvatureGapGraphAudit> {
    source.validate()?;
    check(
        level.is_finite() && level > 0.,
        "Curvature gap level must be finite and positive",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Curvature graph work must be in [1,1000000]",
    )?;
    let [ku, kv] = span;
    check(
        (source.degree_u..source.control_points.len()).contains(&ku)
            && (source.degree_v..source.control_points[0].len()).contains(&kv),
        "Curvature graph requires active knot indices",
    )?;
    let domain = [
        [source.knots_u[ku], source.knots_u[ku + 1]],
        [source.knots_v[kv], source.knots_v[kv + 1]],
    ];
    check(
        domain.iter().all(|[a, b]| a < b),
        "Curvature graph requires a nonempty original cell",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| {
        let level = work.lift(level)?;
        let residual = threshold_residual(source, ku, kv, &level, &mut work)?;
        classify(&residual, axis, &mut work)
    })();
    let (graph, refusal) = match result {
        Ok(graph) => (Some(graph), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(CurvatureGapGraphAudit {
        domain,
        graph,
        refusal,
        work_used: work.used,
    })
}

fn classify(residual: &Poly, axis: Axis, work: &mut Work) -> PResult<CurvatureGapGraph> {
    use CurvatureGapGraph::*;
    if residual.is_empty() {
        return Ok(EverywhereAtLevel);
    }
    let (pu, pv) = storage::degrees(std::slice::from_ref(residual));
    if pu > 25 || pv > 25 {
        return Ok(Unresolved);
    }
    let net = storage::bernstein(residual, pu, pv, work)?;
    if net.iter().all(Signed::is_positive) || net.iter().all(Signed::is_negative) {
        return Ok(NoContour);
    }
    let (line_len, lines) = match axis {
        Axis::U => (pu + 1, pv + 1),
        Axis::V => (pv + 1, pu + 1),
    };
    let coefficient = |line, i| match axis {
        Axis::U => &net[i * (pv + 1) + line],
        Axis::V => &net[line * (pv + 1) + i],
    };
    let increasing = coefficient(0, 0).is_negative();
    for line in 0..lines {
        let first = coefficient(line, 0);
        let last = coefficient(line, line_len - 1);
        let opposite = if increasing {
            first.is_negative() && last.is_positive()
        } else {
            first.is_positive() && last.is_negative()
        };
        if !opposite {
            return Ok(Unresolved);
        }
        // Differentiating a Bernstein polynomial multiplies successive
        // coefficient differences by the positive degree.
        work.charge((line_len - 1) as u64)?;
        for i in 0..line_len - 1 {
            let a = coefficient(line, i);
            let b = coefficient(line, i + 1);
            if (increasing && b <= a) || (!increasing && b >= a) {
                return Ok(Unresolved);
            }
        }
    }
    Ok(Unique { increasing })
}
