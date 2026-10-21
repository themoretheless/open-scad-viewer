//! Copy-coherent rational homotopies without pretending coincident loops embed.
mod projective;
use super::{
    AnchorLimits, AnchorTransfer, PeriodicWindingReport, Report, inspect_periodic_winding,
    verify_anchor_winding,
};
use crate::{Result, check, curve::Curve, exact_products::sum_sign};
pub use projective::{
    ProjectiveCurveCorrespondence, ProjectivePartitionHomotopy,
    verify_projective_curve_correspondence, verify_projective_partition_homotopy,
};
use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub struct CoherentAnchorTransfer {
    /// Every original source index occurs exactly once in these groups.
    pub groups: Vec<Vec<usize>>,
    pub representatives: Vec<usize>,
    /// Orientation of each original source relative to its representative.
    pub source_orientations: Vec<i8>,
    /// Embedding/isotopy proof of distinct representative functions only.
    pub quotient: AnchorTransfer,
    /// Actual query on all original sources, with their full multiplicity.
    pub original_source: Option<PeriodicWindingReport>,
    pub winding: Option<i32>,
    pub coherent_source_winding_preserved: bool,
    /// Shared representative and full-source classification cells.
    pub classification_cells: usize,
    pub reason: Option<&'static str>,
}
// Equal normalized coordinates, using exactly eight original products.
fn normalized_equal(x: f64, a: [f64; 2], y: f64, b: [f64; 2], reverse: bool) -> bool {
    let (z, z0) = if reverse { (b[1], y) } else { (y, b[0]) };
    sum_sign(&[
        (x, b[1]),
        (-x, b[0]),
        (-a[0], b[1]),
        (a[0], b[0]),
        (-z, a[1]),
        (z, a[0]),
        (z0, a[1]),
        (-z0, a[0]),
    ]) == Ordering::Equal
}
/// Exact normalized parameter identity, optionally with reversed orientation.
/// No affine slope, offset or reflected knot is materialized in binary64.
pub(super) fn normalized_function(a: &Curve, b: &Curve, reverse: bool) -> bool {
    if a.degree != b.degree
        || a.periodic != b.periodic
        || a.knots.len() != b.knots.len()
        || a.weights.len() != b.weights.len()
        || a.weights.is_empty()
        || a.weights.len() != a.control_points.len()
        || b.weights.len() != b.control_points.len()
    {
        return false;
    }
    let n = a.weights.len();
    if a.control_points.len() != b.control_points.len() {
        return false;
    }
    if !(0..a.control_points.len())
        .all(|i| a.control_points[i] == b.control_points[if reverse { n - 1 - i } else { i }])
    {
        return false;
    }
    let wb = b.weights[if reverse { n - 1 } else { 0 }];
    if !(0..n).all(|i| {
        sum_sign(&[
            (a.weights[i], wb),
            (
                -b.weights[if reverse { n - 1 - i } else { i }],
                a.weights[0],
            ),
        ]) == Ordering::Equal
    }) {
        return false;
    }
    a.knots.iter().enumerate().all(|(i, &x)| {
        normalized_equal(
            x,
            a.domain(),
            b.knots[if reverse { b.knots.len() - 1 - i } else { i }],
            b.domain(),
            reverse,
        )
    })
}
fn same_homotopy(a: &Report, b: &Report, ad: [f64; 2], bd: [f64; 2], reverse: bool) -> bool {
    a.pieces.len() == b.pieces.len()
        && a.pieces.iter().enumerate().all(|(i, x)| {
            let y = &b.pieces[if reverse { b.pieces.len() - 1 - i } else { i }];
            normalized_equal(
                x.source_domain[0],
                ad,
                y.source_domain[if reverse { 1 } else { 0 }],
                bd,
                reverse,
            ) && normalized_equal(
                x.source_domain[1],
                ad,
                y.source_domain[if reverse { 0 } else { 1 }],
                bd,
                reverse,
            ) && x.translation == y.translation
                && x.tile == y.tile
                && normalized_function(&x.curve, &y.curve, reverse)
        })
}
/// Prove coincident copies move identically, then verify the distinct-function
/// quotient isotopy and the actual original winding at the anchor. Function
/// identity requires exactly matching normalized (optionally reversed) basis,
/// control values and proportional positive weights, for sources and candidates.
/// Exact affine source-domain correspondence is checked for every paired piece. Independent source/candidate global weight scales are allowed.
/// Preview hull flags are not authority. Nearly matching normalized bases or
/// unsynchronized candidate domains/functions are never merged.
/// This is a weighted-source topology proof, not an embedding of all copies
/// or a periodic chart/owned-face admission. Classifier cells are shared across
/// the quotient and full-source anchor queries; other anchor limits are retained.
fn verify_coherent_anchor_winding_impl(
    sources: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: AnchorLimits,
    projective: &[ProjectivePartitionHomotopy],
) -> Result<CoherentAnchorTransfer> {
    check(
        !sources.is_empty() && sources.len() <= 64 && sources.len() == partitions.len(),
        "Invalid coherent source count",
    )?;
    let mut pieces = 0usize;
    for p in partitions {
        pieces = pieces
            .checked_add(p.pieces.len())
            .ok_or_else(|| crate::input("Coherent piece count overflow"))?;
    }
    check(pieces <= 256, "Coherent source exceeds 256 pieces")?;
    for c in sources {
        c.validate()?;
    }
    for (source, partition) in sources.iter().zip(partitions) {
        let d = source.domain();
        for p in &partition.pieces {
            p.curve.validate()?;
            check(
                p.source_domain
                    .iter()
                    .chain(&p.translation)
                    .all(|x| x.is_finite())
                    && d[0] <= p.source_domain[0]
                    && p.source_domain[0] < p.source_domain[1]
                    && p.source_domain[1] <= d[1],
                "Invalid coherent piece domain or translation",
            )?;
        }
    }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut source_orientations = vec![1_i8; sources.len()];
    for i in 0..sources.len() {
        let mut found = None;
        for (g, group) in groups.iter().enumerate() {
            let r = group[0];
            for reverse in [false, true] {
                if (normalized_function(&sources[r], &sources[i], reverse)
                    && same_homotopy(
                        &partitions[r],
                        &partitions[i],
                        sources[r].domain(),
                        sources[i].domain(),
                        reverse,
                    ))
                    || projective.iter().any(|p| {
                        p.matches(
                            &sources[r],
                            &sources[i],
                            &partitions[r],
                            &partitions[i],
                            reverse,
                        )
                    })
                {
                    found = Some((g, reverse));
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        if let Some((g, reverse)) = found {
            groups[g].push(i);
            source_orientations[i] = if reverse { -1 } else { 1 };
        } else {
            groups.push(vec![i]);
        }
    }
    let representatives: Vec<_> = groups.iter().map(|g| g[0]).collect();
    let unique_sources: Vec<_> = representatives
        .iter()
        .map(|&i| sources[i].clone())
        .collect();
    let unique_partitions: Vec<_> = representatives
        .iter()
        .map(|&i| partitions[i].clone())
        .collect();
    let quotient =
        verify_anchor_winding(&unique_sources, &unique_partitions, chart, anchor, limits)?;
    let spent = quotient.source.as_ref().map_or(0, |q| q.cells);
    let mut result = CoherentAnchorTransfer {
        groups,
        representatives,
        source_orientations,
        quotient,
        original_source: None,
        winding: None,
        coherent_source_winding_preserved: false,
        classification_cells: spent,
        reason: None,
    };
    if result.quotient.winding.is_none()
        || !result.quotient.isotopy.lifted_isotopy_preserved
        || !result.quotient.isotopy.periodic_isotopy_preserved
    {
        result.reason = Some("coherent-quotient-topology-unproven");
        return Ok(result);
    }
    let original = if result.groups.len() == sources.len() {
        // Same original ordering and no duplicate groups: reuse the actual
        // original query rather than charge or fabricate a second receipt.
        let Some(query) = result.quotient.source.as_ref() else {
            result.reason = Some("coherent-original-anchor-unproven");
            return Ok(result);
        };
        query.clone()
    } else {
        let Some(remaining) = limits.max_cells.checked_sub(spent).filter(|n| *n > 0) else {
            result.reason = Some("coherent-anchor-classification-budget");
            return Ok(result);
        };
        let loops: Vec<_> = sources.iter().map(|c| vec![c.clone()]).collect();
        let query = inspect_periodic_winding(
            &loops,
            chart,
            anchor,
            limits.tolerance_uv,
            limits.max_images,
            remaining,
        )?;
        result.classification_cells = spent + query.cells;
        query
    };
    if result.classification_cells > limits.max_cells {
        result.reason = Some("coherent-anchor-classification-budget");
        return Ok(result);
    }
    result.winding = original.winding;
    result.coherent_source_winding_preserved = result.winding.is_some();
    if result.winding.is_none() {
        result.reason = Some("coherent-original-anchor-unproven");
    }
    result.original_source = Some(original);
    Ok(result)
}

/// Original exact normalized-function coherent transfer.
pub fn verify_coherent_anchor_winding(
    sources: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: AnchorLimits,
) -> Result<CoherentAnchorTransfer> {
    verify_coherent_anchor_winding_impl(sources, partitions, chart, anchor, limits, &[])
}
#[derive(Clone, Debug)]
pub struct ProjectiveCoherentAnchorTransfer {
    pub transfer: CoherentAnchorTransfer,
    correspondences: Vec<ProjectivePartitionHomotopy>,
    correspondence_work: u64,
}
impl ProjectiveCoherentAnchorTransfer {
    pub fn correspondences(&self) -> &[ProjectivePartitionHomotopy] {
        &self.correspondences
    }
    pub fn correspondence_work(&self) -> u64 {
        self.correspondence_work
    }
}
/// Extend coherent source multiplicity to separately qualified nonlinear
/// source/candidate homotopies. Every supplied receipt is freshly replayed under
/// one shared exact-predicate work budget before quotient topology is checked.
/// The old normalized-function identity is unchanged.
pub fn verify_projective_coherent_anchor_winding(
    sources: &[Curve],
    partitions: &[Report],
    correspondences: &[ProjectivePartitionHomotopy],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: AnchorLimits,
    max_correspondence_work: u64,
) -> Result<ProjectiveCoherentAnchorTransfer> {
    check(
        correspondences.len() <= 64 && max_correspondence_work <= cad_predicates::MAX_WORK,
        "Projective coherent transfer exceeds bounded correspondence allowance",
    )?;
    let mut work = 0;
    for p in correspondences {
        p.reverify(max_correspondence_work - work)?;
        work = work
            .checked_add(p.work_used())
            .ok_or_else(|| crate::input("Correspondence work overflow"))?;
        check(
            work <= max_correspondence_work,
            "Projective correspondence shared work exhausted",
        )?;
    }
    let transfer = verify_coherent_anchor_winding_impl(
        sources,
        partitions,
        chart,
        anchor,
        limits,
        correspondences,
    )?;
    Ok(ProjectiveCoherentAnchorTransfer {
        transfer,
        correspondences: correspondences.to_vec(),
        correspondence_work: work,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalized_basis_identity_does_not_underflow_or_merge_a_one_ulp_change() {
        let mut a = Curve::from_polyline(vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![0., 0.],
        ])
        .unwrap();
        for k in &mut a.knots {
            *k *= 1e-200;
        }
        let mut b = a.clone();
        for k in &mut b.knots {
            *k *= 2.;
        }
        assert_eq!(a.domain()[1] * b.domain()[1], 0.);
        assert!(normalized_function(&a, &b, false));
        let mut reverse = b.clone();
        reverse.control_points.reverse();
        reverse.weights.reverse();
        reverse.knots = reverse.knots.into_iter().rev().map(|k| -k).collect();
        assert!(normalized_function(&a, &reverse, true));
        for u in [0.5e-200, 1.5e-200, 2.5e-200, 3.5e-200] {
            let x = a.evaluate_de_boor(u).unwrap();
            let y = reverse.evaluate_de_boor(-2. * u).unwrap();
            assert!(x.iter().zip(y).all(|(x, y)| (x - y).abs() < 1e-12));
        }
        b.knots[2] = b.knots[2].next_up();
        assert!(!normalized_function(&a, &b, false));
    }
}
