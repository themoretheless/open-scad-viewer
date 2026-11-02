use brep_core::{
    feature_ops::full_round_curved::build,
    profile_certificate::original_restriction::wire::{jordan, nesting},
    Model,
};
use num_rational::BigRational as Q;
use num_traits::{FromPrimitive, ToPrimitive};
use nurbs_core::curve::Curve;
fn source() -> Model {
    let c = |p: [[f64; 2]; 3]| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: p.map(|p| p.to_vec()).to_vec(),
        weights: vec![1., 0.7, 1.],
        periodic: false,
    };
    let line =
        |a: [f64; 2], b: [f64; 2]| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
    brep_core::prism::extrude(
        &[vec![
            line([0., 0.], [4., 0.]),
            c([[4., 0.], [5., 4.], [4., 8.]]),
            line([4., 8.], [0., 8.]),
            c([[0., 8.], [-1., 4.], [0., 0.]]),
        ]],
        0.,
        6.,
    )
    .unwrap()
}
fn decode(v: &[String; 2]) -> Q {
    Q::new(v[0].parse().unwrap(), v[1].parse().unwrap())
}
#[test]
fn original_raw_edge_clocks_and_exact_contact_parameter_map_are_authoritative() {
    let mut m = source();
    for (i, e) in m.edges.iter_mut().enumerate() {
        let a = 0.1 + i as f64 * 2.;
        let b = a + 1.8;
        for k in &mut e.curve.knots {
            *k = if *k == 0. { a } else { b };
        }
    }
    let edge = m
        .edges
        .iter()
        .position(|e| {
            e.curve.degree == 2
                && e.curve
                    .control_points
                    .iter()
                    .all(|p| p[2] == 6. && p[0] >= 4.)
        })
        .unwrap();
    let e = &mut m.edges[edge];
    e.vertices.swap(0, 1);
    e.curve.control_points.reverse();
    e.curve.weights.reverse();
    e.curve.knots = e.curve.knots.iter().rev().map(|k| -k).collect();
    for w in &mut m.loops {
        for c in &mut w.coedges {
            if c.edge == edge {
                c.reversed = !c.reversed;
            }
        }
    }
    m.validate().unwrap();
    let before = m.clone();
    let r = build(&m, [3, 2, 1]).unwrap();
    let caps = r.qualify_source_caps().unwrap();
    caps.reverify().unwrap();
    let pullbacks = caps.canonical_pullbacks().unwrap();
    assert_eq!(pullbacks.original().source(), &before);
    pullbacks.reverify().unwrap();
    r.qualify_source_boundary().unwrap().reverify().unwrap();
    replay_exact_recipe(&r, &m);
    r.qualify_source_supports()
        .unwrap()
        .qualify_native_profiles()
        .unwrap()
        .reverify()
        .unwrap();
    r.qualify_source_supports().unwrap().reverify().unwrap();
    assert_eq!(m, before);
    let raw = &r.raw_profile_sources()[0][1];
    assert_eq!(raw.edge(), edge);
    assert!(raw.reversed());
    raw.reverify(&m).unwrap();
    let mut expected = m.edges[edge].curve.clone();
    for p in &mut expected.control_points {
        p.truncate(2);
    }
    expected.control_points.reverse();
    expected.weights.reverse();
    expected.knots = expected.knots.iter().rev().map(|k| -k).collect();
    assert_eq!(raw.curve(), &expected);
    assert_eq!(r.retained_supports()[0].source(), raw.curve());
    let d = raw.curve().domain().map(|x| Q::from_f64(x).unwrap());
    let t = Q::from_f64(r.proposal().parameters()[0]).unwrap();
    let exact = &d[0] + t * (&d[1] - &d[0]);
    let actual = decode(&r.retained_supports()[0].parameter_range()[1]);
    assert_eq!(actual, exact);
    assert_ne!(actual, Q::from_f64(actual.to_f64().unwrap()).unwrap());
    for (piece, editable) in r.exact_profile()[0].editable().iter().enumerate() {
        if !editable {
            assert_ne!(
                r.exact_profile()[0].sources()[piece].source().domain(),
                [0., 1.]
            );
        }
    }
    let solid = r
        .qualify_exact_solid(jordan::Limits::default(), nesting::Limits::default())
        .unwrap();
    solid.reverify().unwrap();
    let mut changed = m.clone();
    changed.edges[edge].curve.control_points[1][0] += 1e-4;
    assert!(raw.reverify(&changed).is_err());
}
#[test]
fn reversed_top_carrier_and_coedges_bind_physical_source_traversal() {
    let mut m = source();
    let top = m
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 6.)
        })
        .unwrap();
    let cp = m.faces[top].surface.control_points.clone();
    let weights = m.faces[top].surface.weights.clone();
    m.faces[top].surface.control_points = (0..2)
        .map(|i| (0..2).map(|j| cp[j][i].clone()).collect())
        .collect();
    m.faces[top].surface.weights = (0..2)
        .map(|i| (0..2).map(|j| weights[j][i]).collect())
        .collect();
    let loops = std::iter::once(m.faces[top].outer)
        .chain(m.faces[top].holes.iter().copied())
        .collect::<Vec<_>>();
    for w in loops {
        m.loops[w].coedges.reverse();
        for c in &mut m.loops[w].coedges {
            c.reversed = !c.reversed;
            c.pcurve = c.pcurve.reverse().unwrap();
            for p in &mut c.pcurve.control_points {
                p.swap(0, 1);
            }
        }
    }
    for s in &mut m.shells {
        for u in &mut s.faces {
            if u.face == top {
                u.reversed = !u.reversed;
            }
        }
    }
    m.validate().unwrap();
    let before = m.clone();
    let r = build(&m, [3, 2, 1]).unwrap();
    let caps = r.qualify_source_caps().unwrap();
    caps.reverify().unwrap();
    let pullbacks = caps.canonical_pullbacks().unwrap();
    assert_eq!(pullbacks.original().source(), &before);
    pullbacks.reverify().unwrap();
    r.qualify_source_boundary().unwrap().reverify().unwrap();
    replay_exact_recipe(&r, &m);
    r.qualify_source_supports()
        .unwrap()
        .qualify_native_profiles()
        .unwrap()
        .reverify()
        .unwrap();
    assert_eq!(m, before);
    for raw in &r.raw_profile_sources()[0] {
        assert_eq!(raw.face(), top);
        raw.reverify(&m).unwrap();
    }
    r.qualify_exact_solid(jordan::Limits::default(), nesting::Limits::default())
        .unwrap()
        .reverify()
        .unwrap();
}

