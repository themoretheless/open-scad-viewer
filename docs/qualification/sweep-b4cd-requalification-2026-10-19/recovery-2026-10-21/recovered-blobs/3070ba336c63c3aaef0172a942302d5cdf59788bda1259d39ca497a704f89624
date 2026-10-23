//! Combined retained-chart and inter-chart evidence. Shell containment and
//! cap ownership are separate obligations, so this report is not a Solid proof.
use crate::{Result, check, surface::Surface, surface_monotonicity, sweep_pair_audit};

#[derive(Clone, Debug)]
pub struct Report {
    pub charts_and_pairs_certified: bool,
    pub injectivity_cells: usize,
    pub unresolved_charts: Vec<usize>,
    /// One report per input chart, in input order; retains both successful
    /// projections and unresolved reasons instead of just aggregate flags.
    pub charts: Vec<surface_monotonicity::Report>,
    pub declared_boundaries_c0: bool,
    pub c0_boundaries: Vec<[usize; 2]>,
    pub unresolved_boundaries: Vec<[usize; 2]>,
    pub pairs: sweep_pair_audit::Report,
}

/// The injectivity budget is shared across all charts, independently of the
/// pair hierarchy and pair refinement budgets. No partial chart certificate
/// promotes the aggregate result.
pub fn inspect(
    walls: &[Surface],
    shared: &[[usize; 2]],
    clearance: f64,
    distance_tolerance: f64,
    max_injectivity_cells: usize,
    max_pairs: usize,
    max_pair_cells: usize,
) -> Result<Report> {
    check(
        max_injectivity_cells <= 100000,
        "Invalid injectivity budget",
    )?;
    let pairs = sweep_pair_audit::inspect(
        walls,
        shared,
        clearance,
        distance_tolerance,
        max_pairs,
        max_pair_cells,
    )?;
    let mut c0_boundaries = Vec::new();
    let mut unresolved_boundaries = Vec::new();
    let declarations: std::collections::BTreeSet<_> = shared.iter().copied().collect();
    for pair in declarations {
        if retained_boundary_c0(&walls[pair[0]], &walls[pair[1]]) {
            c0_boundaries.push(pair);
        } else {
            unresolved_boundaries.push(pair);
        }
    }
    let mut injectivity_cells = 0;
    let mut unresolved_charts = Vec::new();
    let mut charts = Vec::with_capacity(walls.len());
    for (index, wall) in walls.iter().enumerate() {
        let report =
            surface_monotonicity::inspect(wall, max_injectivity_cells - injectivity_cells)?;
        injectivity_cells += report.cells;
        if !report.certified {
            unresolved_charts.push(index);
        }
        charts.push(report);
    }
    Ok(Report {
        charts_and_pairs_certified: unresolved_charts.is_empty()
            && pairs.all_pairs_compatible
            && unresolved_boundaries.is_empty(),
        injectivity_cells,
        unresolved_charts,
        charts,
        declared_boundaries_c0: unresolved_boundaries.is_empty(),
        c0_boundaries,
        unresolved_boundaries,
        pairs,
    })
}

