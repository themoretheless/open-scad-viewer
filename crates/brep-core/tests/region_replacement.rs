use brep_core::{face_cut, region_replacement};
use nurbs_core::{curve::Curve, surface::Surface};
#[path = "../examples/support/corner_close_fixture.rs"]
mod corner_fixture;
#[test]
fn curved_uv_seams_on_a_nonplanar_chart_are_freshly_qualified_before_replacement() {
    let (open, supports, options) = corner_fixture::fixture_extended();
    let closed = brep_core::corner_close::close(
        &open,
        0,
        supports,
        options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    let chart = closed.model.faces[closed.face].surface.clone();
    let pcs = [
        vec![vec![0.25, 0.5], vec![0.5, 0.25], vec![0.75, 0.5]],
        vec![vec![0.75, 0.5], vec![0.5, 0.75], vec![0.25, 0.5]],
    ];
    let mut boundary = Vec::new();
    for poles in pcs {
        let pc = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: poles,
            weights: vec![1.; 3],
            periodic: false,
        };
        // Bernstein product of the actual authored quadratic UV polynomials:
        // x=u, y=v, z=u²/4+v²/8. Rounded proposals require fresh full lift proof.
        let bin2 = [1., 2., 1.];
        let bin4 = [1., 4., 6., 4., 1.];
        let mut points = Vec::new();
        for k in 0..5 {
            let mut p = vec![0.; 3];
            for i in 0..3 {
                for j in 0..3 {
                    if i + j == k {
                        let factor = bin2[i] * bin2[j] / bin4[k];
                        let (a, b) = (&pc.control_points[i], &pc.control_points[j]);
                        p[0] += factor * a[0];
                        p[1] += factor * a[1];
                        p[2] += factor * (a[0] * b[0] / 4. + a[1] * b[1] / 8.);
                    }
                }
            }
            points.push(p);
        }
        let curve = Curve {
            degree: 4,
            knots: [vec![0.; 5], vec![1.; 5]].concat(),
            control_points: points,
            weights: vec![1.; 5],
            periodic: false,
        };
        boundary.push(brep_core::face_window::Boundary {
            curve,
            source_pcurve: pc.clone(),
            replacement_pcurve: pc,
        });
    }
    let window = brep_core::face_window::replace(
        &closed.model,
        closed.face,
        &chart,
        &boundary,
        1e-8,
        100000,
    )
    .unwrap();
    let source = window.model;
    let before = source.clone();
    let face = window.replacement_face;
    let rim = source.loops[source.faces[face].outer].coedges.clone();
    let reversed = source.shells[0]
        .faces
        .iter()
        .find(|u| u.face == face)
        .unwrap()
        .reversed;
    let out = region_replacement::replace_g2_candidate(
        &source,
        &[face],
        &chart,
        &[rim.clone()],
        reversed,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    assert!(out.interior.certified);
    assert_eq!(out.seams.len(), 2);
    for (_, s) in &out.seams {
        assert!(s.positions.proven && s.geometric.regularity_proven);
        assert_eq!(
            s.geometric.decision,
            nurbs_core::surface_join::Decision::WithinTolerance
        );
        assert_eq!(s.geometric.branches[0].normal_upper, Some(0.));
        assert_eq!(s.geometric.branches[0].tensor_upper, Some(0.));
    }
    assert_eq!(
        out.replacement.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert!(out.cells <= 100000 && out.exact_work <= 1000000);
    let mut kink = chart.clone();
    assert_eq!((chart.degree_u, chart.degree_v), (5, 5));
    assert!(chart.weights.iter().flatten().all(|w| *w == 1.));
    let choose = |n: usize, k: usize| {
        let mut x = 1.;
        for j in 0..k {
            x *= (n - j) as f64 / (j + 1) as f64;
        }
        x
    };
    // F=(v-.5)^2-(-2u²+2u-.375)^2 vanishes on BOTH curved
    // lens arcs. Its transverse gradient changes their normals.
    let b2 = [0.140625, -1.5, 5.5, -8., 4.];
    for i in 0..6 {
        for j in 0..6 {
            let fu = 0.25
                - (0..=i.min(4))
                    .map(|k| b2[k] * choose(i, k) / choose(5, k))
                    .sum::<f64>();
            let v = j as f64;
            let fv = -v / 5. + v * (v - 1.) / 20.;
            kink.control_points[i][j][2] += 0.1 * (fu + fv);
        }
    }
    let edge = &rim[0];
    let support = source
        .loops
        .iter()
        .flat_map(|l| &l.coedges)
        .find(|c| c.edge == edge.edge && c.reversed != edge.reversed)
        .unwrap();
    let invalid = nurbs_core::surface_g2_pcurves::inspect(
        &source.edges[edge.edge].curve,
        [&support.pcurve, &edge.pcurve],
        [&chart, &kink],
        [support.reversed, edge.reversed],
        1e-7,
        1e-5,
        1e-4,
        100000,
        1000000,
    )
    .unwrap();
    assert!(invalid.positions.proven);
    assert_eq!(
        invalid.geometric.decision,
        nurbs_core::surface_join::Decision::CorrespondenceExceedsTolerance
    );
    assert!(
        region_replacement::replace_g2_candidate(
            &source,
            &[face],
            &kink,
            &[rim.clone()],
            reversed,
            1e-8,
            [1e-7, 1e-5, 1e-4],
            100000,
            1000000
        )
        .is_err()
    );
    let mut bad = chart.clone();
    bad.control_points[2][2][2] += 0.1;
    assert!(
        region_replacement::replace_g2_candidate(
            &source,
            &[face],
            &bad,
            &[rim.clone()],
            reversed,
            1e-8,
            [1e-7, 1e-5, 1e-4],
            100000,
            1000000
        )
        .is_err()
    );
    assert!(
        region_replacement::replace_g2_candidate(
            &source,
            &[face],
            &chart,
            &[rim],
            reversed,
            1e-8,
            [1e-7, 1e-5, 1e-4],
            4,
            0
        )
        .is_err()
    );
    assert_eq!(source, before);
}
#[test]
fn root_partitioned_rim_chains_keep_all_shared_edges_during_g2_replacement() {
    use brep_core::{
        corner_close::BoundaryUse,
        edge_partition::{self, PcurveCut},
    };
    let (open, supports, options) = corner_fixture::fixture_extended();
    let closed = brep_core::corner_close::close(
        &open,
        0,
        supports,
        options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    let selected = closed.face;
    let mut source = closed.model;
    let mut chains = supports.map(|u| vec![u]);
    let mut roots = Vec::new();
    for side in 0..4 {
        let edge = supports[side].edge;
        let fraction = [0.37, 0.5, 0.61, 0.5][side];
        let d = source.edges[edge].curve.domain();
        let uses = source
            .loops
            .iter()
            .enumerate()
            .flat_map(|(wire, l)| {
                l.coedges
                    .iter()
                    .enumerate()
                    .filter(move |(_, c)| c.edge == edge)
                    .map(move |(coedge, c)| {
                        let pc = c.pcurve.domain();
                        PcurveCut {
                            wire,
                            coedge,
                            parameter: pc[0]
                                + (pc[1] - pc[0])
                                    * if c.reversed { 1. - fraction } else { fraction },
                        }
                    })
            })
            .collect::<Vec<_>>();
        let split = edge_partition::partition(
            &source,
            edge,
            d[0] + (d[1] - d[0]) * fraction,
            &uses,
            100000,
        )
        .unwrap();
        roots.push(split.vertex);
        chains[side] = split
            .child_edges
            .map(|edge| BoundaryUse {
                face: supports[side].face,
                edge,
            })
            .to_vec();
        // Input ordering cannot substitute for owned chart/vertex continuity.
        chains[side].reverse();
        source = split.model;
    }
    let before = source.clone();
    let out = region_replacement::replace_g2_sides(
        &source,
        &[selected],
        chains.clone(),
        options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    assert!(out.corner.admitted && out.corner.interior.certified);
    assert_eq!(
        out.replacement.model.loops[out.replacement.model.faces[out.replacement.face].outer]
            .coedges
            .len(),
        8
    );
    assert_eq!(out.replacement.model.edges.len(), source.edges.len());
    for (i, v) in out.replacement.source_edges.iter().enumerate() {
        let edge = v.unwrap();
        assert_eq!(
            out.replacement.model.edges[edge].curve,
            source.edges[i].curve
        );
        assert_eq!(out.replacement.model.1.edges[edge], source.1.edges[i]);
    }
    for vertex in roots {
        let mapped = out.replacement.source_vertices[vertex].unwrap();
        assert_eq!(
            out.replacement.model.vertices[mapped],
            source.vertices[vertex]
        );
        assert_eq!(
            out.replacement.model.1.vertices[mapped],
            source.1.vertices[vertex]
        );
    }
    assert_eq!(
        out.replacement.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert!(
        out.cells <= 100000 && out.exact_work <= 1000000 && out.corner.coefficient_work <= 2000000
    );
    let mut duplicate = chains.clone();
    duplicate[0].push(duplicate[0][0]);
    assert!(
        region_replacement::replace_g2_sides(
            &source,
            &[selected],
            duplicate,
            options,
            1e-8,
            [1e-7, 1e-5, 1e-4],
            100000,
            1000000
        )
        .is_err()
    );
    let mut gap = chains;
    gap[0].pop();
    assert!(
        region_replacement::replace_g2_sides(
            &source,
            &[selected],
            gap,
            options,
            1e-8,
            [1e-7, 1e-5, 1e-4],
            100000,
            1000000
        )
        .is_err()
    );
    assert_eq!(source, before);
}
#[test]
fn g2_region_is_built_from_original_retained_supports_and_replaces_the_old_chart() {
    use nurbs_core::{surface::Axis, surface_edit};
    let (open, supports, options) = corner_fixture::fixture_extended();
    let closed = brep_core::corner_close::close(
        &open,
        0,
        supports,
        options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    let selected = closed.face;
    let expected = closed.model.faces[selected]
        .surface
        .evaluate(0.5, 0.5)
        .unwrap()
        .point;
    let mut source = closed.model;
    let elevated = surface_edit::elevate(&source.faces[selected].surface, Axis::U, 6).unwrap();
    let mut elevated = surface_edit::elevate(&elevated, Axis::V, 6).unwrap();
    // A central degree-six control changes the interior while preserving
    // the first three boundary layers. The old chart is not used as a support.
    elevated.control_points[3][3][2] += 0.1;
    source.faces[selected].surface = elevated;
    source.rebuild_topology_ids();
    source.validate().unwrap();
    let before = source.clone();
    assert!(
        (source.faces[selected]
            .surface
            .evaluate(0.5, 0.5)
            .unwrap()
            .point[2]
            - expected[2])
            .abs()
            > 0.001
    );
    let out = region_replacement::replace_g2(
        &source,
        &[selected],
        supports,
        options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    assert!(out.corner.admitted && out.corner.interior.certified && out.corner.within_tolerance);
    assert_eq!(out.corner.seams.len(), 4);
    assert!(
        out.cells <= 100000 && out.exact_work <= 1000000 && out.corner.coefficient_work <= 2000000
    );
    let result = &out.replacement;
    assert_eq!(
        result.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert_eq!(result.model.bodies.len(), source.bodies.len());
    assert_eq!(result.model.faces.len(), source.faces.len());
    let actual = result.model.faces[result.face]
        .surface
        .evaluate(0.5, 0.5)
        .unwrap()
        .point;
    for k in 0..3 {
        assert!((actual[k] - expected[k]).abs() < 1e-12);
    }
    for (i, mapped) in result.source_faces.iter().enumerate() {
        if let Some(f) = mapped {
            assert_eq!(result.model.faces[*f].surface, source.faces[i].surface);
            assert_eq!(result.model.1.faces[*f], source.1.faces[i]);
        }
    }
    let mut bad = supports;
    bad[0].face = selected;
    assert!(
        region_replacement::replace_g2(
            &source,
            &[selected],
            bad,
            options,
            1e-8,
            [1e-7, 1e-5, 1e-4],
            100000,
            1000000
        )
        .is_err()
    );
    assert!(
        region_replacement::replace_g2(
            &source,
            &[selected],
            supports,
            options,
            1e-8,
            [1e-7, 1e-5, 1e-4],
            1,
            0
        )
        .is_err()
    );
    assert_eq!(source, before);
}
#[test]
fn connected_sectors_are_replaced_atomically_with_owned_rim_and_merge_lineage() {
    let source = brep_core::cuboid([0.; 3], [8.; 3]).unwrap();
    let top = source
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 8.)
        })
        .unwrap();
    let rim = source.loops[source.faces[top].outer].coedges.clone();
    let endpoints =
        [0, 2].map(|i| source.edges[rim[i].edge].vertices[usize::from(rim[i].reversed)]);
    let cut = Curve::from_polyline(
        endpoints
            .map(|v| source.vertices[v].point.to_vec())
            .to_vec(),
    )
    .unwrap();
    let partition =
        face_cut::partition_projected(&source, top, endpoints, &[cut], 1e-8, 100000).unwrap();
    let model = &partition.model;
    let before = model.clone();
    let original = &source.faces[top].surface;
    let surface = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let mut p = original
                            .evaluate(i as f64 * 0.5, j as f64 * 0.5)
                            .unwrap()
                            .point
                            .to_vec();
                        if i == 1 && j == 1 {
                            p[2] -= 0.4;
                        }
                        p
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let reversed = source.shells[0]
        .faces
        .iter()
        .find(|u| u.face == top)
        .unwrap()
        .reversed;
    let out = region_replacement::replace(
        model,
        &partition.faces,
        &surface,
        &[rim.clone()],
        reversed,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    assert_eq!(out.model.faces.len(), source.faces.len());
    assert_eq!(out.model.edges.len(), source.edges.len());
    assert_eq!(out.source_edges[partition.edges[0]], None);
    assert!(out.trims.all_valid);
    assert_eq!(
        out.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert!(out.model.shells[0].closed);
    assert!(out.cells <= 100000 && out.exact_work <= 1000000);
    let restored: brep_core::Model =
        value_codec::from_str(&value_codec::to_string(&out.model).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, out.model);
    assert!(
        (out.model.faces[out.face]
            .surface
            .evaluate(0.5, 0.5)
            .unwrap()
            .point[2]
            - 7.9)
            .abs()
            < 1e-12
    );
    for (i, mapped) in out.source_edges.iter().enumerate() {
        if let Some(k) = mapped {
            assert_eq!(out.model.edges[*k].curve, model.edges[i].curve);
            assert_eq!(out.model.1.edges[*k], model.1.edges[i]);
        }
    }
    for (i, mapped) in out.source_faces.iter().enumerate() {
        if let Some(k) = mapped {
            assert_eq!(out.model.faces[*k].surface, model.faces[i].surface);
            assert_eq!(out.model.1.faces[*k], model.1.faces[i]);
        }
    }
    assert!(out.model.1.lineage.iter().any(|r| r.operation == "merge"
        && r.entity_kind == "face"
        && r.parents == partition.faces.map(|f| model.1.faces[f])
        && r.children == [out.model.1.faces[out.face]]));
    let mut missing = rim.clone();
    missing.pop();
    assert!(
        region_replacement::replace(
            model,
            &partition.faces,
            &surface,
            &[missing],
            reversed,
            1e-8,
            100000,
            1000000
        )
        .is_err()
    );
    let bottom = model
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 0.)
        })
        .unwrap();
    assert!(
        region_replacement::replace(
            model,
            &[partition.faces[0], bottom],
            &surface,
            &[rim.clone()],
            reversed,
            1e-8,
            100000,
            1000000
        )
        .is_err()
    );
    assert!(
        region_replacement::replace(
            model,
            &[partition.faces[0]; 2],
            &surface,
            &[rim.clone()],
            reversed,
            1e-8,
            100000,
            1000000
        )
        .is_err()
    );
    assert!(
        region_replacement::replace(
            model,
            &partition.faces,
            &surface,
            &[rim.clone()],
            !reversed,
            1e-8,
            100000,
            1000000
        )
        .is_err()
    );
    let mut bad = surface.clone();
    bad.control_points[0][1][2] -= 0.1;
    assert!(
        region_replacement::replace(
            model,
            &partition.faces,
            &bad,
            &[rim.clone()],
            reversed,
            1e-8,
            100000,
            1000000
        )
        .is_err()
    );
    assert!(
        region_replacement::replace(
            model,
            &partition.faces,
            &surface,
            &[rim],
            reversed,
            1e-8,
            3,
            0
        )
        .is_err()
    );
    assert_eq!(*model, before);
}
