use brep_core::{Model, edge_partition, face_crossings};
use nurbs_core::{curve::Curve, trim_domain::Location};
fn polygon(points: &[[f64; 2]]) -> Vec<Curve> {
    (0..points.len())
        .map(|i| {
            Curve::from_polyline(vec![
                vec![points[i][0], points[i][1], 1.],
                vec![
                    points[(i + 1) % points.len()][0],
                    points[(i + 1) % points.len()][1],
                    1.,
                ],
            ])
            .unwrap()
        })
        .collect()
}
fn support(hole: Option<[[f64; 2]; 2]>, triangle: bool) -> Model {
    let outer = if triangle {
        vec![[-20., -20.], [20., -20.], [-20., 10.]]
    } else {
        vec![[-20., -20.], [20., -20.], [20., 20.], [-20., 20.]]
    };
    let mut loops = vec![polygon(&outer)];
    if let Some([[a, b], [c, d]]) = hole {
        loops.push(polygon(&[[a, c], [a, d], [b, d], [b, c]]));
    }
    let mut model = brep_core::planar_cap::build_sheet(&loops, 1., 1e-6, 100000).unwrap();
    fn rotate(p: &mut [f64]) {
        let old = p.to_vec();
        p[0] = old[2];
        p[1] = old[0];
        p[2] = old[1];
    }
    for v in &mut model.vertices {
        rotate(&mut v.point);
    }
    for e in &mut model.edges {
        for p in &mut e.curve.control_points {
            rotate(p);
        }
    }
    for f in &mut model.faces {
        for row in &mut f.surface.control_points {
            for p in row {
                rotate(p);
            }
        }
    }
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}
fn source() -> (Model, usize) {
    let model = brep_core::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    let edge = model
        .edges
        .iter()
        .position(|e| {
            let a = model.vertices[e.vertices[0]].point;
            let b = model.vertices[e.vertices[1]].point;
            a[1] == 0. && b[1] == 0. && a[2] == 0. && b[2] == 0. && a[0] != b[0]
        })
        .unwrap();
    (model, edge)
}
#[test]
fn roots_inside_holes_and_outside_outer_contour_do_not_author_a_cut() {
    let (model, edge) = source();
    for cutter in [
        support(Some([[-2., 2.], [-2., 2.]]), false),
        support(None, true),
    ] {
        let before = (model.clone(), cutter.clone());
        let report = face_crossings::inspect(&model, edge, &cutter, 0, 1e-8, 100000).unwrap();
        assert!(report.complete, "{report:?}");
        assert!(report.native_complete);
        assert_eq!(report.roots.len(), 1);
        assert_eq!(report.roots[0].location, Location::Outside);
        assert!(report.eligible.is_empty());
        assert!(report.cells <= 100000);
        assert!(
            edge_partition::partition_at_face(&model, edge, &cutter, 0, 0, 1e-8, 100000).is_err()
        );
        assert_eq!((model.clone(), cutter), before);
    }
}
#[test]
fn valid_trimmed_face_root_splits_both_faces_and_respects_oriented_shell_use() {
    let (model, edge) = source();
    let cutter = support(Some([[4., 6.], [4., 6.]]), false);
    for reversed in [false, true] {
        let mut cut = cutter.clone();
        cut.shells[0].faces[0].reversed = reversed;
        cut.rebuild_topology_ids();
        let before = (model.clone(), cut.clone());
        let out =
            edge_partition::partition_at_face(&model, edge, &cut, 0, 0, 1e-8, 100000).unwrap();
        assert!(out.coverage.complete);
        assert_eq!(out.coverage.eligible.len(), 1);
        assert_eq!(
            out.result.crossing_sign,
            Some(if reversed { -1 } else { 1 })
        );
        assert_eq!(out.coverage.support_face_identity, cut.1.faces[0]);
        assert!(out.result.partition.cells <= 100000);
        for child in out.result.partition.child_edges {
            assert_eq!(
                out.result
                    .partition
                    .model
                    .loops
                    .iter()
                    .flat_map(|l| &l.coedges)
                    .filter(|c| c.edge == child)
                    .count(),
                2
            );
        }
        assert_eq!((model.clone(), cut), before);
    }
}
#[test]
fn root_on_hole_boundary_and_exhausted_membership_never_author_a_cut() {
    let (model, edge) = source();
    let cut = support(Some([[-2., 0.], [-2., 2.]]), false);
    let r = face_crossings::inspect(&model, edge, &cut, 0, 1e-8, 100000).unwrap();
    assert!(!r.complete, "{r:?}");
    assert_eq!(r.roots[0].location, Location::Unresolved);
    assert!(edge_partition::partition_at_face(&model, edge, &cut, 0, 0, 1e-8, 100000).is_err());
    assert!(edge_partition::partition_at_face(&model, edge, &cut, 0, 0, 1e-8, 1).is_err());
}

