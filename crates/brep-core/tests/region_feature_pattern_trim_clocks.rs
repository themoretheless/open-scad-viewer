use brep_core::{
    Model,
    feature_ops::{
        PatternMode,
        region_pattern::{self, FeatureRole, Limits, Placement},
    },
};
use brep_topology::CoedgeTrim;
use nurbs_core::{curve::Curve, foundation::guards::Budget};
fn budget() -> Budget {
    Budget::new(10000000, 16, 60000).unwrap()
}
fn volume(m: &Model) -> f64 {
    brep_core::analysis::mass_properties(m, 1e-6, 2000000)
        .unwrap()
        .signed_volume_mm3
}
fn boss() -> Model {
    brep_core::operations::boolean(
        &brep_core::cuboid([0.; 3], [12., 8., 2.]).unwrap(),
        &brep_core::cuboid([1., 1., 2.], [3., 3., 3.]).unwrap(),
        "union",
    )
    .unwrap()
}
fn selected(m: &Model) -> Vec<usize> {
    m.faces
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] >= 2.)
                && f.surface.control_points.iter().flatten().any(|p| p[2] > 2.)
        })
        .map(|(i, _)| i)
        .collect()
}
fn clocks(m: &mut Model, selected_only: bool) {
    for edge in &mut m.edges {
        let d = edge.curve.domain();
        for k in &mut edge.curve.knots {
            *k = -7. + 20. * (*k - d[0]) / (d[1] - d[0]);
        }
    }
    for wire in &mut m.loops {
        for c in &mut wire.coedges {
            let d = c.pcurve.domain();
            for k in &mut c.pcurve.knots {
                *k = 2. + 3. * (*k - d[0]) / (d[1] - d[0]);
            }
        }
    }
    let faces = if selected_only {
        selected(m)
    } else {
        (0..m.faces.len()).collect()
    };
    for f in faces {
        for w in std::iter::once(m.faces[f].outer).chain(m.faces[f].holes.clone()) {
            for (i, c) in m.loops[w].coedges.iter_mut().enumerate() {
                let (mut curve, mut pcurve) = ([-2., 8.], [2.75, 4.25]);
                if i % 2 == 1 {
                    curve.reverse();
                    pcurve.reverse();
                }
                *c = c
                    .clone()
                    .with_trim(CoedgeTrim {
                        curve_parameter: curve,
                        pcurve_parameter: pcurve,
                        periodic_lift: [[0; 2]; 2],
                    })
                    .unwrap();
            }
        }
    }
    m.validate().unwrap();
}
#[test]
fn partial_descending_and_nonunit_authored_rim_clocks_transfer_without_mutating_originals() {
    let mut s = boss();
    clocks(&mut s, false);
    let before = s.clone();
    let faces = selected(&s);
    assert!(
        brep_core::boundary_agreement::verify(&s, 100000)
            .unwrap()
            .complete
    );
    let f = region_pattern::extract(&s, &faces, FeatureRole::Additive, Limits::default()).unwrap();
    assert!((volume(f.tool()) - 4.).abs() < 1e-6);
    for face in faces {
        let id = s.1.faces[face];
        let new = f.tool().1.faces.iter().position(|v| *v == id).unwrap();
        let old = &s.loops[s.faces[face].outer];
        let copied = &f.tool().loops[f.tool().faces[new].outer];
        assert_eq!(
            old.coedges.iter().map(|c| c.trim()).collect::<Vec<_>>(),
            copied.coedges.iter().map(|c| c.trim()).collect::<Vec<_>>()
        );
    }
    for face in f.cap_faces() {
        for c in &f.tool().loops[f.tool().faces[*face].outer].coedges {
            assert!(c.trim().is_some());
        }
    }
    assert!(f.reverify(Limits::default()).unwrap().boundary_complete);
    assert_eq!(s, before);
}
#[test]
fn selected_only_trim_clocks_follow_original_edge_into_derived_cap_and_affine_instances() {
    let mut s = boss();
    clocks(&mut s, true);
    let f = region_pattern::extract(&s, &selected(&s), FeatureRole::Additive, Limits::default())
        .unwrap();
    let empty = Model::empty(s.tolerance_mm).unwrap();
    let p = region_pattern::pattern(
        &empty,
        &f,
        &[Placement {
            key: "scaled".into(),
            matrix: [
                [2., 0., 0., 0.],
                [0., 3., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        }],
        PatternMode::Join,
        "trim-clock",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(p.model()) - 24.).abs() < 1e-6);
    assert!(p.reverify(&budget()).unwrap().boundary_complete);
    let joined = region_pattern::pattern(
        &s,
        &f,
        &[Placement {
            key: "next".into(),
            matrix: [
                [1., 0., 0., 4.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        }],
        PatternMode::Join,
        "trim-clock-join",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(joined.model()) - 200.).abs() < 1e-6);
    assert!(joined.reverify(&budget()).unwrap().boundary_complete);
}
// Author exact coordinate charts to exercise the affine cap reexpression
// family, while keeping every original 3D edge and effective shell direction.
fn coordinate_caps(m: &mut Model) {
    let edges = m.edges.clone();
    let points = m.vertices.iter().map(|v| v.point).collect::<Vec<_>>();
    for f in 0..m.faces.len() {
        let height = m.faces[f].surface.control_points[0][0][2];
        if !m.faces[f]
            .surface
            .control_points
            .iter()
            .flatten()
            .all(|p| p[2] == height)
        {
            continue;
        }
        let wires = std::iter::once(m.faces[f].outer)
            .chain(m.faces[f].holes.clone())
            .collect::<Vec<_>>();
        let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
        for &w in &wires {
            for c in &m.loops[w].coedges {
                for p in &edges[c.edge].curve.control_points {
                    for k in 0..2 {
                        bounds[k][0] = bounds[k][0].min(p[k]);
                        bounds[k][1] = bounds[k][1].max(p[k]);
                    }
                }
            }
        }
        m.faces[f].surface = nurbs_core::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![bounds[0][0], bounds[0][0], bounds[0][1], bounds[0][1]],
            knots_v: vec![bounds[1][0], bounds[1][0], bounds[1][1], bounds[1][1]],
            control_points: (0..2)
                .map(|i| {
                    (0..2)
                        .map(|j| vec![bounds[0][i], bounds[1][j], height])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        for &w in &wires {
            for c in &mut m.loops[w].coedges {
                let mut uv = if c.reversed {
                    edges[c.edge].curve.reverse().unwrap()
                } else {
                    edges[c.edge].curve.clone()
                };
                for p in &mut uv.control_points {
                    p.truncate(2);
                }
                c.pcurve = uv;
            }
        }
        let w = m.faces[f].outer;
        let polygon = m.loops[w]
            .coedges
            .iter()
            .map(|c| points[edges[c.edge].vertices[usize::from(c.reversed)]])
            .collect::<Vec<_>>();
        let area = (0..polygon.len())
            .map(|i| {
                polygon[i][0] * polygon[(i + 1) % polygon.len()][1]
                    - polygon[(i + 1) % polygon.len()][0] * polygon[i][1]
            })
            .sum::<f64>();
        if area < 0. {
            for w in wires {
                m.loops[w].coedges.reverse();
                for c in &mut m.loops[w].coedges {
                    c.reversed = !c.reversed;
                    c.pcurve = c.pcurve.reverse().unwrap();
                }
            }
            for shell in &mut m.shells {
                for u in &mut shell.faces {
                    if u.face == f {
                        u.reversed = !u.reversed;
                    }
                }
            }
        }
    }
    m.validate().unwrap();
}
fn circle() -> Vec<Curve> {
    let p = [
        [[3., 3.], [3., 4.], [2., 4.]],
        [[2., 4.], [1., 4.], [1., 3.]],
        [[1., 3.], [1., 2.], [2., 2.]],
        [[2., 2.], [3., 2.], [3., 3.]],
    ];
    p.into_iter()
        .map(|p| Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: p.into_iter().map(|p| p.to_vec()).collect(),
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        })
        .collect()
}
#[test]
fn rational_original_support_pcurves_and_explicit_partial_clocks_remain_owned() {
    let tool = brep_core::prism::extrude(&[circle()], 2., 3.).unwrap();
    let mut s = brep_core::operations::boolean(
        &brep_core::cuboid([0.; 3], [12., 8., 2.]).unwrap(),
        &tool,
        "union",
    )
    .unwrap();
    coordinate_caps(&mut s);
    clocks(&mut s, false);
    let f = region_pattern::extract(&s, &selected(&s), FeatureRole::Additive, Limits::default())
        .unwrap();
    assert!((volume(f.tool()) - std::f64::consts::PI).abs() < 1e-5);
    assert!(f.tool().edges.iter().any(|e| e.curve.degree == 2));
    assert!(f.reverify(Limits::default()).unwrap().trim_regions_valid);
    let empty = Model::empty(s.tolerance_mm).unwrap();
    let p = region_pattern::pattern(
        &empty,
        &f,
        &[Placement {
            key: "rational-scaled".into(),
            matrix: [
                [2., 0., 0., 0.],
                [0., 3., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        }],
        PatternMode::Join,
        "rational-trim-affine",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(p.model()) - 6. * std::f64::consts::PI).abs() < 1e-5);
    let trims = |m: &Model| {
        m.loops
            .iter()
            .flat_map(|w| w.coedges.iter().map(|c| c.trim()))
            .collect::<Vec<_>>()
    };
    assert_eq!(trims(f.tool()), trims(p.instances()[0].model()));
    assert!(p.reverify(&budget()).unwrap().boundary_complete);
}
#[test]
fn partial_metadata_cannot_hide_geometry_mismatch_outside_its_authored_interval() {
    let mut s = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let e = 0;
    let old = s.edges[e].curve.clone();
    let a = old.control_points[0].clone();
    let b = old.control_points[1].clone();
    let axis = (0..3)
        .find(|k| a[*k] != b[*k])
        .map(|k| (k + 1) % 3)
        .unwrap();
    let t = [0., 0.1, 0.105, 0.11, 1.];
    s.edges[e].curve = Curve {
        degree: 1,
        knots: vec![0., 0., 0.1, 0.105, 0.11, 1., 1.],
        control_points: t
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let mut p = (0..3).map(|k| a[k] + (b[k] - a[k]) * t).collect::<Vec<_>>();
                if i == 2 {
                    p[axis] += 0.02;
                }
                p
            })
            .collect(),
        weights: vec![1.; 5],
        periodic: false,
    };
    for wire in &mut s.loops {
        for c in &mut wire.coedges {
            if c.edge == e {
                *c = c
                    .clone()
                    .with_trim(CoedgeTrim {
                        curve_parameter: [0.25, 0.5],
                        pcurve_parameter: [0.25, 0.5],
                        periodic_lift: [[0; 2]; 2],
                    })
                    .unwrap();
            }
        }
    }
    s.validate().unwrap();
    let before = s.clone();
    assert!(
        brep_core::boundary_agreement::verify(&s, 100000)
            .unwrap()
            .complete
    );
    let bottom = s
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
    let faces = (0..s.faces.len())
        .filter(|i| *i != bottom)
        .collect::<Vec<_>>();
    assert!(region_pattern::extract(&s, &faces, FeatureRole::Additive, Limits::default()).is_err());
    assert_eq!(s, before);
}
#[test]
fn invalid_authored_domains_lifts_and_exhausted_transfer_work_refuse_atomically() {
    let mut s = boss();
    clocks(&mut s, false);
    let faces = selected(&s);
    let original = s.clone();
    for field in 0..3 {
        let mut invalid = s.clone();
        let w = invalid.faces[faces[0]].outer;
        let c = invalid.loops[w].coedges[0].clone();
        let mut trim = c.trim().unwrap();
        match field {
            0 => trim.curve_parameter[0] = -8.,
            1 => trim.periodic_lift[0][0] = 1,
            _ => trim.pcurve_parameter.reverse(),
        }
        invalid.loops[w].coedges[0] = c.with_trim(trim).unwrap();
        assert!(
            region_pattern::extract(&invalid, &faces, FeatureRole::Additive, Limits::default())
                .is_err()
        );
    }
    assert!(
        region_pattern::extract(
            &s,
            &faces,
            FeatureRole::Additive,
            Limits {
                max_cells: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert_eq!(s, original);
}
#[test]
fn nondyadic_authored_clock_values_are_retained_and_converted_clocks_are_reproved() {
    let mut s = boss();
    clocks(&mut s, false);
    for wire in &mut s.loops {
        for c in &mut wire.coedges {
            let mut curve = [-2.1, 6.7];
            if c.trim().unwrap().curve_parameter[0] > c.trim().unwrap().curve_parameter[1] {
                curve.reverse();
            }
            let pcurve = curve.map(|v| 2. + 3. * (v + 7.) / 20.);
            *c = c
                .clone()
                .with_trim(CoedgeTrim {
                    curve_parameter: curve,
                    pcurve_parameter: pcurve,
                    periodic_lift: [[0; 2]; 2],
                })
                .unwrap();
        }
    }
    let original = s.clone();
    let f = region_pattern::extract(&s, &selected(&s), FeatureRole::Additive, Limits::default())
        .unwrap();
    assert_eq!(f.source(), &original);
    assert!((volume(f.tool()) - 4.).abs() < 1e-6);
    assert!(f.reverify(Limits::default()).unwrap().boundary_complete);
    let empty = Model::empty(s.tolerance_mm).unwrap();
    let p = region_pattern::pattern(
        &empty,
        &f,
        &[Placement {
            key: "reflected".into(),
            matrix: [
                [-2., 0., 0., 0.],
                [0., 3., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        }],
        PatternMode::Join,
        "nondyadic-clock",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(p.model()) - 24.).abs() < 1e-6);
    assert!(p.reverify(&budget()).unwrap().boundary_complete);
}
