//! Exact original-function chart transfer; endpoint contacts are separate from
//! a complete interior-contact census and owned nonzero-fill regularization.
mod mapped;
use super::{
    ChartAssemblyLimits, ChartCalibration, ContactOwnershipReport, ExactPartitionIdentity,
    LinkStatus, Report, WeightedChartGraph, double_chart, exact_partition, inspect_connectivity,
    inspect_contact_ownership, inspect_periodic_winding, weighted,
};
use crate::{Result, check, curve::Curve, exact_products::sum_sign};
pub use mapped::{ClockAliasSpec, ClockMappedSourceChart, assemble_clock_mapped_source_chart};
#[derive(Clone, Debug)]
pub struct ExactSourceChart {
    sources: Vec<Curve>,
    partitions: Vec<Report>,
    windings: Vec<[i32; 2]>,
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ChartAssemblyLimits,
    max_work: u64,
    identities: Vec<ExactPartitionIdentity>,
    work: u64,
    graph: Option<WeightedChartGraph>,
    materialized: Option<super::ChartBoundaryGraph>,
    contacts: Option<ContactOwnershipReport>,
    calibration: Option<ChartCalibration>,
    reason: Option<&'static str>,
}
impl ExactSourceChart {
    /// Recompute correspondence using the owned original curves and partitions.
    /// This proves world deviation and requested seams, not surface embedding.
    pub fn verify_surface(
        &self,
        surface: &crate::surface::Surface,
        wrapped: [bool; 2],
        limits: super::SurfaceCorrespondenceLimits,
    ) -> Result<super::SurfaceCorrespondence> {
        self.reverify()?;
        super::verify_surface_correspondence(
            surface,
            &self.sources,
            &self.partitions,
            self.chart,
            wrapped,
            limits,
        )
    }

