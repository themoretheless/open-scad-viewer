use brep_core::{
    Model,
    feature_ops::{
        PatternMode,
        region_pattern::{self, FeatureRole, Limits, Placement},
    },
};
use nurbs_core::{
    foundation::guards::Budget,
    surface::{Axis, Surface},
};
fn budget() -> Budget {
    Budget::new(10000000, 16, 60000).unwrap()
}
fn volume(m: &Model) -> f64 {
    brep_core::analysis::mass_properties(m, 1e-6, 2000000)
        .unwrap()
        .signed_volume_mm3
}
fn box3(lo: [f64; 3], hi: [f64; 3]) -> Model {
    brep_core::cuboid(lo, hi).unwrap()
}
fn union(a: &Model, b: &Model) -> Model {
    brep_core::operations::boolean(a, b, "union").unwrap()
}
fn stock() -> Model {
    box3([0.; 3], [12., 12., 2.])
}
fn vertices(m: &Model, f: usize) -> Vec<[f64; 3]> {
    std::iter::once(m.faces[f].outer)
        .chain(m.faces[f].holes.iter().copied())
        .flat_map(|w| m.loops[w].coedges.iter())
        .flat_map(|c| m.edges[c.edge].vertices)
        .map(|v| m.vertices[v].point)
        .collect()
}
fn shift(key: &str, y: f64) -> Placement {
    Placement {
        key: key.into(),
        matrix: [
            [1., 0., 0., 0.],
            [0., 1., 0., y],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    }
}
fn stepped() -> (Model, Vec<usize>) {
    let a = union(&stock(), &box3([4., 0., 2.], [12., 12., 3.]));
    let b = union(&a, &box3([1., 1., 2.], [2., 3., 5.]));
    let c = union(&b, &box3([5., 1., 3.], [6., 3., 5.]));
    let s = union(&c, &box3([1., 1., 5.], [6., 3., 6.]));
    let faces = (0..s.faces.len())
        .filter(|i| {
            let p = vertices(&s, *i);
            p.iter()
                .all(|p| p[0] >= 1. && p[0] <= 6. && p[1] >= 1. && p[1] <= 3. && p[2] >= 2.)
                && !p.iter().all(|p| p[2] == 2.)
                && !p.iter().all(|p| p[2] == 3.)
        })
        .collect();
    (s, faces)
}
#[test]
fn different_parallel_support_levels_close_independently_and_pattern() {
    let (s, faces) = stepped();
    let before = s.clone();
    let f = region_pattern::extract(&s, &faces, FeatureRole::Additive, Limits::default()).unwrap();
    assert_eq!(f.cap_faces().len(), 2);
    assert!(f.cap_faces().iter().any(|i| {
        f.tool().faces[*i]
            .surface
            .control_points
            .iter()
            .flatten()
            .all(|p| p[2] == 2.)
    }));
    assert!(f.cap_faces().iter().any(|i| {
        f.tool().faces[*i]
            .surface
            .control_points
            .iter()
            .flatten()
            .all(|p| p[2] == 3.)
    }));
    assert!((volume(f.tool()) - 20.).abs() < 1e-6);
    assert!((volume(&s) - 404.).abs() < 1e-6);
    assert!(f.reverify(Limits::default()).unwrap().boundary_complete);
    let p = region_pattern::pattern(
        &s,
        &f,
        &[shift("next", 5.)],
        PatternMode::Join,
        "stepped-chart",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(p.model()) - 424.).abs() < 1e-6);
    assert!(p.reverify(&budget()).unwrap().trim_regions_valid);
    assert_eq!(s, before);
}
#[test]
fn perpendicular_floor_and_wall_supports_keep_both_original_rims() {
    let a = union(&stock(), &box3([8., 0., 2.], [10., 12., 8.]));
    let b = union(&a, &box3([1., 1., 2.], [2., 3., 5.]));
    let s = union(&b, &box3([1., 1., 5.], [8., 3., 6.]));
    let faces = (0..s.faces.len())
        .filter(|i| {
            let p = vertices(&s, *i);
            p.iter()
                .all(|p| p[0] >= 1. && p[0] <= 8. && p[1] >= 1. && p[1] <= 3. && p[2] >= 2.)
                && !p.iter().all(|p| p[2] == 2.)
                && !p.iter().all(|p| p[0] == 8.)
        })
        .collect::<Vec<_>>();
    let f = region_pattern::extract(&s, &faces, FeatureRole::Additive, Limits::default()).unwrap();
    assert_eq!(f.cap_faces().len(), 2);
    assert!((volume(f.tool()) - 20.).abs() < 1e-6);
    assert!((volume(&s) - 452.).abs() < 1e-6);
    assert!(f.cap_faces().iter().any(|i| {
        f.tool().faces[*i]
            .surface
            .control_points
            .iter()
            .flatten()
            .all(|p| p[0] == 8.)
    }));
    let p = region_pattern::pattern(
        &s,
        &f,
        &[shift("next", 5.)],
        PatternMode::Join,
        "perpendicular-chart",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(p.model()) - 472.).abs() < 1e-6);
}
fn bent_ends() -> (Model, Vec<usize>, Vec<Surface>) {
    let mut s = box3([0.; 3], [2., 2., 3.]);
    let ends = (0..s.faces.len())
        .filter(|i| {
            s.faces[*i]
                .surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 0.)
                || s.faces[*i]
                    .surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[2] == 3.)
        })
        .collect::<Vec<_>>();
    let mut surfaces = Vec::new();
    for &i in &ends {
        let bottom = s.faces[i].surface.control_points[0][0][2] == 0.;
        let t = nurbs_core::surface_edit::elevate(&s.faces[i].surface, Axis::U, 3).unwrap();
        let mut t = nurbs_core::surface_edit::elevate(&t, Axis::V, 3).unwrap();
        t.control_points[1][1][2] += if bottom { 0.125 } else { 0.25 };
        s.faces[i].surface = t.clone();
        surfaces.push(t);
    }
    s.validate().unwrap();
    let faces = (0..s.faces.len()).filter(|i| !ends.contains(i)).collect();
    (s, faces, surfaces)
}
#[test]
fn distinct_original_nonplanar_end_charts_are_retained_without_fitting() {
    let (s, faces, surfaces) = bent_ends();
    let f = region_pattern::extract(&s, &faces, FeatureRole::Additive, Limits::default()).unwrap();
    assert_eq!(f.cap_faces().len(), 2);
    for surface in surfaces {
        assert!(
            f.cap_faces()
                .iter()
                .any(|i| f.tool().faces[*i].surface == surface)
        );
    }
    // Each bicubic interior Bernstein basis integrates to area/16 = 1/4.
    let expected = 12. + (0.25 - 0.125) / 4.;
    assert!((volume(f.tool()) - expected).abs() < 1e-6);
    assert!(f.reverify(Limits::default()).unwrap().charts_injective);
    let empty = Model::empty(s.tolerance_mm).unwrap();
    let mut scaled = shift("scaled", 0.);
    scaled.matrix[0][0] = 2.;
    scaled.matrix[1][1] = 3.;
    let p = region_pattern::pattern(
        &empty,
        &f,
        &[scaled],
        PatternMode::Join,
        "nonplanar-ends",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(p.model()) - 6. * expected).abs() < 1e-6);
}
fn flat_bridge() -> (Model, Vec<usize>) {
    let a = union(&stock(), &box3([1., 1., 2.], [2., 3., 4.]));
    let b = union(&a, &box3([4., 1., 2.], [5., 3., 4.]));
    let s = union(&b, &box3([1., 1., 4.], [5., 3., 5.]));
    let faces = (0..s.faces.len())
        .filter(|i| {
            let p = vertices(&s, *i);
            p.iter().all(|p| p[2] >= 2.) && p.iter().any(|p| p[2] > 2.)
        })
        .collect();
    (s, faces)
}
fn broad_graph(a: f64, b: f64) -> Surface {
    let x = [a * b, a * b - (a + b) * 8., (16. - a) * (16. - b)];
    let y = [3., -29., 195.];
    let net = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| vec![8. * i as f64, 8. * j as f64, 2. + x[i] * y[j] / 1024.])
                .collect()
        })
        .collect();
    nurbs_core::patches::bezier(net, None).unwrap()
}
#[test]
fn overlapping_complete_carriers_require_and_pass_actual_trimmed_cap_separation() {
    let (original, faces) = flat_bridge();
    let seed = region_pattern::extract(&original, &faces, FeatureRole::Additive, Limits::default())
        .unwrap();
    let ends = seed.cap_faces().to_vec();
    let mut s = seed.tool().clone();
    let edges = s.edges.iter().map(|e| e.vertices).collect::<Vec<_>>();
    let points = s.vertices.iter().map(|v| v.point).collect::<Vec<_>>();
    for &face in &ends {
        let p = vertices(&s, face);
        let a = p.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
        s.faces[face].surface = broad_graph(a, a + 1.);
        for wire in std::iter::once(s.faces[face].outer).chain(s.faces[face].holes.clone()) {
            for c in &mut s.loops[wire].coedges {
                let v = edges[c.edge];
                let p = points[v[usize::from(c.reversed)]];
                let q = points[v[usize::from(!c.reversed)]];
                c.pcurve = nurbs_core::curve::Curve::from_polyline(vec![
                    vec![p[0] / 16., p[1] / 16.],
                    vec![q[0] / 16., q[1] / 16.],
                ])
                .unwrap();
            }
        }
    }
    // These replacement charts use +X/+Y coordinates. Canonicalize the
    // stored UV loops and flip their shell uses together, preserving each
    // original effective world-edge orientation.
    for &face in &ends {
        let wire = s.faces[face].outer;
        let polygon = s.loops[wire]
            .coedges
            .iter()
            .map(|c| points[edges[c.edge][usize::from(c.reversed)]])
            .collect::<Vec<_>>();
        let area = (0..polygon.len())
            .map(|i| {
                polygon[i][0] * polygon[(i + 1) % polygon.len()][1]
                    - polygon[(i + 1) % polygon.len()][0] * polygon[i][1]
            })
            .sum::<f64>();
        if area < 0. {
            s.loops[wire].coedges.reverse();
            for c in &mut s.loops[wire].coedges {
                c.reversed = !c.reversed;
                c.pcurve = c.pcurve.reverse().unwrap();
            }
            for shell in &mut s.shells {
                for u in &mut shell.faces {
                    if u.face == face {
                        u.reversed = !u.reversed;
                    }
                }
            }
        }
    }
    s.validate().unwrap();
    let faces = (0..s.faces.len())
        .filter(|i| !ends.contains(i))
        .collect::<Vec<_>>();
    let f = region_pattern::extract(&s, &faces, FeatureRole::Additive, Limits::default()).unwrap();
    assert_eq!(f.cap_faces().len(), 2);
    assert!((volume(f.tool()) - (16. - 1. / 2304.)).abs() < 1e-6);
    assert!(f.reverify(Limits::default()).unwrap().trim_regions_valid);
}
#[test]
fn distinct_chart_work_limits_and_source_reordering_preserve_atomicity_and_ids() {
    let (s, faces) = stepped();
    let before = s.clone();
    let f = region_pattern::extract(&s, &faces, FeatureRole::Additive, Limits::default()).unwrap();
    for limits in [
        Limits {
            max_cells: 1,
            ..Limits::default()
        },
        Limits {
            max_pairs: 1,
            ..Limits::default()
        },
        Limits {
            max_domain_cells: 1,
            ..Limits::default()
        },
    ] {
        assert!(f.reverify(limits).is_err());
    }
    let mut reordered = s.clone();
    let n = reordered.faces.len();
    reordered.faces.reverse();
    reordered.1.faces.reverse();
    for shell in &mut reordered.shells {
        for u in &mut shell.faces {
            u.face = n - 1 - u.face;
        }
    }
    let faces = faces.into_iter().map(|i| n - 1 - i).collect::<Vec<_>>();
    let g = region_pattern::extract(&reordered, &faces, FeatureRole::Additive, Limits::default())
        .unwrap();
    let ids = |r: &region_pattern::FeatureRegion| {
        r.cap_faces()
            .iter()
            .map(|i| r.tool().1.faces[*i])
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&f), ids(&g));
    assert_eq!(s, before);
}
#[test]
fn individually_regular_but_intersecting_original_caps_are_not_published() {
    let (mut s, faces, _) = bent_ends();
    for face in &mut s.faces {
        if face.surface.degree_u == 3 && face.surface.degree_v == 3 {
            face.surface.control_points[1][1][2] = if face.surface.control_points[0][0][2] == 0. {
                16.
            } else {
                -13.
            };
        }
    }
    // Only interior poles changed: all original boundary lifts and topology
    // remain valid, while the two graph images cross inside their UV domains.
    for face in &s.faces {
        if face.surface.degree_u == 3 && face.surface.degree_v == 3 {
            assert!(
                nurbs_core::surface_regularity::inspect(&face.surface, 100000)
                    .unwrap()
                    .spanwise_regular
            );
            assert!(
                nurbs_core::surface_injectivity::certify(&face.surface, 100000)
                    .unwrap()
                    .proven
            );
        }
    }
    s.validate().unwrap();
    let before = s.clone();
    let err = region_pattern::extract(&s, &faces, FeatureRole::Additive, Limits::default())
        .err()
        .unwrap();
    assert!(err.message.contains("trimmed-cap separation"), "{err:?}");
    assert_eq!(s, before);
}