/// Exact equality of clamped tensor-boundary rational coefficient data
/// proves C0 along the complete common parameter interval, independently of
/// chart injectivity or intersection exclusion. Does not claim G1/G2.
fn retained_boundary_c0(a: &Surface, b: &Surface) -> bool {
    !sweep_pair_audit::matching_boundaries(a, b).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plane(x: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![x, 0., 0.], vec![x, 1., 0.]],
                vec![vec![x + 1., 0., 0.], vec![x + 1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn separation_does_not_mask_folded_chart_or_shared_budget_exhaustion() {
        let walls = [plane(0.), plane(3.)];
        let r = inspect(&walls, &[], 0., 0.001, 2, 100, 100).unwrap();
        assert!(r.charts_and_pairs_certified);
        let r = inspect(&walls, &[], 0., 0.001, 1, 100, 100).unwrap();
        assert!(!r.charts_and_pairs_certified);
        assert_eq!(r.injectivity_cells, 1);
        assert_eq!(r.unresolved_charts, vec![1]);
        assert!(r.charts[0].certified && r.charts[0].projection.is_some());
        assert_eq!(r.charts[1].reason, Some("cell-budget-exhausted"));
        let mut folded = plane(0.);
        folded.knots_u = vec![0., 0., 0.5, 1., 1.];
        folded.control_points.push(folded.control_points[0].clone());
        folded.weights.push(vec![1.; 2]);
        let r = inspect(&[folded, plane(3.)], &[], 0., 0.001, 100, 100, 100).unwrap();
        assert!(r.pairs.all_pairs_separated);
        assert!(!r.charts_and_pairs_certified);
        assert!(r.injectivity_cells <= 100);
        assert_eq!(r.unresolved_charts, vec![0]);
        assert!(r.charts[1].certified);
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_boundary_preserves_evidence_and_independent_budget_refusals() {
        use value_codec::json;
        let request = json!({
            "op": "sweep_wall_audit", "walls": [plane(0.), plane(3.)],
            "sharedBoundaries": [], "clearance": 0., "distanceTolerance": 0.001,
            "maxInjectivityCells": 2, "maxPairs": 100, "maxPairCells": 100,
        });
        let report = crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(report["chartsAndPairsCertified"], json!(true));
        assert_eq!(report["globalEmbeddingCertified"], json!(false));
        assert_eq!(report["charts"][0]["certified"], json!(true));
        for field in ["maxInjectivityCells", "maxPairs"] {
            let mut exhausted = request.clone();
            exhausted[field] = json!(0);
            let report = crate::transport::dispatch(exhausted).unwrap();
            assert_eq!(report["chartsAndPairsCertified"], json!(false));
            assert_eq!(report["globalEmbeddingCertified"], json!(false));
            if field == "maxInjectivityCells" {
                assert_eq!(report["unresolvedCharts"], json!([0, 1]));
                assert_eq!(report["charts"][0]["reason"], json!("cell-budget-exhausted"));
                assert_eq!(report["pairs"]["allPairsSeparated"], json!(true));
            } else {
                assert_eq!(report["charts"][0]["certified"], json!(true));
                assert_eq!(report["pairs"]["unresolved"][0]["patches"], json!([0, 1]));
            }
        }
        let mut invalid = request;
        invalid["sharedBoundaries"] = json!([[0, 2]]);
        assert!(crate::transport::dispatch(invalid).is_err());
    }
    #[cfg(feature = "transport")]
    #[test]
    fn constructor_transport_derives_path_neighbors_and_rejects_correspondence_changes() {
        use value_codec::json;
        let curve = |points: Vec<Vec<f64>>| crate::curve::Curve {
            degree: 1, knots: vec![0., 0., 1., 1.], control_points: points,
            weights: vec![1., 1.], periodic: false,
        };
        let mut request = json!({
            "op": "curve_progressive_miter_level",
            "profiles": [curve(vec![vec![0.1,0.,0.],vec![0.2,0.,0.]])],
            "points": [[0.,0.,0.],[0.,0.,10.]],
            "scale": curve(vec![vec![1.,0.,0.],vec![1.,0.,0.]]),
            "twist": curve(vec![vec![0.,0.,0.],vec![0.,0.,0.]]),
            "normal": [1.,0.,0.], "closed": false, "miter_limit": 4.,
            "initial_steps": 1, "max_steps": 4, "max_deviation": 0.001,
            "preview_steps": 3,
        });
        let preview = crate::transport::dispatch(request.clone()).unwrap();
        request["op"] = json!("curve_progressive_miter_wall_audit");
        request["sections"] = preview["sections"].clone();
        request["clearance"] = json!(0.);
        request["distanceTolerance"] = json!(0.001);
        request["maxInjectivityCells"] = json!(100);
        request["maxPairs"] = json!(100);
        request["maxPairCells"] = json!(100);
        let report = crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(report["chartsAndPairsCertified"], json!(true));
        assert_eq!(report["c0Boundaries"], json!([[0,1],[1,2]]));
        assert_eq!(report["globalEmbeddingCertified"], json!(false));
        let mut loops = request.clone();
        loops["profiles"] = json!([
            curve(vec![vec![0.,0.,0.],vec![1.,0.,0.]]),
            curve(vec![vec![1.,0.,0.],vec![1.,1.,0.]]),
            curve(vec![vec![1.,1.,0.],vec![0.,1.,0.]]),
            curve(vec![vec![0.,1.,0.],vec![0.,0.,0.]])
        ]);
        loops["op"] = json!("curve_progressive_miter_level");
        loops["preview_steps"] = json!(1);
        loops["sections"] = crate::transport::dispatch(loops.clone()).unwrap()["sections"].clone();
        loops["op"] = json!("curve_progressive_miter_wall_audit");
        loops["loopSizes"] = json!([4]);
        let report = crate::transport::dispatch(loops.clone()).unwrap();
        assert_eq!(report["c0Boundaries"], json!([[0,1],[0,3],[1,2],[2,3]]));
        assert_eq!(report["pairs"]["allPairsCompatible"], json!(true));
        assert_eq!(report["chartsAndPairsCertified"], json!(true));
        assert_eq!(report["globalEmbeddingCertified"], json!(false));
        let mut disjoint = loops.clone();
        let mut profiles = value_codec::from_value::<Vec<crate::curve::Curve>>(disjoint["profiles"].clone()).unwrap();
        let translated: Vec<_> = profiles.iter().cloned().map(|mut c| {
            for p in &mut c.control_points { p[0] += 3.; } c
        }).collect();
        profiles.extend(translated);
        disjoint["profiles"] = json!(profiles);
        disjoint["op"] = json!("curve_progressive_miter_level");
        disjoint["sections"] = crate::transport::dispatch(disjoint.clone()).unwrap()["sections"].clone();
        disjoint["op"] = json!("curve_progressive_miter_wall_audit");
        disjoint["loopSizes"] = json!([4,4]);
        let report = crate::transport::dispatch(disjoint).unwrap();
        let owned: Vec<[usize;2]> = value_codec::from_value(report["c0Boundaries"].clone()).unwrap();
        assert_eq!(owned.len(),8);
        assert!(owned.iter().all(|pair| (pair[0]<4) == (pair[1]<4)));
        for sizes in [json!([2,2]),json!([0,4]),json!([3])] {
            loops["loopSizes"] = sizes;
            assert!(crate::transport::dispatch(loops.clone()).is_err());
        }
        request["sections"][1][0]["weights"][0] = json!(2.);
        assert!(crate::transport::dispatch(request).is_err());
    }
    #[test]
    fn disconnected_parameter_chart_is_rejected_before_audit() {
        let mut s = plane(0.);
        s.knots_u = vec![0., 0., 0.5, 0.5, 1., 1.];
        s.control_points.extend(plane(3.).control_points);
        s.weights.extend(vec![vec![1.; 2]; 2]);
        let error = inspect(&[s], &[], 0., 0.001, 100, 100, 100).unwrap_err();
        assert_eq!(error.code, "NURBS_INVALID_INPUT");
        assert!(error.message.contains("Interior multiplicity"));
    }
    #[test]
    fn declared_c0_boundary_is_independent_of_pair_separation() {
        let a = plane(0.);
        let mut b = a.clone();
        for row in &mut b.control_points {
            for point in row {
                point[1] += 1.;
            }
        }
        let r = inspect(&[a.clone(), b.clone()], &[[0, 1]], 0., 0.001, 100, 100, 100).unwrap();
        assert!(r.charts_and_pairs_certified && r.declared_boundaries_c0);
        assert_eq!(r.c0_boundaries, vec![[0, 1]]);
        for row in &mut b.control_points {
            for point in row {
                point[1] += 0.25;
            }
        }
        let separated = inspect(&[a.clone(), b.clone()], &[], 0., 0.001, 100, 100, 100).unwrap();
        assert!(separated.charts_and_pairs_certified && separated.pairs.all_pairs_separated);
        let r = inspect(&[a, b], &[[0, 1]], 0., 0.001, 100, 100, 100).unwrap();
        assert!(!r.charts_and_pairs_certified && !r.declared_boundaries_c0);
        assert_eq!(r.unresolved_boundaries, vec![[0, 1]]);
    }
}