    pub fn chart(&self) -> [[f64; 2]; 2] {
        self.chart
    }
    pub fn identities(&self) -> &[ExactPartitionIdentity] {
        &self.identities
    }
    /// Actual expanded graph AFTER source-anchor/perimeter winding correction.
    /// calibration.raw_graph is a diagnostic BEFORE that correction.
    pub fn materialized_graph(&self) -> Option<&super::ChartBoundaryGraph> {
        self.materialized.as_ref()
    }
    pub fn graph(&self) -> Option<&WeightedChartGraph> {
        self.graph.as_ref()
    }
    pub fn contact_ownership(&self) -> Option<&ContactOwnershipReport> {
        self.contacts.as_ref()
    }
    pub fn calibration(&self) -> Option<&ChartCalibration> {
        self.calibration.as_ref()
    }
    pub fn reason(&self) -> Option<&'static str> {
        self.reason
    }
    pub fn work_used(&self) -> u64 {
        self.work
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = assemble_exact_source_chart(
            &self.sources,
            &self.partitions,
            &self.windings,
            self.chart,
            self.anchor,
            self.limits,
            self.max_work,
        )?;
        check(
            self.graph.is_some()
                && fresh.graph == self.graph
                && fresh.materialized == self.materialized
                && fresh.contacts == self.contacts
                && fresh.work == self.work
                && fresh.reason == self.reason
                && fresh.identities == self.identities
                && fresh.calibration == self.calibration,
            "Exact source chart graph/contact receipt changed or remains unproven",
        )
    }
}
/// Transfer complete unchanged original functions to a weighted periodic chart.
/// All source identities, actual closure/tiles/hulls and winding calibration
/// are fresh. Contact ownership covers endpoint ports; no interior-contact or
/// regular-face claim is made here. Complete nonzero-fill admission is separate.
pub fn assemble_exact_source_chart(
    sources: &[Curve],
    partitions: &[Report],
    windings: &[[i32; 2]],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ChartAssemblyLimits,
    max_work: u64,
) -> Result<ExactSourceChart> {
    double_chart::validate_limits(chart, anchor, limits)?;
    check(
        (1..=64).contains(&sources.len())
            && sources.len() == partitions.len()
            && sources.len() == windings.len()
            && max_work <= cad_predicates::MAX_WORK
            && (1..=512).contains(&limits.max_segments)
            && (1..=512).contains(&limits.max_links),
        "Invalid exact source chart inputs or shared budgets",
    )?;
    let mut out = ExactSourceChart {
        sources: sources.to_vec(),
        partitions: partitions.to_vec(),
        windings: windings.to_vec(),
        chart,
        anchor,
        limits,
        max_work,
        identities: Vec::new(),
        work: 0,
        graph: None,
        materialized: None,
        contacts: None,
        calibration: None,
        reason: None,
    };
    let count = partitions.iter().map(|p| p.pieces.len()).sum::<usize>();
    if count > limits.max_segments || count > limits.max_edges || count > limits.max_links {
        out.reason = Some("exact-source-chain-budget");
        return Ok(out);
    }
    if (0..2).any(|a| windings.iter().map(|w| i64::from(w[a])).sum::<i64>() != 0) {
        out.reason = Some("nonzero-total-source-homology");
        return Ok(out);
    }
    let mut links = 0;
    for ((source, partition), winding) in sources.iter().zip(partitions).zip(windings) {
        let (proof, used) =
            exact_partition::verify_with_work(source, partition, max_work - out.work)?;
        out.work += used;
        let Some(proof) = proof else {
            out.reason = Some("original-function-identity-unproven");
            return Ok(out);
        };
        out.identities.push(proof);
        for p in &partition.pieces {
            let c = &p.curve;
            let d = c.domain();
            if !c.knots[..=c.degree].iter().all(|&u| u == d[0])
                || !c.knots[c.control_points.len()..].iter().all(|&u| u == d[1])
            {
                out.reason = Some("candidate-endpoint-basis-unproven");
                return Ok(out);
            }
            for a in 0..2 {
                if sum_sign(&[
                    (p.translation[a], 1.),
                    (-f64::from(p.tile[a]), chart[a][1]),
                    (f64::from(p.tile[a]), chart[a][0]),
                ]) != std::cmp::Ordering::Equal
                {
                    out.reason = Some("exact-source-tile-unproven");
                    return Ok(out);
                }
                if c.control_points
                    .iter()
                    .any(|v| v[a] < chart[a][0] || v[a] > chart[a][1])
                {
                    out.reason = Some("exact-source-chart-hull-unproven");
                    return Ok(out);
                }
            }
        }
        let remaining = limits.max_links - links;
        if remaining == 0 {
            out.reason = Some("exact-source-chain-budget");
            return Ok(out);
        }
        let connectivity =
            inspect_connectivity(partition, chart, Some(*winding), remaining.min(257))?;
        links += connectivity.links.len();
        if connectivity.status != LinkStatus::Exact {
            out.reason = Some("exact-source-closure-unproven");
            return Ok(out);
        }
    }
    let assembled =
        double_chart::assemble_candidate(partitions, chart, anchor, limits, false, || {
            let loops = sources.iter().map(|c| vec![c.clone()]).collect::<Vec<_>>();
            inspect_periodic_winding(
                &loops,
                chart,
                anchor,
                limits.tolerance_uv,
                limits.max_images,
                limits.max_cells,
            )
        })?;
    out.calibration = assembled.calibration;
    out.reason = assembled.reason;
    if !assembled.chart_winding_preserved {
        return Ok(out);
    }
    out.materialized = assembled.graph;
    out.graph = out.materialized.clone().and_then(weighted::compact);
    let Some(graph) = &out.graph else {
        out.reason = Some("exact-source-weighted-flow-unproven");
        return Ok(out);
    };
    let edges = graph
        .edges
        .iter()
        .map(|e| super::ChartEdge {
            curve: e.curve.clone(),
            source: None,
        })
        .collect::<Vec<_>>();
    // The dictionary retains geometric branches once; integer multiplicity
    // stays in the weighted graph. Do not fabricate duplicate contact rays.
    if !edges.is_empty() {
        out.contacts = Some(inspect_contact_ownership(&edges, 1024)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::periodic_trim::Piece;
    #[test]
    fn cached_identity_calibration_and_contact_receipts_are_replayed() {
        let sources = [-1., 1.]
            .into_iter()
            .map(|s| Curve {
                degree: 3,
                knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
                control_points: vec![
                    vec![0.5, 0.5],
                    vec![0.5 + s * 0.25, 0.5],
                    vec![0.5, 0.5 + s * 0.25],
                    vec![0.5, 0.5],
                ],
                weights: vec![1.; 4],
                periodic: false,
            })
            .collect::<Vec<_>>();
        let partitions = sources
            .iter()
            .map(|c| Report {
                pieces: vec![Piece {
                    curve: c.clone(),
                    source_domain: c.domain(),
                    translation: [0.; 2],
                    tile: [0; 2],
                }],
                candidate_hulls_contained: false,
            })
            .collect::<Vec<_>>();
        let limits = ChartAssemblyLimits {
            max_segments: 64,
            max_links: 32,
            max_edges: 32,
            max_images: 256,
            max_cells: 30000,
            tolerance_uv: 1e-8,
        };
        let owner = assemble_exact_source_chart(
            &sources,
            &partitions,
            &[[0, 0]; 2],
            [[0., 1.]; 2],
            [0.53, 0.53],
            limits,
            1_000_000,
        )
        .unwrap();
        owner.reverify().unwrap();
        let mut bad = owner.clone();
        bad.identities.pop();
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.work -= 1;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.calibration.as_mut().unwrap().offset = Some(123);
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.contacts.as_mut().unwrap().complete = false;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.graph.as_mut().unwrap().edges[0].curve.weights[1] = 2.;
        assert!(bad.reverify().is_err());
        let mut bad = owner;
        bad.sources[0].control_points[1][0] = 0.25_f64.next_up();
        assert!(bad.reverify().is_err());
    }
}