fn parabolic_support(hole: [[f64; 2]; 2]) -> Model {
    let mut model = support(Some(hole), false);
    model.faces[0].surface = nurbs_core::surface::Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., -20., 25.], vec![0., 20., 25.]],
            vec![vec![5., -20., -25.], vec![5., 20., -25.]],
            vec![vec![10., -20., 25.], vec![10., 20., 25.]],
        ],
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let uses: Vec<_> = model
        .loops
        .iter()
        .flat_map(|l| l.coedges.iter().cloned())
        .collect();
    let world = |u: f64, v: f64| vec![10. * u, -20. + 40. * v, 100. * u * u - 100. * u + 25.];
    for c in uses {
        let p = &c.pcurve.control_points[0];
        let q = c.pcurve.control_points.last().unwrap();
        let start = world(p[0], p[1]);
        let end = world(q[0], q[1]);
        let du = q[0] - p[0];
        let middle = vec![
            (start[0] + end[0]) * 0.5,
            (start[1] + end[1]) * 0.5,
            start[2] + (100. * p[0] - 50.) * du,
        ];
        let mut curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![start, middle, end],
            weights: vec![1.; 3],
            periodic: false,
        };
        if c.reversed {
            curve = curve.reverse().unwrap();
        }
        let vertices = model.edges[c.edge].vertices;
        model.vertices[vertices[0]].point = curve.control_points[0].clone().try_into().unwrap();
        model.vertices[vertices[1]].point = curve.control_points[2].clone().try_into().unwrap();
        model.edges[c.edge].curve = curve;
    }
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}
#[test]
fn unresolved_tangent_surface_cells_are_excluded_only_when_wholly_inside_a_hole() {
    let (model, edge) = source();
    for (hole, empty_face_cover) in [
        ([[-2., 2.], [-2., 2.]], true),
        ([[4., 6.], [4., 6.]], false),
    ] {
        let cutter = parabolic_support(hole);
        let before = cutter.clone();
        let report = face_crossings::inspect(&model, edge, &cutter, 0, 1e-8, 1000000).unwrap();
        assert!(!report.native_complete);
        assert!(!report.unresolved.is_empty());
        assert_eq!(report.complete, empty_face_cover, "{report:?}");
        assert!(report.eligible.is_empty());
        assert!(report.cells <= 1000000);
        if empty_face_cover {
            assert!(
                report
                    .unresolved
                    .iter()
                    .all(|c| c.location == Location::Outside)
            );
        }
        assert_eq!(cutter, before);
    }
}

#[test]
fn retained_crossing_order_excludes_only_the_root_inside_a_hole() {
    let (model, edge) = source();
    let mut cutter = parabolic_support([[-14., -10.], [-2., 2.]]);
    for v in &mut cutter.vertices {
        v.point[2] -= 9.;
    }
    for e in &mut cutter.edges {
        for p in &mut e.curve.control_points {
            p[2] -= 9.;
        }
    }
    for row in &mut cutter.faces[0].surface.control_points {
        for p in row {
            p[2] -= 9.;
        }
    }
    cutter.rebuild_topology_ids();
    cutter.validate().unwrap();
    let report = face_crossings::inspect(&model, edge, &cutter, 0, 1e-8, 1000000).unwrap();
    assert!(report.native_complete && report.complete, "{report:?}");
    assert_eq!(report.roots.len(), 2);
    assert_eq!(report.roots[0].location, Location::Outside);
    assert_eq!(report.roots[1].location, Location::Inside);
    assert_eq!(report.eligible, vec![1]);
    let out =
        edge_partition::partition_at_face(&model, edge, &cutter, 0, 0, 1e-8, 1000000).unwrap();
    assert!(
        (out.result.partition.model.vertices[out.result.partition.vertex].point[0] - 8.).abs()
            <= model.tolerance_mm
    );
    assert!(edge_partition::partition_at_face(&model, edge, &cutter, 0, 1, 1e-8, 1000000).is_err());
}

