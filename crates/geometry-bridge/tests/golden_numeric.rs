//! Bit-exact golden outputs for the code paths whose private vector/scalar
//! helpers are shared with `math_core` (or within one crate). Each case is
//! reduced to a SHA-256 over its exact f64 bit patterns (`Value` floats as
//! `to_bits`, `Debug` output for typed results, whose shortest round-trip f64
//! formatting is injective on bit patterns). Any numeric change fails.
//!
//! Regenerate only for an intentional numeric change:
//! `UPDATE_GOLDEN=1 cargo test -p geometry-bridge --test golden_numeric`.
use brep_core::cuboid;
use geometry_bridge::{brep, dispatch};
use nurbs_core::{curve::Curve, primitives, surface};
use planar_geometry::{
    corners::{self, CornerStyle},
    curve::BezPath,
    curve_boolean::{BinaryOp, FillRule, binary_op_with_eps},
    edit, effects,
    path::BezierPath,
    scissors,
    stroke::{self, LineCap, LineJoin, StrokeOptions},
};
use sha2::{Digest, Sha256};
use std::fmt::Debug;
use value_codec::{Number, Value, json, to_value};

const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/golden_numeric.txt");

fn walk(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push('n'),
        Value::Bool(b) => out.push(if *b { 't' } else { 'f' }),
        Value::Number(Number::Float(x)) => out.push_str(&format!("d{:016x}", x.to_bits())),
        Value::Number(Number::Unsigned(x)) => out.push_str(&format!("u{x}")),
        Value::Number(Number::Signed(x)) => out.push_str(&format!("i{x}")),
        Value::String(s) => out.push_str(&format!("s{}:{s}", s.len())),
        Value::Array(a) => {
            out.push('[');
            a.iter().for_each(|x| walk(x, out));
            out.push(']');
        }
        Value::Object(m) => {
            out.push('{');
            for (k, x) in m {
                out.push_str(&format!("k{}:{k}", k.len()));
                walk(x, out);
            }
            out.push('}');
        }
    }
}

struct Cases(Vec<(String, String)>);
impl Cases {
    fn push(&mut self, name: &str, canonical: String) {
        let hash = Sha256::digest(canonical.as_bytes());
        let hex = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();
        self.0.push((name.to_owned(), hex));
    }
    fn value(&mut self, name: &str, v: Value) {
        let mut s = String::new();
        walk(&v, &mut s);
        self.push(name, s);
    }
    fn debug(&mut self, name: &str, v: impl Debug) {
        self.push(name, format!("{v:?}"));
    }
}

fn poly(points: &[[f64; 2]], closed: bool) -> BezierPath {
    BezierPath::from_polyline(points, closed).unwrap()
}

fn bez(points: &[(f64, f64)]) -> BezPath {
    let mut p = BezPath::new();
    p.move_to(points[0]);
    for &q in &points[1..] {
        p.line_to(q);
    }
    p.close_path();
    p
}

