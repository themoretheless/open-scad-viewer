use brep_core::{
    Model,
    feature_ops::{
        PatternMode,
        region_pattern::{self, FeatureRole, Limits, Placement},
    },
};
use nurbs_core::{curve::Curve, foundation::guards::Budget};
fn budget() -> Budget {
    Budget::new(10000000, 16, 60000).unwrap()
}
fn stock() -> Model {
    brep_core::cuboid([0.; 3], [12., 8., 2.]).unwrap()
}
fn volume(m: &Model) -> f64 {
    brep_core::analysis::mass_properties(m, 1e-6, 2000000)
        .unwrap()
        .signed_volume_mm3
}
fn rectangle(lo: [f64; 2], hi: [f64; 2], reverse: bool) -> Vec<Curve> {
    let mut p = vec![lo, [hi[0], lo[1]], hi, [lo[0], hi[1]]];
    if reverse {
        p.reverse();
    }
    (0..4)
        .map(|i| Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()]).unwrap())
        .collect()
}
fn profile() -> Vec<Vec<Curve>> {
    vec![
        rectangle([1., 1.], [4., 5.], false),
        rectangle([1.5, 2.], [2., 3.], true),
        rectangle([2.5, 3.], [3., 4.], true),
    ]
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
                .all(|p| p[2] >= 2. - 1e-9)
                && f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .any(|p| p[2] > 2. + 1e-9)
        })
        .map(|(i, _)| i)
        .collect()
}
fn shift(key: &str, x: f64, z: f64) -> Placement {
    Placement {
        key: key.into(),
        matrix: [
            [1., 0., 0., x],
            [0., 1., 0., 0.],
            [0., 0., 1., z],
            [0., 0., 0., 1.],
        ],
    }
}
fn source(p: &[Vec<Curve>]) -> Model {
    let tool = brep_core::prism::extrude(p, 2., 3.).unwrap();
    brep_core::operations::boolean(&stock(), &tool, "union").unwrap()
}
#[test]
fn original_feature_with_two_holes_patterns_its_material_without_filling_holes() {
    let s = source(&profile());
    let before = s.clone();
    let f = region_pattern::extract(&s, &selected(&s), FeatureRole::Additive, Limits::default())
        .unwrap();
    assert_eq!(f.tool().faces[f.cap_face()].holes.len(), 2);
    assert!((volume(f.tool()) - 11.).abs() < 1e-6);
    for (i, id) in f.tool().1.edges.iter().enumerate() {
        let old = s.1.edges.iter().position(|p| p == id).unwrap();
        assert_eq!(f.tool().edges[i].curve, s.edges[old].curve);
    }
    let joined = region_pattern::pattern(
        &s,
        &f,
        &[shift("next", 5., 0.)],
        PatternMode::Join,
        "two-holes",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(joined.model()) - 214.).abs() < 1e-5);
    assert!(joined.reverify(&budget()).unwrap().trim_regions_valid);
    let cut = region_pattern::pattern(
        &s,
        &f,
        &[shift("below", 5., -1.)],
        PatternMode::Cut,
        "perforated-cut",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(cut.model()) - 192.).abs() < 1e-5);
    // Cutting directly below the original boss leaves an unsupported upper
    // annulus touching the stock only along edges. That result must refuse.
    assert!(
        region_pattern::pattern(
            &s,
            &f,
            &[shift("unsupported", 0., -1.)],
            PatternMode::Cut,
            "edge-contact",
            Limits::default(),
            &budget()
        )
        .is_err()
    );
    let scarce = Limits {
        max_cells: 1,
        ..Limits::default()
    };
    assert!(f.reverify(scarce).is_err());
    assert_eq!(s, before);
}
fn circle(radius: f64, reverse: bool) -> Vec<Curve> {
    let (x, y, r) = (2., 3., radius);
    let points = [
        [[x + r, y], [x + r, y + r], [x, y + r]],
        [[x, y + r], [x - r, y + r], [x - r, y]],
        [[x - r, y], [x - r, y - r], [x, y - r]],
        [[x, y - r], [x + r, y - r], [x + r, y]],
    ];
    let mut curves = points
        .into_iter()
        .map(|p| Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: p.into_iter().map(|p| p.to_vec()).collect(),
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        })
        .collect::<Vec<_>>();
    if reverse {
        curves.reverse();
        for c in &mut curves {
            *c = c.reverse().unwrap();
        }
    }
    curves
}
#[test]
fn rational_annular_boss_retains_original_curves_and_hole_under_native_join() {
    let s = source(&[circle(1., false), circle(0.5, true)]);
    let f = region_pattern::extract(&s, &selected(&s), FeatureRole::Additive, Limits::default())
        .unwrap();
    assert_eq!(f.tool().faces[f.cap_face()].holes.len(), 1);
    assert!((volume(f.tool()) - 0.75 * std::f64::consts::PI).abs() < 1e-5);
    assert!(f.reverify(Limits::default()).unwrap().boundary_complete);
    let p = region_pattern::pattern(
        &s,
        &f,
        &[shift("annulus", 4., 0.)],
        PatternMode::Join,
        "rational-holes",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(p.model()) - 192. - 1.5 * std::f64::consts::PI).abs() < 1e-4);
    assert!(p.model().edges.iter().any(|e| e.curve.degree == 2));
}
#[test]
fn complete_original_multiloop_support_reuses_surface_and_independent_clocks() {
    let mut s = brep_core::prism::extrude(&profile(), 2., 3.).unwrap();
    let bottom = s
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 2.)
        })
        .unwrap();
    for e in &mut s.edges {
        for k in &mut e.curve.knots {
            *k = -3. + 7. * *k;
        }
    }
    for w in &mut s.loops {
        for c in &mut w.coedges {
            for k in &mut c.pcurve.knots {
                *k = 11. + 2. * *k;
            }
        }
    }
    s.validate().unwrap();
    let faces = (0..s.faces.len())
        .filter(|i| *i != bottom)
        .collect::<Vec<_>>();
    let f = region_pattern::extract(&s, &faces, FeatureRole::Additive, Limits::default()).unwrap();
    assert_eq!(f.support_faces(), &[bottom]);
    assert_eq!(f.support_loop(), None);
    assert_eq!(
        f.tool().faces[f.cap_face()].surface,
        s.faces[bottom].surface
    );
    assert_eq!(f.tool().faces[f.cap_face()].holes.len(), 2);
    assert!((volume(f.tool()) - 11.).abs() < 1e-6);
    assert!(f.reverify(Limits::default()).unwrap().trim_regions_valid);
}
#[test]
fn hole_loop_identity_survives_all_source_topology_storage_reordering() {
    let s = source(&profile());
    let f = region_pattern::extract(&s, &selected(&s), FeatureRole::Additive, Limits::default())
        .unwrap();
    let mut reordered = s.clone();
    let nv = reordered.vertices.len();
    reordered.vertices.reverse();
    reordered.1.vertices.reverse();
    for e in &mut reordered.edges {
        e.vertices = e.vertices.map(|v| nv - 1 - v);
    }
    let ne = reordered.edges.len();
    reordered.edges.reverse();
    reordered.1.edges.reverse();
    for w in &mut reordered.loops {
        for c in &mut w.coedges {
            c.edge = ne - 1 - c.edge;
        }
    }
    let nl = reordered.loops.len();
    reordered.loops.reverse();
    reordered.1.loops.reverse();
    for face in &mut reordered.faces {
        face.outer = nl - 1 - face.outer;
        for w in &mut face.holes {
            *w = nl - 1 - *w;
        }
    }
    let nf = reordered.faces.len();
    reordered.faces.reverse();
    reordered.1.faces.reverse();
    for shell in &mut reordered.shells {
        for u in &mut shell.faces {
            u.face = nf - 1 - u.face;
        }
    }
    reordered.validate().unwrap();
    let g = region_pattern::extract(
        &reordered,
        &selected(&reordered),
        FeatureRole::Additive,
        Limits::default(),
    )
    .unwrap();
    let empty = Model::empty(s.tolerance_mm).unwrap();
    let a = region_pattern::pattern(
        &empty,
        &f,
        &[shift("same", 0., 0.)],
        PatternMode::Join,
        "stable-holes",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    let b = region_pattern::pattern(
        &empty,
        &g,
        &[shift("same", 0., 0.)],
        PatternMode::Join,
        "stable-holes",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert_eq!(a.instances()[0].tool_ids(), b.instances()[0].tool_ids());
}