fn replay_exact_recipe(
    r: &brep_core::feature_ops::full_round_curved::CurvedFullRound,
    source: &brep_core::Model,
) {
    use brep_core::feature_ops::full_round_curved::admitted::recipe::{self, Recipe};
    let recipe = Recipe {
        contact_ranges: r
            .retained_supports()
            .each_ref()
            .map(|r| r.parameter_range()),
        bridges: r.proposal().bridges().to_vec(),
        center: r.exact_radius().center(),
        radius: r.exact_radius().radius(),
    };
    let restored = recipe::restore(source, [3, 2, 1], &recipe, Default::default()).unwrap();
    let original = r
        .qualify_exact_solid(Default::default(), Default::default())
        .unwrap();
    assert_eq!(restored.model(), original.model());
    restored.reverify().unwrap();
    let lineage = restored.lineage().unwrap();
    lineage.reverify(&restored).unwrap();
    for record in &lineage.faces {
        if let brep_core::feature_ops::full_round_curved::admitted::lineage::FaceOrigin::RetainedSide {rim,source_loop,coedge,stored_range,reversed,..}=&record.origin {
            let edge=source.1.edges.iter().position(|id|id==rim).unwrap();
            assert_eq!(source.loops[coedge[0]].coedges[coedge[1]].edge,edge);
            assert_eq!(*source_loop,source.1.loops[coedge[0]]);
            let domain=source.edges[edge].curve.domain().map(|x|num_rational::BigRational::from_float(x).unwrap());
            let decode=|r:&[String;2]|num_rational::BigRational::new(r[0].parse().unwrap(),r[1].parse().unwrap());
            let range=stored_range.each_ref().map(decode);
            assert!(range.iter().all(|t|t>=&domain[0] && t<=&domain[1]));
            assert_eq!(range[0]>range[1],*reversed);
        }
    }
}