fn planar(c: &mut Cases) {
    let star: Vec<[f64; 2]> = (0..10)
        .map(|i| {
            let a = i as f64 * std::f64::consts::PI / 5. + 0.1;
            let r = if i % 2 == 0 { 10. } else { 4.3 };
            [r * a.cos() + 0.37, r * a.sin() - 0.21]
        })
        .collect();
    let square = poly(&[[0., 0.], [20., 0.], [20., 20.], [0., 20.]], true);
    let hole = poly(&[[5., 5.], [5., 15.], [15., 15.], [15., 5.]], true);
    let open = poly(&[[0., 0.], [10., 0.3], [10.7, 10.], [-3., 13.1]], false);

    c.debug(
        "planar.rounded_rect",
        corners::rounded_rect(
            [-1.3, 0.2],
            [17.9, 9.4],
            &[1., 2.5, 0.7, 3.],
            &[
                CornerStyle::Round,
                CornerStyle::Chamfer,
                CornerStyle::Inverted,
                CornerStyle::Notch,
            ],
        ),
    );
    c.debug(
        "planar.rounded_polygon",
        corners::rounded_polygon(&star, &[0.8, 1.3], &[CornerStyle::Round]),
    );
    c.debug("planar.round_corners", corners::round_corners(&open, 2.2));
    c.debug(
        "planar.knife_split",
        scissors::knife_split_compound(&[square.clone(), hole.clone()], [-1., 3.3], [21., 17.1]),
    );
    c.debug(
        "planar.knife_cut",
        scissors::knife_cut_compound(&[poly(&star, true)], [-11., 0.7], [11., -1.9]),
    );
    for (cap, join, name) in [
        (LineCap::Round, LineJoin::Round, "planar.stroke.round"),
        (LineCap::Square, LineJoin::Miter, "planar.stroke.miter"),
        (LineCap::Butt, LineJoin::Bevel, "planar.stroke.bevel"),
    ] {
        let options = StrokeOptions {
            width: 1.7,
            cap,
            join,
            ..Default::default()
        };
        c.debug(name, stroke::outline_stroke_tol(&open, &options, 0.01));
        c.debug(
            &format!("{name}.closed"),
            stroke::outline_stroke_tol(&poly(&star, true), &options, 0.01),
        );
    }
    let curved = corners::round_corners(&poly(&star, true), 1.5).unwrap();
    c.debug("planar.flatten", curved.flatten_tol(0.003));
    c.debug("planar.to_ring", curved.to_ring(0.01));
    for (i, cursor) in [[9.9, 0.2], [5.1, 5.05], [3.3, 7.7], [0.21, 13.]]
        .into_iter()
        .enumerate()
    {
        c.debug(
            &format!("planar.find_snap.{i}"),
            edit::find_snap(cursor, 1.5, Some(0.75), &[square.clone(), curved.clone(), open.clone()]),
        );
    }
    let rings = vec![square.to_ring(0.01).unwrap(), hole.to_ring(0.01).unwrap()];
    c.debug(
        "planar.concentric_offset",
        effects::concentric_offset(&rings, 3, 1.1, "round", 8),
    );
    c.debug(
        "planar.radial_repeat",
        effects::radial_repeat(&curved, 5, [1.5, -2.]),
    );
    let a = bez(&[(0., 0.), (10., 0.), (10., 10.), (0., 10.)]);
    let b = bez(&[(5.3, 2.1), (15.7, 4.4), (12.2, 13.9), (3.9, 11.6)]);
    for op in [
        BinaryOp::Union,
        BinaryOp::Intersection,
        BinaryOp::Difference,
        BinaryOp::Xor,
    ] {
        c.debug(
            &format!("planar.boolean.{op:?}"),
            binary_op_with_eps(&a, &b, FillRule::NonZero, op, 0.05).map(|r| r.to_path()),
        );
    }
}

fn nurbs(c: &mut Cases) {
    let a = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 0.4, 1., 1., 1., 1.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 2., 0.3],
            vec![2.5, -1., 0.1],
            vec![3.7, 1.4, -0.5],
            vec![5., 0.2, 0.],
        ],
        weights: vec![1., 0.8, 1.3, 1., 1.],
        periodic: false,
    };
    let b = primitives::circle([2.4, 0.4, 0.], [0.1, 0.2, 1.], 1.7).unwrap();
    let far = primitives::line([0., 5., 3.], [4., 6., 3.5]).unwrap();
    c.value(
        "nurbs.intersect_curve_curve",
        nurbs_core::intersection::intersect_curve_curve(&a, &b, None).unwrap(),
    );
    c.debug(
        "nurbs.curve_distance",
        nurbs_core::curve_distance::distance(&a, &far, 1e-4, 4000),
    );
    c.debug(
        "nurbs.curve_separation",
        nurbs_core::curve_distance::prove_separation(&a, &far, 1e-4, 4000),
    );
    let s = surface::extrude(&a, [0.2, 0.3, 2.]).unwrap();
    let t = surface::extrude(&b, [0., 0., 1.5]).unwrap();
    let u = surface::extrude(&far, [0.3, -0.4, 1.]).unwrap();
    c.value(
        "nurbs.intersect_curve_surface",
        nurbs_core::intersection::intersect_curve_surface(&far, &s, None)
            .unwrap_or_else(|e| json!({ "err": format!("{e:?}") })),
    );
    c.value(
        "nurbs.intersect_curve_surface.2",
        nurbs_core::intersection::intersect_curve_surface(&a, &t, None)
            .unwrap_or_else(|e| json!({ "err": format!("{e:?}") })),
    );
    c.value(
        "nurbs.intersect_surface_surface",
        nurbs_core::ss_intersection::intersect_surface_surface(&s, &t, None)
            .unwrap_or_else(|e| json!({ "err": format!("{e:?}") })),
    );
    c.debug(
        "nurbs.surface_distance",
        nurbs_core::surface_distance::distance(&s, &u, 1e-3, 3000),
    );
    for (i, uv) in [[0.1, 0.2], [0.55, 0.5], [0.93, 0.81]].into_iter().enumerate() {
        c.debug(
            &format!("nurbs.surface_offset.{i}"),
            nurbs_core::surface_offset::evaluate(&s, uv, 0.25),
        );
    }
    let pts = [
        [0., 0., 0.],
        [10., 0., 0.5],
        [10.4, 7., 1.],
        [3., 9., -2.],
        [0.5, 4., 0.],
    ];
    c.debug(
        "nurbs.round_polyline",
        nurbs_core::paths::round_polyline(&pts, 1.25),
    );
    c.debug(
        "nurbs.closed_round_polyline",
        nurbs_core::paths::closed_round_polyline(&pts, 0.9),
    );
    c.debug(
        "nurbs.transition_polyline",
        nurbs_core::paths::transition_polyline(&pts, 0.7),
    );
}

