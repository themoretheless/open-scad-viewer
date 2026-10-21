//! Original chain transfer plus actual candidate-edge nonlinear image aliases.
use super::*;
use crate::periodic_trim::{ChartEdge, CurveClockCorrespondence, clock};
#[derive(Clone, Debug)]
pub struct ClockAliasSpec {
    pub edges: [usize; 2],
    pub clock: Curve,
}
#[derive(Clone, Debug)]
pub struct ClockMappedSourceChart {
    source: ExactSourceChart,
    specs: Vec<ClockAliasSpec>,
    proofs: Vec<CurveClockCorrespondence>,
    graph: Option<WeightedChartGraph>,
    contacts: Option<ContactOwnershipReport>,
    work: u64,
    reason: Option<&'static str>,
}
impl ClockMappedSourceChart {
    pub fn source(&self) -> &ExactSourceChart {
        &self.source
    }
    pub fn proofs(&self) -> &[CurveClockCorrespondence] {
        &self.proofs
    }
    pub fn graph(&self) -> Option<&WeightedChartGraph> {
        self.graph.as_ref()
    }
    pub fn contact_ownership(&self) -> Option<&ContactOwnershipReport> {
        self.contacts.as_ref()
    }
    pub fn work_used(&self) -> u64 {
        self.work
    }
    pub fn reason(&self) -> Option<&'static str> {
        self.reason
    }
    pub fn reverify(&self) -> Result<()> {
        let s = &self.source;
        let fresh = assemble_clock_mapped_source_chart(
            &s.sources,
            &s.partitions,
            &s.windings,
            s.chart,
            s.anchor,
            s.limits,
            &self.specs,
            s.max_work,
        )?;
        check(
            self.graph.is_some()
                && self.graph == fresh.graph
                && self.contacts == fresh.contacts
                && self.proofs == fresh.proofs
                && self.work == fresh.work
                && self.reason == fresh.reason
                && s.identities == fresh.source.identities
                && s.calibration == fresh.source.calibration
                && s.graph == fresh.source.graph
                && s.materialized == fresh.source.materialized
                && s.contacts == fresh.source.contacts
                && s.work == fresh.source.work
                && s.reason == fresh.source.reason,
            "Original clock-mapped chart graph, source bindings or work changed",
        )
    }
}
/// Exact original-function restriction transfer followed by exact nonlinear
/// candidate image compaction. Alias indices refer to the actual materialized_graph AFTER winding correction,
/// not a caller-authored substitute dictionary. All actual original restriction
/// and actual candidate-clock predicates share max_work. Signed source roles,
/// cancellations and multiplicity remain in the weighted graph. Endpoint angular
/// ownership is recomputed AFTER aliases; complete interior/region/face proofs
/// remain separate. No disjoint-source isotopy claim is manufactured.
pub fn assemble_clock_mapped_source_chart(
    sources: &[Curve],
    partitions: &[Report],
    windings: &[[i32; 2]],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ChartAssemblyLimits,
    specs: &[ClockAliasSpec],
    max_work: u64,
) -> Result<ClockMappedSourceChart> {
    check(
        specs.len() <= 256,
        "Too many actual candidate clock bindings",
    )?;
    let source = assemble_exact_source_chart(
        sources, partitions, windings, chart, anchor, limits, max_work,
    )?;
    let mut out = ClockMappedSourceChart {
        work: source.work_used(),
        source,
        specs: specs.to_vec(),
        proofs: Vec::new(),
        graph: None,
        contacts: None,
        reason: None,
    };
    if out.source.graph.is_none() {
        out.reason = Some("original-chain-transfer-unproven");
        return Ok(out);
    }
    let raw = out
        .source
        .materialized
        .as_ref()
        .ok_or_else(|| crate::input("Missing actual corrected original chart graph"))?;
    for spec in specs {
        check(
            spec.edges[0] < raw.edges.len()
                && spec.edges[1] < raw.edges.len()
                && spec.edges[0] != spec.edges[1],
            "Clock binding references missing or identical actual raw edges",
        )?;
        let a = &raw.edges[spec.edges[0]].curve;
        let b = &raw.edges[spec.edges[1]].curve;
        let (proof, used) = clock::verify_with_work(a, b, &spec.clock, max_work - out.work)?;
        out.work += used;
        let Some(proof) = proof else {
            out.reason = Some("actual-candidate-clock-unproven");
            return Ok(out);
        };
        out.proofs.push(proof);
    }
    out.graph = weighted::compact_with(raw.clone(), |group, edge| {
        out.proofs.iter().find_map(|p| {
            let [a, b] = p.sources();
            ((&group.curve == a && &edge.curve == b) || (&group.curve == b && &edge.curve == a))
                .then_some(p.orientation())
        })
    });
    let Some(graph) = &out.graph else {
        out.reason = Some("mapped-weighted-flow-unproven");
        return Ok(out);
    };
    let edges = graph
        .edges
        .iter()
        .map(|e| ChartEdge {
            curve: e.curve.clone(),
            source: None,
        })
        .collect::<Vec<_>>();
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
    fn actual_mapped_graph_and_original_chain_receipts_are_independently_replayed() {
        let a = Curve {
            degree: 3,
            knots: [vec![0.; 4], vec![1.; 4]].concat(),
            control_points: vec![vec![0., 0.], vec![5., 0.], vec![0., 5.], vec![0., 0.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let b = Curve {
            degree: 6,
            knots: [vec![0.; 7], vec![1.; 7]].concat(),
            control_points: vec![
                vec![0., 0.],
                vec![0., 0.],
                vec![1., 0.],
                vec![3., 0.],
                vec![4., 1.],
                vec![0., 5.],
                vec![0., 0.],
            ],
            weights: vec![1.; 7],
            periodic: false,
        };
        let clock = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![0., 0.], vec![1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let parts = |c: &Curve, cut| Report {
            pieces: c
                .split(cut)
                .unwrap()
                .into_iter()
                .map(|curve| Piece {
                    source_domain: curve.domain(),
                    curve,
                    translation: [0.; 2],
                    tile: [0; 2],
                })
                .collect(),
            candidate_hulls_contained: false,
        };
        let partitions = vec![parts(&a, 0.25), parts(&b, 0.5)];
        let sources = vec![a, b];
        let clocks = clock.split(0.5).unwrap();
        let specs = vec![
            ClockAliasSpec {
                edges: [0, 2],
                clock: clocks[0].clone(),
            },
            ClockAliasSpec {
                edges: [1, 3],
                clock: clocks[1].clone(),
            },
        ];
        let limits = ChartAssemblyLimits {
            max_segments: 64,
            max_links: 32,
            max_edges: 32,
            max_images: 256,
            max_cells: 30000,
            tolerance_uv: 1e-8,
        };
        let owner = assemble_clock_mapped_source_chart(
            &sources,
            &partitions,
            &[[0, 0]; 2],
            [[0., 8.]; 2],
            [1., 1.],
            limits,
            &specs,
            1_000_000,
        )
        .unwrap();
        owner.reverify().unwrap();
        let mut bad = owner.clone();
        bad.proofs.pop();
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.source.identities.clear();
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.source.calibration.as_mut().unwrap().offset = Some(123);
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.source.materialized = None;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.graph.as_mut().unwrap().edges[0].coefficient = 1;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.graph.as_mut().unwrap().edges[0].contributions.pop();
        assert!(bad.reverify().is_err());
        let mut bad = owner;
        bad.work -= 1;
        assert!(bad.reverify().is_err());
    }
}
