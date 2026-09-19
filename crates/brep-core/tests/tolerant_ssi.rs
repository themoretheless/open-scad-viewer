//! Coverage of the tolerant (numerical SSI) Boolean through the public API:
//! operand matrix, volume identities, cross-validation against exact
//! families, rigid-motion and operand-order invariance, multi-body and
//! genus-changing results, the stated tolerance audited on every fitted
//! edge, and explicit refusals.
use brep_core::tolerant_boolean::{self, TOLERANT_FLOOR};
use brep_core::{
    Model, analysis::mass_properties, boolean, cuboid, cylinder, sphere, torus, transform,
};

fn translate(model: &Model, at: [f64; 3]) -> Model {
    transform::affine(
        model,
        [
            [1., 0., 0., at[0]],
            [0., 1., 0., at[1]],
            [0., 0., 1., at[2]],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap()
}

/// Rotation about a unit axis by an angle, then translation.
fn rigid(model: &Model, axis: [f64; 3], degrees: f64, at: [f64; 3]) -> Model {
    let n = (axis[0].powi(2) + axis[1].powi(2) + axis[2].powi(2)).sqrt();
    let [x, y, z] = [axis[0] / n, axis[1] / n, axis[2] / n];
    let (s, c) = degrees.to_radians().sin_cos();
    let t = 1. - c;
    transform::affine(
        model,
        [
            [t * x * x + c, t * x * y - s * z, t * x * z + s * y, at[0]],
            [t * x * y + s * z, t * y * y + c, t * y * z - s * x, at[1]],
            [t * x * z - s * y, t * y * z + s * x, t * z * z + c, at[2]],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap()
}

fn volume(model: &Model) -> f64 {
    mass_properties(model, 1e-5, 2_000_000)
        .unwrap()
        .signed_volume_mm3
}

fn near(a: f64, b: f64, relative: f64, what: &str) {
    let scale = a.abs().max(b.abs()).max(1e-9);
    assert!(
        (a - b).abs() <= relative * scale,
        "{what}: {a} vs {b} (relative {:.2e})",
        (a - b).abs() / scale
    );
}

/// Every fitted (degree-3) edge of a tolerant result must agree with both
/// adjacent surfaces through its pcurves within the stated tolerance, at a
/// far denser sampling than the validator uses.
fn audit_tolerance(model: &Model) {
    let tol = model.tolerance_mm;
    let mut audited = 0;
    for face in &model.faces {
        for &wire in std::iter::once(&face.outer).chain(&face.holes) {
            for coedge in &model.loops[wire].coedges {
                let edge = &model.edges[coedge.edge];
                if edge.curve.degree != 3 {
                    continue;
                }
                audited += 1;
                for i in 0..=64 {
                    let t = i as f64 / 64.;
                    let uv = coedge.pcurve.evaluate(t).unwrap().point;
                    let on_surface = face.surface.evaluate(uv[0], uv[1]).unwrap().point;
                    let s = if coedge.reversed { 1. - t } else { t };
                    let on_edge = edge.curve.evaluate(s).unwrap().point;
                    let d = (0..3)
                        .map(|k| (on_surface[k] - on_edge[k]).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    assert!(
                        d <= tol,
                        "fitted edge deviates {d:.3e} mm from its face, tolerance {tol:.3e}"
                    );
                }
            }
        }
    }
    assert!(audited > 0, "a tolerant result must contain fitted edges");
}

fn euler_characteristic(model: &Model) -> i64 {
    model.vertices.len() as i64 - model.edges.len() as i64 + model.faces.len() as i64
}

/// Runs the three operations, validates, audits tolerance and checks the
/// volume identities `|A∪B| = |A| + |B| - |A∩B|` and `|A-B| = |A| - |A∩B|`.
fn identities(a: &Model, b: &Model, what: &str) -> [Model; 3] {
    let union = boolean(a, b, "union").unwrap();
    let difference = boolean(a, b, "difference").unwrap();
    let intersection = boolean(a, b, "intersection").unwrap();
    for (op, m) in [
        ("union", &union),
        ("difference", &difference),
        ("intersection", &intersection),
    ] {
        m.validate().unwrap();
        assert!(
            m.tolerance_mm >= TOLERANT_FLOOR,
            "{what} {op} should be tolerant"
        );
        audit_tolerance(m);
    }
    let (va, vb) = (volume(a), volume(b));
    let vi = volume(&intersection);
    near(
        volume(&union),
        va + vb - vi,
        2e-3,
        &format!("{what}: union identity"),
    );
    near(
        volume(&difference),
        va - vi,
        2e-3,
        &format!("{what}: difference identity"),
    );
    assert!(
        vi > 0. && vi < va.min(vb),
        "{what}: intersection volume in range"
    );
    [union, difference, intersection]
}

#[test]
fn off_axis_sphere_in_a_cylinder_wall() {
    let c = cylinder(3., 10.).unwrap();
    let s = translate(&sphere(2.).unwrap(), [2.5, 0.4, 5.]);
    let [union, difference, intersection] = identities(&c, &s, "sphere in wall");
    assert_eq!(union.bodies.len(), 1);
    assert_eq!(difference.bodies.len(), 1);
    assert_eq!(intersection.bodies.len(), 1);
    assert_eq!(euler_characteristic(&intersection), 2);
}

#[test]
fn crossing_cylinders_of_different_radii() {
    let a = cylinder(2., 12.).unwrap();
    let b = rigid(
        &cylinder(1.2, 12.).unwrap(),
        [1., 0., 0.],
        90.,
        [0.3, 6., 6.],
    );
    let [union, _, intersection] = identities(&a, &b, "crossing cylinders");
    assert_eq!(union.bodies.len(), 1);
    // The intersection is a single convex-ish lens-like solid.
    assert_eq!(intersection.bodies.len(), 1);
    assert_eq!(euler_characteristic(&intersection), 2);
}

#[test]
fn torus_through_a_box() {
    let b = cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let t = translate(&torus(4., 1.).unwrap(), [0.3, 0.7, 0.4]);
    identities(&b, &t, "torus through box");
}

#[test]
fn thin_cylinder_drilled_through_a_sphere_changes_genus() {
    let s = sphere(5.).unwrap();
    let drill = translate(&cylinder(1., 20.).unwrap(), [0.3, 0.2, -10.]);
    let [_, drilled, plug] = identities(&s, &drill, "drilled sphere");
    assert_eq!(drilled.bodies.len(), 1);
    assert_eq!(
        euler_characteristic(&drilled),
        0,
        "a drilled sphere is a torus"
    );
    assert_eq!(euler_characteristic(&plug), 2, "the plug is a ball");
}

#[test]
fn a_wide_sphere_cuts_a_cylinder_into_two_stubs() {
    let c = cylinder(3., 10.).unwrap();
    let s = translate(&sphere(4.).unwrap(), [0.2, 0.1, 5.]);
    let [union, difference, intersection] = identities(&c, &s, "two stubs");
    assert_eq!(difference.bodies.len(), 2);
    assert_eq!(union.bodies.len(), 1);
    assert_eq!(intersection.bodies.len(), 1);
}

#[test]
fn tolerant_path_agrees_with_exact_families() {
    // Box dome (exact box/sphere family) and a generic sphere pair (exact
    // sphere/sphere family): the tolerant path, forced directly, must give
    // the same volumes.
    let cases = [
        (
            cuboid([-10., -10., -10.], [10., 10., 10.]).unwrap(),
            translate(&sphere(5.).unwrap(), [0.3, -0.7, 8.]),
            "box dome",
        ),
        (
            sphere(3.).unwrap(),
            translate(&sphere(3.).unwrap(), [4.1, 0.3, -0.2]),
            "sphere pair",
        ),
        (
            cylinder(3., 10.).unwrap(),
            translate(&sphere(2.).unwrap(), [0., 0., 9.]),
            "axial cap ring",
        ),
    ];
    for (a, b, what) in &cases {
        for op in ["union", "difference", "intersection"] {
            let exact = boolean(a, b, op).unwrap();
            assert!(exact.tolerance_mm < TOLERANT_FLOOR, "{what} {op} is exact");
            let tolerant = tolerant_boolean::boolean(a, b, op).unwrap();
            tolerant.validate().unwrap();
            audit_tolerance(&tolerant);
            near(
                volume(&tolerant),
                volume(&exact),
                1e-3,
                &format!("{what} {op}: tolerant vs exact volume"),
            );
            assert_eq!(tolerant.bodies.len(), exact.bodies.len(), "{what} {op}");
        }
    }
}

#[test]
fn operand_order_and_rigid_motion_do_not_change_volumes() {
    let c = cylinder(3., 10.).unwrap();
    let s = translate(&sphere(2.).unwrap(), [2.5, 0.4, 5.]);
    let v_union = volume(&boolean(&c, &s, "union").unwrap());
    let v_int = volume(&boolean(&c, &s, "intersection").unwrap());
    near(
        volume(&boolean(&s, &c, "union").unwrap()),
        v_union,
        2e-3,
        "union commutes",
    );
    near(
        volume(&boolean(&s, &c, "intersection").unwrap()),
        v_int,
        2e-3,
        "intersection commutes",
    );
    near(
        volume(&boolean(&s, &c, "difference").unwrap()),
        volume(&s) - v_int,
        2e-3,
        "swapped difference identity",
    );
    for (axis, degrees, at) in [
        ([1., 0., 0.], 37., [3., -2., 1.]),
        ([0.3, 1., 0.2], 118., [-5., 4., 7.]),
        ([1., 1., 1.], 200., [0., 0., -3.]),
    ] {
        let rc = rigid(&c, axis, degrees, at);
        let rs = rigid(&s, axis, degrees, at);
        near(
            volume(&boolean(&rc, &rs, "union").unwrap()),
            v_union,
            2e-3,
            "union under rigid motion",
        );
        near(
            volume(&boolean(&rc, &rs, "intersection").unwrap()),
            v_int,
            2e-3,
            "intersection under rigid motion",
        );
    }
}

#[test]
fn tolerance_is_stated_bounded_and_honest() {
    let c = cylinder(3., 10.).unwrap();
    let s = translate(&sphere(2.).unwrap(), [2.5, 0.4, 5.]);
    let m = boolean(&c, &s, "difference").unwrap();
    assert!(m.tolerance_mm >= TOLERANT_FLOOR);
    assert!(m.tolerance_mm <= 1e-2);
    // Exact inputs keep the kernel tolerance; the fallback is the only path
    // that raises it.
    assert!(c.tolerance_mm < TOLERANT_FLOOR && s.tolerance_mm < TOLERANT_FLOOR);
    audit_tolerance(&m);
    // Serialization round-trip keeps the stated tolerance.
    let restored: Model = value_codec::from_str(&value_codec::to_string(&m).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored.tolerance_mm, m.tolerance_mm);
}

#[test]
fn results_chain_into_further_booleans() {
    let c = cylinder(3., 10.).unwrap();
    let s = translate(&sphere(2.).unwrap(), [2.5, 0.4, 5.]);
    let dimpled = boolean(&c, &s, "difference").unwrap();
    // A second, exact-capable cut on a tolerant body still resolves (through
    // the tolerant path, since the body is no longer canonical).
    // (Not [-2.4, 0.3, 3]: there the sphere's +y vertex would sit exactly on
    // the wall, 2.4² + 1.8² = 9, a vertex contact the trace refuses.)
    let s2 = translate(&sphere(1.5).unwrap(), [-2.4, 0.35, 3.1]);
    let twice = boolean(&dimpled, &s2, "difference").unwrap();
    twice.validate().unwrap();
    audit_tolerance(&twice);
    near(
        volume(&twice),
        volume(&dimpled) - volume(&boolean(&dimpled, &s2, "intersection").unwrap()),
        2e-3,
        "second cut identity",
    );
}

#[test]
fn containment_and_separation_through_the_tolerant_path() {
    let c = cylinder(3., 10.).unwrap();
    let inside = translate(&sphere(1.).unwrap(), [0.7, -0.4, 5.]);
    let cavity = tolerant_boolean::boolean(&c, &inside, "difference").unwrap();
    assert_eq!(cavity.bodies[0].inner_shells.len(), 1);
    assert!(
        tolerant_boolean::boolean(&inside, &c, "difference")
            .unwrap()
            .is_empty()
    );
    let around = translate(&sphere(20.).unwrap(), [1., 2., 3.]);
    assert_eq!(
        tolerant_boolean::boolean(&c, &around, "union")
            .unwrap()
            .faces
            .len(),
        8
    );
    let far = translate(&sphere(1.).unwrap(), [40., 0., 0.]);
    assert_eq!(
        tolerant_boolean::boolean(&c, &far, "union")
            .unwrap()
            .bodies
            .len(),
        2
    );
    assert!(
        tolerant_boolean::boolean(&c, &far, "intersection")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn short_cylinder_with_a_wall_sphere_as_in_the_solid_workspace() {
    let c = cylinder(3., 5.).unwrap();
    let s = translate(&sphere(2.).unwrap(), [2.5, 0.4, 2.5]);
    let m = boolean(&c, &s, "difference").unwrap_or_else(|e| panic!("{} {}", e.code, e.message));
    assert!(m.tolerance_mm >= TOLERANT_FLOOR);
    audit_tolerance(&m);
}

#[test]
fn an_operand_vertex_on_the_other_surface_is_refused_not_guessed() {
    // The sphere's +y vertex lies exactly on the cylinder wall
    // (2.4² + 1.8² = 9): the intersection passes through a vertex.
    let c = cylinder(3., 10.).unwrap();
    let s = translate(&sphere(1.5).unwrap(), [-2.4, 0.3, 3.]);
    let err = boolean(&c, &s, "difference").unwrap_err();
    assert_eq!(err.code, "BREP_UNSUPPORTED_OPERATION");
}

#[test]
fn degenerate_contacts_are_refused_explicitly() {
    let c = cylinder(3., 10.).unwrap();
    // A sphere through the cap rim: every rim point lies on the sphere, so
    // the intersection runs along an existing edge.
    let rim = translate(&sphere(13f64.sqrt()).unwrap(), [0., 0., 8.]);
    let err = boolean(&c, &rim, "union").unwrap_err();
    assert!(!err.message.is_empty());
    // xor has no tolerant fallback.
    let s = translate(&sphere(2.).unwrap(), [2.5, 0.4, 5.]);
    assert!(boolean(&c, &s, "xor").is_err());
    // A torus at the origin meets the box exactly along its seams.
    let b = cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let err = boolean(&b, &torus(4., 1.).unwrap(), "union").unwrap_err();
    assert!(err.message.contains("coincident"), "{}", err.message);
}

#[test]
fn many_placements_never_panic_and_always_validate_or_refuse() {
    // A sweep of sphere positions through the cylinder wall, cap and rim
    // zones: every outcome is either a valid tolerant/exact solid or an
    // explicit refusal, never a panic or an invalid model.
    let c = cylinder(3., 10.).unwrap();
    let mut solved = 0;
    for i in 0..6 {
        for j in 0..4 {
            let x = 0.5 + i as f64 * 0.9;
            let z = 1. + j as f64 * 2.9;
            let s = translate(&sphere(1.7).unwrap(), [x, 0.35, z]);
            for op in ["union", "difference", "intersection"] {
                match boolean(&c, &s, op) {
                    Ok(model) => {
                        model.validate().unwrap();
                        if model.tolerance_mm >= TOLERANT_FLOOR {
                            audit_tolerance(&model);
                        }
                        solved += 1;
                    }
                    Err(e) => assert!(
                        matches!(
                            e.code,
                            "BREP_UNSUPPORTED_OPERATION" | "BREP_ANALYTIC_BOOLEAN_REFUSED"
                        ),
                        "{op} at ({x}, {z}): {} {}",
                        e.code,
                        e.message
                    ),
                }
            }
        }
    }
    assert!(
        solved >= 40,
        "most generic placements must resolve, solved {solved}"
    );
}