fn sketch(c: &mut Cases) {
    use sketch_core::{Circle, Constraint, Sketch, solve};
    let s = Sketch {
        points: vec![[0., 0.], [10., 0.], [3., 3.], [7., 6.], [1., 8.]],
        circles: vec![Circle {
            center: 2,
            radius: 1.,
        }],
        constraints: vec![
            Constraint::Fix {
                point: 0,
                at: [0., 0.],
            },
            Constraint::Fix {
                point: 1,
                at: [10., 0.],
            },
            Constraint::Radius {
                circle: 0,
                value: 2.,
            },
            Constraint::TangentLineCircle {
                a: 0,
                b: 1,
                circle: 0,
            },
            Constraint::Distance {
                a: 2,
                b: 3,
                value: 5.5,
            },
            Constraint::Coincident { a: 3, b: 3 },
            Constraint::Distance {
                a: 3,
                b: 4,
                value: 6.25,
            },
        ],
    };
    c.value("sketch.solve", to_value(solve(&s, 1e-7).unwrap()).unwrap());
}

fn bridge(c: &mut Cases) {
    for (name, model, segments) in [
        ("cuboid", cuboid([0.; 3], [2., 3., 4.]).unwrap(), 2),
        ("sphere", brep_core::sphere(2.).unwrap(), 6),
    ] {
        let mesh = brep::nurbs(&model, segments).unwrap().built.mesh;
        let mesh = to_value(&mesh).unwrap();
        c.value(
            &format!("bridge.cad_mesh_topology.{name}"),
            dispatch(json!({"op":"cad_mesh_topology","mesh":mesh})).unwrap(),
        );
        for (tolerance, target) in [(0.05, 0.), (0.3, 40.)] {
            c.value(
                &format!("bridge.cad_lattice_decimate.{name}.{tolerance}"),
                dispatch(json!({"op":"cad_lattice_decimate","mesh":mesh,"tolerance":tolerance,"target":target}))
                    .unwrap_or_else(|e| json!({ "err": format!("{e:?}") })),
            );
        }
    }
}

#[test]
fn golden_numeric_outputs_are_bit_identical() {
    let mut c = Cases(Vec::new());
    planar(&mut c);
    nurbs(&mut c);
    sketch(&mut c);
    bridge(&mut c);
    let actual = c
        .0
        .iter()
        .map(|(name, hash)| format!("{name} {hash}\n"))
        .collect::<String>();
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(GOLDEN, &actual).unwrap();
        return;
    }
    // sin/cos/atan2/hypot come from the platform libm, whose last-bit results differ between
    // glibc, macOS and MSVC. The hashes were recorded on x86_64 Linux (the CI `rust` job), so
    // only that target compares them; elsewhere every case still runs and must not panic.
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return;
    }
    let expected = std::fs::read_to_string(GOLDEN)
        .expect("missing golden file; run with UPDATE_GOLDEN=1 to create it");
    let changed = actual
        .lines()
        .zip(expected.lines())
        .filter(|(a, e)| a != e)
        .map(|(a, _)| a.split(' ').next().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert!(
        changed.is_empty() && actual.lines().count() == expected.lines().count(),
        "golden numeric outputs changed: {changed:?}"
    );
}
