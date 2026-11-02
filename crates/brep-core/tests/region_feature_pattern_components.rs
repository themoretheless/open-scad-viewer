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
fn bridge() -> Model {
    let a = union(&stock(), &box3([1., 1., 2.], [2., 3., 4.]));
    let b = union(&a, &box3([4., 1., 2.], [5., 3., 4.]));
    union(&b, &box3([1., 1., 4.], [5., 3., 5.]))
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
fn shift(key: &str, x: f64, y: f64, z: f64) -> Placement {
    Placement {
        key: key.into(),
        matrix: [
            [1., 0., 0., x],
            [0., 1., 0., y],
            [0., 0., 1., z],
            [0., 0., 0., 1.],
        ],
    }
}
#[test]
fn equal_area_disjoint_bridge_feet_close_separately_and_pattern_with_original_geometry() {
    let s = bridge();
    let original = s.clone();
    let faces = selected(&s);
    let f = region_pattern::extract(&s, &faces, FeatureRole::Additive, Limits::default()).unwrap();
    assert_eq!(f.cap_faces().len(), 2);
    assert!(
        f.cap_faces()
            .iter()
            .all(|c| f.tool().faces[*c].holes.is_empty())
    );
    assert!((volume(f.tool()) - 16.).abs() < 1e-6);
    for face in faces {
        let id = s.1.faces[face];
        let copied = f.tool().1.faces.iter().position(|p| *p == id).unwrap();
        assert_eq!(f.tool().faces[copied].surface, s.faces[face].surface);
    }
    assert!(f.reverify(Limits::default()).unwrap().boundary_complete);
    let p = region_pattern::pattern(
        &s,
        &f,
        &[shift("next", 0., 5., 0.)],
        PatternMode::Join,
        "bridge-feet",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(p.model()) - 320.).abs() < 1e-6);
    assert!(p.reverify(&budget()).unwrap().trim_regions_valid);
    let cut = region_pattern::pattern(
        &s,
        &f,
        &[shift("arch-cut", 6., 5., -3.)],
        PatternMode::Cut,
        "bridge-cut",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(cut.model()) - 292.).abs() < 1e-6);
    assert_eq!(s, original);
}
fn rectangle(lo: f64, hi: f64, reverse: bool) -> Vec<Curve> {
    let mut p = vec![[lo, lo], [hi, lo], [hi, hi], [lo, hi]];
    if reverse {
        p.reverse();
    }
    (0..4)
        .map(|i| Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()]).unwrap())
        .collect()
}
fn vented_nested_source() -> Model {
    let annular =
        brep_core::prism::extrude(&[rectangle(1., 7., false), rectangle(2., 6., true)], 2., 4.)
            .unwrap();
    let a = union(&stock(), &annular);
    let b = union(&a, &box3([3., 3., 2.], [5., 5., 4.]));
    let c = union(&b, &box3([1., 1., 4.], [7., 7., 5.]));
    // The vent joins the enclosed air region to the outside; all selected
    // boundary faces then belong to one original outer shell.
    brep_core::operations::boolean(&c, &box3([0., 3., 2.5], [3., 4., 3.5]), "difference").unwrap()
}
#[test]
fn nested_island_gets_its_own_cap_inside_the_other_caps_hole() {
    let s = vented_nested_source();
    assert!(s.bodies[0].inner_shells.is_empty());
    let f = region_pattern::extract(&s, &selected(&s), FeatureRole::Additive, Limits::default())
        .unwrap();
    assert_eq!(f.cap_faces().len(), 2);
    let mut holes = f
        .cap_faces()
        .iter()
        .map(|c| f.tool().faces[*c].holes.len())
        .collect::<Vec<_>>();
    holes.sort();
    assert_eq!(holes, vec![0, 1]);
    // Ring columns: (36-16)*2, center column: 4*2, lid:36, vent:-1.
    assert!((volume(f.tool()) - 83.).abs() < 1e-6);
    assert!((volume(&s) - 371.).abs() < 1e-6);
    assert!(f.reverify(Limits::default()).unwrap().trim_regions_valid);
    let empty = Model::empty(s.tolerance_mm).unwrap();
    let p = region_pattern::pattern(
        &empty,
        &f,
        &[shift("nested", 0., 0., 0.)],
        PatternMode::Join,
        "nested-cap",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(p.model()) - 83.).abs() < 1e-6);
}
#[test]
fn multi_cap_construction_accounts_shared_separation_and_winding_budgets() {
    let s = bridge();
    let original = s.clone();
    let faces = selected(&s);
    let f = region_pattern::extract(&s, &faces, FeatureRole::Additive, Limits::default()).unwrap();
    assert!(f.evidence().pairs > 0 && f.evidence().domain_cells > 0);
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
        assert!(region_pattern::extract(&s, &faces, FeatureRole::Additive, limits).is_err());
        assert!(f.reverify(limits).is_err());
    }
    assert_eq!(s, original);
}
#[test]
fn component_ids_and_order_follow_original_rims_under_storage_reordering() {
    let s = bridge();
    let f = region_pattern::extract(&s, &selected(&s), FeatureRole::Additive, Limits::default())
        .unwrap();
    let mut reordered = s.clone();
    let ne = reordered.edges.len();
    reordered.edges.reverse();
    reordered.1.edges.reverse();
    for w in &mut reordered.loops {
        for c in &mut w.coedges {
            c.edge = ne - 1 - c.edge;
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
    let ids = |r: &region_pattern::FeatureRegion| {
        r.cap_faces()
            .iter()
            .map(|c| r.tool().1.faces[*c])
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&f), ids(&g));
    let empty = Model::empty(s.tolerance_mm).unwrap();
    let a = region_pattern::pattern(
        &empty,
        &f,
        &[shift("same", 0., 0., 0.)],
        PatternMode::Join,
        "stable-cap",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    let b = region_pattern::pattern(
        &empty,
        &g,
        &[shift("same", 0., 0., 0.)],
        PatternMode::Join,
        "stable-cap",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert_eq!(a.instances()[0].tool_ids(), b.instances()[0].tool_ids());
}
fn circle(x: f64) -> Vec<Curve> {
    let (y, r) = (3., 1.);
    let p = [
        [[x + r, y], [x + r, y + r], [x, y + r]],
        [[x, y + r], [x - r, y + r], [x - r, y]],
        [[x - r, y], [x - r, y - r], [x, y - r]],
        [[x, y - r], [x + r, y - r], [x + r, y]],
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
fn rational_bridge_feet_use_whole_curve_nesting_without_faceting() {
    let columns = brep_core::prism::extrude(&[circle(2.), circle(6.)], 2., 4.).unwrap();
    let a = union(&stock(), &columns);
    let s = union(&a, &box3([0.5, 1.5, 4.], [7.5, 4.5, 5.]));
    let f = region_pattern::extract(&s, &selected(&s), FeatureRole::Additive, Limits::default())
        .unwrap();
    assert_eq!(f.cap_faces().len(), 2);
    assert!((volume(f.tool()) - 21. - 4. * std::f64::consts::PI).abs() < 1e-5);
    for (i, id) in f.tool().1.edges.iter().enumerate() {
        let original = s.1.edges.iter().position(|p| p == id).unwrap();
        assert_eq!(f.tool().edges[i].curve, s.edges[original].curve);
    }
    assert!(f.tool().edges.iter().any(|e| e.curve.degree > 1));
    assert!(f.reverify(Limits::default()).unwrap().trim_regions_valid);
}