#[test]
fn all_retained_roots_partition_atomically_and_empty_coverage_is_unchanged() {
    let (model, edge) = source();
    let before = model.clone();
    for hole in [None, Some([[-2., 2.], [-2., 2.]])] {
        let cutter = support(hole, false);
        let out = edge_partition::partition_all_at_face(&model, edge, &cutter, 0, 1e-8, 100000).unwrap();
        assert!(out.coverage.complete);
        assert!(out.cells <= 100000);
        assert_eq!(out.edges.len(), out.vertices.len() + 1);
        if hole.is_some() {
            assert!(out.vertices.is_empty());
            assert_eq!(out.model, model);
        } else {
            assert_eq!(out.vertices.len(), 1);
            assert_eq!(out.model.edges[out.edges[0]].vertices[1], out.vertices[0]);
            assert_eq!(out.model.edges[out.edges[1]].vertices[0], out.vertices[0]);
            assert_eq!(brep_core::watertight::inspect(&out.model, 100000).unwrap().status,
                brep_core::watertight::Closure::WithinTolerance);
        }
    }
    let ambiguous = support(Some([[-2., 0.], [-2., 2.]]), false);
    assert!(edge_partition::partition_all_at_face(&model, edge, &ambiguous, 0, 1e-8, 100000).is_err());
    assert!(edge_partition::partition_all_at_face(&model, edge, &support(None, false), 0, 1e-8, 1).is_err());
    assert_eq!(model, before);
}

#[test]
fn all_two_curved_face_roots_share_vertices_and_original_parameter_order() {
    let (model, edge) = source();
    let mut cutter = parabolic_support([[4., 6.], [4., 6.]]);
    for v in &mut cutter.vertices { v.point[2] -= 9.; }
    for e in &mut cutter.edges { for p in &mut e.curve.control_points { p[2] -= 9.; } }
    for row in &mut cutter.faces[0].surface.control_points { for p in row { p[2] -= 9.; } }
    cutter.rebuild_topology_ids();
    let before = (model.clone(), cutter.clone());
    let out = edge_partition::partition_all_at_face(&model, edge, &cutter, 0, 1e-8, 1000000).unwrap();
    assert_eq!(out.coverage.eligible.len(), 2);
    assert_eq!(out.edges.len(), 3);
    assert_eq!(out.vertices.len(), 2);
    for (i, expected) in [2., 8.].into_iter().enumerate() {
        assert!((out.model.vertices[out.vertices[i]].point[0] - expected).abs() <= model.tolerance_mm);
        assert_eq!(out.model.edges[out.edges[i]].vertices[1], out.vertices[i]);
        assert_eq!(out.model.edges[out.edges[i + 1]].vertices[0], out.vertices[i]);
    }
    for &e in &out.edges {
        assert_eq!(out.model.loops.iter().flat_map(|l| &l.coedges).filter(|c| c.edge == e).count(), 2);
    }
    assert!(out.cells <= 1000000);
    assert!(out.model.persistent_naming_complete());
    let original_domain = model.edges[edge].curve.domain();
    assert_eq!(out.model.edges[out.edges[0]].curve.domain()[0], original_domain[0]);
    assert_eq!(out.model.edges[*out.edges.last().unwrap()].curve.domain()[1], original_domain[1]);
    for pair in out.edges.windows(2) {
        assert_eq!(out.model.edges[pair[0]].curve.domain()[1], out.model.edges[pair[1]].curve.domain()[0]);
    }
    assert!(out.cells > out.coverage.cells);
    assert!(edge_partition::partition_all_at_face(&model, edge, &cutter, 0, 1e-8, 1).is_err());
    assert_eq!(brep_core::watertight::inspect(&out.model, 100000).unwrap().status,
        brep_core::watertight::Closure::WithinTolerance);
    assert_eq!((model, cutter), before);
}
