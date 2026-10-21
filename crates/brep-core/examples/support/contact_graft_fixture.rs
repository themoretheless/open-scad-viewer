use brep_core::{Model, contact_bind, contact_end, contact_graft, fillet_contact};
use nurbs_core::{offset::fillet_surfaces::FilletOptions, surface::Surface};
fn plane(u: [f64; 3], v: [f64; 3]) -> Surface {
    let p = |a: f64, b: f64| (0..3).map(|k| u[k] * a + v[k] * b).collect::<Vec<_>>();
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![vec![p(0., 0.), p(0., 10.)], vec![p(10., 0.), p(10., 10.)]],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn cutter(y: f64) -> (Model, usize) {
    let m = brep_core::cuboid([-5., y, -5.], [15., y + 0.5, 15.]).unwrap();
    let f = m
        .faces
        .iter()
        .position(|f| f.surface.control_points.iter().flatten().all(|p| p[1] == y))
        .unwrap();
    (m, f)
}
fn proposals(
    source: &Model,
    contact: &fillet_contact::Contact,
    ys: [f64; 2],
    side: usize,
) -> [(usize, f64); 2] {
    let d = contact.curve.domain();
    let point = contact.curve.evaluate((d[0] + d[1]) * 0.5).unwrap().point;
    std::array::from_fn(|k| {
        let edge = source
            .edges
            .iter()
            .position(|e| {
                let a = source.vertices[e.vertices[0]].point;
                let b = source.vertices[e.vertices[1]].point;
                a[1] == ys[k]
                    && b[1] == ys[k]
                    && a[if side == 0 { 2 } else { 0 }] == 0.
                    && b[if side == 0 { 2 } else { 0 }] == 0.
                    && a[if side == 0 { 0 } else { 2 }] != b[if side == 0 { 0 } else { 2 }]
            })
            .unwrap();
        let axis = if side == 0 { 0 } else { 2 };
        let e = &source.edges[edge];
        let a = source.vertices[e.vertices[0]].point[axis];
        let b = source.vertices[e.vertices[1]].point[axis];
        let [lo, hi] = e.curve.domain();
        (edge, lo + (hi - lo) * (point[axis] - a) / (b - a))
    })
}
fn source_face(source: &Model, axis: usize, value: f64) -> usize {
    source
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[axis] == value)
        })
        .unwrap()
}
fn vertex(source: &Model, point: [f64; 3]) -> usize {
    source
        .vertices
        .iter()
        .position(|v| v.point == point)
        .unwrap()
}
pub struct Fixture {
    pub source: Model,
    pub patch: Model,
    pub mapping: [usize; 4],
    pub targets: [contact_graft::Target; 4],
    pub law_domain: [f64; 2],
    pub radius_field: [f64; 4],
}
pub fn build() -> Fixture {
    build_with_binding(false, false, false)
}
pub fn build_automatic() -> Fixture {
    build_with_binding(true, false, false)
}
pub fn build_variable_automatic() -> Fixture {
    build_with_binding(true, true, false)
}
pub fn build_rolling_automatic() -> Fixture {
    build_with_binding(true, false, true)
}
pub fn build_variable_rolling_automatic() -> Fixture {
    build_with_binding(true, true, true)
}
fn build_with_binding(automatic: bool, variable: bool, circular: bool) -> Fixture {
    let a = plane([1., 0., 0.], [0., 1., 0.]);
    let b = plane([0., 1., 0.], [0., 0., 1.]);
    let options = FilletOptions {
        max_sections: 5,
        ..Default::default()
    };
    let built = if circular && variable {
        fillet_contact::build_linear_at_stations(
            [&a, &b],
            1,
            [2., 8.],
            [0.7, 1.3],
            &options,
            1e-6,
            100000,
            1000000,
        )
        .map(|(report, _)| report)
    } else if circular {
        fillet_contact::build(
            [&a, &b],
            if variable {
                nurbs_core::offset::fillet_surfaces::RadiusLaw::Linear {
                    start: 0.5,
                    end: 1.5,
                }
            } else {
                nurbs_core::offset::fillet_surfaces::RadiusLaw::Constant(1.5)
            },
            &options,
            1e-6,
            100000,
            1000000,
        )
    } else if variable {
        fillet_contact::build_variable_g2(
            [&a, &b],
            nurbs_core::offset::fillet_surfaces::RadiusLaw::Linear {
                start: 0.5,
                end: 1.5,
            },
            &options,
            1e-6,
            100000,
            1000000,
        )
    } else {
        fillet_contact::build_g2([&a, &b], 1.5, &options, 1e-6, 100000, 1000000)
    }
    .unwrap();
    let contacts = built.contacts.as_ref().unwrap();
    let skin = built.fillet.surface.as_ref().unwrap();
    let d = contacts[0].curve.domain();
    let radius_field = if variable && circular {
        [0., (1.3 - 0.7) / 6., 0., 0.7 - 2. * ((1.3 - 0.7) / 6.)]
    } else if variable {
        let y0 = contacts[0].curve.evaluate(d[0]).unwrap().point[1];
        let y1 = contacts[0].curve.evaluate(d[1]).unwrap().point[1];
        let slope = 1. / (y1 - y0);
        [0., slope, 0., 0.5 - slope * y0]
    } else {
        [0., 0., 0., 1.5]
    };
    let up = contacts[0].curve.evaluate(d[1]).unwrap().point[1]
        > contacts[0].curve.evaluate(d[0]).unwrap().point[1];
    let ys = if up { [2., 8.] } else { [8., 2.] };
    let (first, f) = cutter(ys[0]);
    let (last, l) = cutter(ys[1]);
    let body = brep_core::cuboid([0., 2., 0.], [10., 8., 10.]).unwrap();
    let bind = |source: &Model, side: usize| {
        let supports = [if side == 0 { &a } else { &b }, skin];
        let cutters = [(&first, f, 0), (&last, l, 0)];
        if automatic {
            contact_bind::automatically_between_faces(
                source,
                &contacts[side],
                supports,
                cutters,
                1e-8,
                100000,
                1000000,
            )
        } else {
            contact_bind::between_faces(
                source,
                &contacts[side],
                supports,
                cutters,
                proposals(source, &contacts[side], ys, side),
                1e-8,
                100000,
                1000000,
            )
        }
        .unwrap()
    };
    let b0 = bind(&body, 0);
    let b1 = bind(&b0.source, 1);
    let patch = contact_end::patch(
        [&b0.restriction, &b1.restriction],
        skin,
        [(&first, f), (&last, l)],
        b1.source.tolerance_mm,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    let source = &b1.source;
    let mapping = [
        b0.vertices[0],
        b1.vertices[0],
        b1.vertices[1],
        b0.vertices[1],
    ];
    let targets = [
        contact_graft::Target {
            face: source_face(source, 1, ys[0]),
            keep_vertex: vertex(source, [10., ys[0], 10.]),
        },
        contact_graft::Target {
            face: source_face(source, 0, 0.),
            keep_vertex: vertex(source, [0., ys[0], 10.]),
        },
        contact_graft::Target {
            face: source_face(source, 1, ys[1]),
            keep_vertex: vertex(source, [10., ys[1], 10.]),
        },
        contact_graft::Target {
            face: source_face(source, 2, 0.),
            keep_vertex: vertex(source, [10., ys[0], 0.]),
        },
    ];
    Fixture {
        source: source.clone(),
        patch: patch.model,
        mapping,
        targets,
        law_domain: d,
        radius_field,
    }
}
