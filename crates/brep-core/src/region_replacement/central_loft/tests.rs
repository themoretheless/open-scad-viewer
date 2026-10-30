use super::*;
use crate::region_replacement::{EdgeRadius, StripLimits, build_trihedral_strips};
fn strip_limits() -> StripLimits {
    StripLimits {
        tolerances: [1e-7, 1e-5, 1e-4],
        tolerance_uv: 1e-8,
        max_geometry_work: 10000,
        max_cells: 300000,
        max_exact: 1000000,
        max_coefficients: 2000000,
    }
}
fn options() -> CentralLoftOptions {
    CentralLoftOptions {
        handle_mm: 1. / 256.,
        tolerances: [1e-7, 1e-5, 1e-4],
        tolerance_uv: 1e-8,
        operations: 1000000,
        cells: 100000,
    }
}
#[test]
fn unequal_radius_center_preserves_the_whole_rim_and_stays_inside_original_stock() {
    let source = crate::cuboid([0.; 3], [10.; 3]).unwrap();
    let saved = source.clone();
    let graph = crate::graph_body::tests::limits();
    let budget = crate::source_box::Limits {
        operations: 1000000,
        body: graph.body,
    };
    let body = crate::source_box::qualify(&source, &budget).unwrap();
    let requested: Vec<_> = source
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.vertices.contains(&0))
        .enumerate()
        .map(|(i, (edge, _))| EdgeRadius {
            edge,
            radius_mm: [0.1, 0.2, 0.35][i],
        })
        .collect();
    let strips =
        build_trihedral_strips(&source, 0, 0, &requested, 0.1, 2., strip_limits()).unwrap();
    let candidate = propose_trihedral_center(&body, &strips, options()).unwrap();
    assert_eq!(candidate.curves().len(), 6);
    assert_eq!(candidate.pcurves().len(), 6);
    assert_eq!(candidate.positions().len(), 12);
    assert!(
        candidate
            .positions()
            .iter()
            .all(|r| r.status == agreement::Status::WithinTolerance)
    );
    assert_eq!(candidate.stock_bounds(), [[0., 10.]; 3]);
    let actual = candidate.control_bounds();
    assert!(actual.iter().all(|b| b[0] >= 0. && b[1] <= 10.));
    let s = candidate.surface();
    assert!(s.control_points.len() * s.control_points[0].len() <= 4096);
    let mut cached = nurbs_core::surface::SurfaceEvaluator::new(s).unwrap();
    for u in 0..21 {
        for v in 0..21 {
            let p = s.evaluate(u as f64 / 20., v as f64 / 20.).unwrap().point;
            assert!(
                p.iter().all(|&x| x >= 0. && x <= 10.),
                "u={u} v={v} p={p:?}"
            );
            let q = cached
                .evaluate(u as f64 / 20., v as f64 / 20.)
                .unwrap()
                .point;
            assert!(
                q.iter().all(|&x| x >= 0. && x <= 10.),
                "cached u={u} v={v} q={q:?}"
            );
        }
    }
    // The two pole boundaries are declared, not passed off as regular charts.
    assert!(
        s.control_points[0]
            .iter()
            .all(|p| p == &s.control_points[0][0])
    );
    assert!(
        s.control_points
            .last()
            .unwrap()
            .iter()
            .all(|p| p == &s.control_points.last().unwrap()[0])
    );
    let seams = candidate.inspect_original_g2(6000, 100000).unwrap();
    assert_eq!(seams.len(), 6);
    assert!(
        seams
            .iter()
            .map(|p| p.report.geometric.cells)
            .sum::<usize>()
            <= 6000
    );
    assert!(
        seams
            .iter()
            .map(|p| p.report.positions.exact_work)
            .sum::<u64>()
            <= 100000
    );
    for (i, proof) in seams.iter().enumerate() {
        eprintln!(
            "central original seam {i}: positions={} geometric={:?}",
            proof.report.positions.proven, proof.report.geometric
        );
    }
    super::span_diagnostics::inspect(&candidate);
    let fields = candidate
        .inspect_original_span_fields(6000, 100000)
        .unwrap();
    assert_eq!(fields.len(), 6);
    assert_eq!(
        fields[1].decision,
        nurbs_core::surface_g2_pcurves::FieldDecision::Enclosed
    );
    assert!(fields[1].one_sided_regular && fields[1].pending == 0);
    assert!(fields.iter().all(|r| r.positions.proven));
    assert!(fields.iter().map(|r| r.cells).sum::<usize>() <= 6000);
    assert!(fields.iter().map(|r| r.positions.exact_work).sum::<u64>() <= 100000);
    for (i, r) in fields.iter().enumerate() {
        eprintln!(
            "central whole original span fields {i}: positions={} decision={:?} regular={} pending={} cells={} branches={:?}",
            r.positions.proven, r.decision, r.one_sided_regular, r.pending, r.cells, r.branches
        );
    }

    let internal = candidate
        .inspect_internal_span_fields(6000, 100000)
        .unwrap();
    assert!(!internal.is_empty());
    assert!(internal.iter().all(|(_, r)| r.positions.proven));
    assert!(internal.iter().map(|(_, r)| r.cells).sum::<usize>() <= 6000);
    assert!(
        internal
            .iter()
            .map(|(_, r)| r.positions.exact_work)
            .sum::<u64>()
            <= 100000
    );
    for (u, r) in internal {
        eprintln!(
            "central internal u={u}: decision={:?} regular={} pending={} cells={} branches={:?}",
            r.decision, r.one_sided_regular, r.pending, r.cells, r.branches
        );
    }
    let joins = candidate.inspect_internal_span_joins(6000).unwrap();
    assert_eq!(joins.len(), 4);
    assert!(joins.iter().all(|(_, r)| r.decision
        == nurbs_core::surface_join::Decision::WithinTolerance
        && r.regularity_proven
        && r.pending == 0
        && r.normal[1] <= options().tolerances[1]
        && r.tensor[1] <= options().tolerances[2]));
    assert!(joins.iter().map(|(_, r)| r.cells).sum::<usize>() <= 6000);
    for (u, r) in joins {
        eprintln!(
            "central correlated internal u={u}: decision={:?} regular={} pending={} cells={} normal={:?} tensor={:?}",
            r.decision, r.regularity_proven, r.pending, r.cells, r.normal, r.tensor
        );
    }
    let poles = candidate.inspect_pole_limits(1. / 64., 6000).unwrap();
    assert_eq!(poles.len(), 2);
    assert!(
        poles
            .iter()
            .all(|p| p.report.proven && p.report.quotient_proven && p.report.differential_proven)
    );
    assert!(poles.iter().map(|p| p.report.cells).sum::<usize>() <= 6000);
    for (i, p) in poles.iter().enumerate() {
        eprintln!(
            "central pole {i}: proven={} differential={} quotient={} reason={} axis={} sign={:?} cells={} angle={:?} radius={:?} lower={:?} normal-coeff={:?} tensor-coeff={:?}",
            p.report.proven,
            p.report.differential_proven,
            p.report.quotient_proven,
            p.report.reason,
            p.report.normal_axis,
            p.report.normal_sign,
            p.report.cells,
            p.report.fan_angle_upper,
            p.report.reference_radius,
            p.report.quotient_lower,
            p.report.normal_coefficient,
            p.report.tensor_coefficient
        );
        if let Some(c) = &p.certificate {
            c.reverify().unwrap();
        } else {
            let axis = p.report.normal_axis;
            let n = s.control_points.len();
            let heights: Vec<Vec<_>> = (0..3)
                .map(|j| {
                    s.control_points[if i == 0 { j } else { n - 1 - j }]
                        .iter()
                        .map(|x| x[axis])
                        .collect()
                })
                .collect();
            eprintln!("central pole {i} original heights: {heights:?}");
        }
    }
    let exterior = candidate.inspect_exterior_jets(6000, 100000).unwrap();
    assert_eq!(exterior.len(), 6);
    for (i, r) in exterior.iter().enumerate() {
        eprintln!(
            "central exterior {i}: decision={:?} orientation={:?} regular={} cells={} pending={} normal={:?} tensor={:?} G0={}",
            r.decision,
            r.orientation,
            r.regularity_proven,
            r.cells,
            r.pending,
            r.normal,
            r.tensor,
            r.positions.proven
        );
    }
    assert!(exterior.iter().all(|r| r.decision
        == nurbs_core::surface_join::Decision::WithinTolerance
        && r.regularity_proven
        && r.pending == 0));
    candidate.reverify().unwrap();
    let mut exact = options();
    exact.operations = candidate.operations();
    assert!(propose_trihedral_center(&body, &strips, exact).is_ok());
    exact.operations -= 1;
    assert!(propose_trihedral_center(&body, &strips, exact).is_err());
    let mut outside = options();
    outside.handle_mm = 100.;
    assert!(propose_trihedral_center(&body, &strips, outside).is_err());
    assert_eq!(source, saved);
}
