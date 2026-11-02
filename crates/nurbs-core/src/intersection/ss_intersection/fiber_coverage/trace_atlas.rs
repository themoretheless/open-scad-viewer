//! All original parameter roots covered by source-bound world trace pieces.
//! This is a geometric coverage receipt, not a trim arrangement, a physical
//! branch bijection, or a retained-material classification certificate.
use super::*;

#[derive(Clone, Debug)]
pub struct QualifiedFiberTraceAtlas {
    owner: QualifiedFiberComponents,
    pieces: Vec<Vec<QualifiedFiberWorldTrace>>,
    tolerance_world: f64,
    maximum_error: f64,
}
impl QualifiedFiberTraceAtlas {
    pub fn owner(&self) -> &QualifiedFiberComponents {
        &self.owner
    }
    /// One complete ordered partition per original coverage chart.
    pub fn pieces(&self) -> &[Vec<QualifiedFiberWorldTrace>] {
        &self.pieces
    }
    pub fn maximum_world_error(&self) -> f64 {
        self.maximum_error
    }
    pub fn reverify(&self) -> Result<()> {
        let error = verify(&self.owner, &self.pieces, self.tolerance_world)?;
        check(
            error == self.maximum_error,
            "World trace atlas error receipt changed",
        )
    }
}

fn verify(
    owner: &QualifiedFiberComponents,
    pieces: &[Vec<QualifiedFiberWorldTrace>],
    tolerance: f64,
) -> Result<f64> {
    check(
        (1e-12..=1e-2).contains(&tolerance),
        "Invalid complete trace atlas tolerance",
    )?;
    check(
        owner.coverage().covers_entire_surfaces(),
        "Trace atlas needs complete original surface coverage",
    )?;
    let charts = owner.coverage().fibers();
    check(
        pieces.len() == charts.len() && pieces.len() <= 4096,
        "Every original coverage chart needs its own trace partition",
    )?;
    let count = pieces
        .iter()
        .try_fold(0usize, |n, p| n.checked_add(p.len()));
    check(
        count.is_some_and(|n| n <= 4096),
        "Complete trace atlas exceeds piece allowance",
    )?;
    owner.reverify()?;
    let mut error = 0_f64;
    for (chart, partition) in charts.iter().zip(pieces) {
        check(
            !partition.is_empty(),
            "Missing original root trace partition",
        )?;
        let free = chart.free_axis().index();
        let domain = chart.domain();
        let mut endpoint = domain[free][0];
        for piece in partition {
            let local = piece.source();
            let d = local.domain();
            check(
                local.free_axis() == chart.free_axis()
                    && local.source().first() == chart.source().first()
                    && local.source().second() == chart.source().second(),
                "Trace piece belongs to another original source or root clock",
            )?;
            // Keeping the same unknown box is essential: local uniqueness in
            // an unrelated smaller box does not cover the parent's root.
            check(
                (0..4).all(|k| k == free || d[k] == domain[k])
                    && d[free][0] == endpoint
                    && d[free][0] < d[free][1]
                    && d[free][1] <= domain[free][1],
                "Trace partition has a gap, overlap, foreign root box or reversed interval",
            )?;
            piece.reverify()?;
            let bound = piece.world_error_upper();
            check(
                bound.is_finite() && bound >= 0. && bound <= tolerance,
                "Original root trace exceeds atlas world tolerance",
            )?;
            error = error.max(bound);
            endpoint = d[free][1];
        }
        check(
            endpoint == domain[free][1],
            "Trace partition omits original roots at the chart end",
        )?;
    }
    Ok(error)
}

/// Replays complete original-product exclusion, component ownership, and every
/// world trace. Pieces must partition every chart's full free interval exactly.
/// Overlapping charts are retained; their traces need not coincide exactly.
pub fn qualify_fiber_trace_atlas(
    owner: &QualifiedFiberComponents,
    pieces: &[Vec<QualifiedFiberWorldTrace>],
    tolerance_world: f64,
) -> Result<QualifiedFiberTraceAtlas> {
    let maximum_error = verify(owner, pieces, tolerance_world)?;
    Ok(QualifiedFiberTraceAtlas {
        owner: owner.clone(),
        pieces: pieces.to_vec(),
        tolerance_world,
        maximum_error,
    })
}
