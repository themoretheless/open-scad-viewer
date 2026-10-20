use brep_core::{contact_bind, contact_end, contact_graft, fillet_contact::Contact};
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
    surface_blend::{self, Options, Trace},
};
fn plane(cap: bool) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..2)
            .map(|i| {
                (0..2)
                    .map(|j| {
                        if cap {
                            vec![10. * i as f64, 10., 10. * j as f64]
                        } else {
                            vec![10. * i as f64, 10. * j as f64, 0.]
                        }
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn line(p: [[f64; 2]; 2]) -> Curve {
    Curve::from_polyline(p.map(|p| p.to_vec()).to_vec()).unwrap()
}
pub fn build() -> (brep_core::Model, contact_graft::Graft) {
    let supports = [plane(false), plane(true)];
    let built = surface_blend::g2_traces(
        supports.each_ref(),
        [
            Trace {
                at: 0.99,
                along: [0., 1.],
            },
            Trace {
                at: 0.01,
                along: [0., 1.],
            },
        ],
        Options {
            gains: [0.01; 2],
            ..Default::default()
        },
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    assert!(built.complete && built.within_tolerance);
    assert_eq!(built.construction.patches.len(), 1);
    let skin = &built.construction.patches[0];
    let contacts: [Contact; 2] = std::array::from_fn(|side| Contact {
        curve: skin.iso(Axis::V, side as f64).unwrap(),
        support_pcurve: line([
            [0., if side == 0 { 0.99 } else { 0.01 }],
            [1., if side == 0 { 0.99 } else { 0.01 }],
        ]),
        fillet_pcurve: line([[0., side as f64], [1., side as f64]]),
        positions: built.support_seams[side].2.positions.clone(),
    });
    let cutter = |x: f64| {
        let m = brep_core::cuboid([x, -5., -5.], [x + 0.5, 15., 15.]).unwrap();
        let face = m
            .faces
            .iter()
            .position(|f| f.surface.control_points.iter().flatten().all(|p| p[0] == x))
            .unwrap();
        (m, face)
    };
    let first = cutter(2.);
    let last = cutter(8.);
    let source = brep_core::cuboid([2., 0., 0.], [8., 10., 10.]).unwrap();
    let original = source.clone();
    let bind = |m: &brep_core::Model, side: usize| {
        contact_bind::automatically_between_faces(
            m,
            &contacts[side],
            [&supports[side], skin],
            [(&first.0, first.1, 0), (&last.0, last.1, 0)],
            1e-8,
            100000,
            1000000,
        )
        .unwrap()
    };
    let a = bind(&source, 0);
    let b = bind(&a.source, 1);
    let patch = contact_end::patch(
        [&a.restriction, &b.restriction],
        skin,
        [(&first.0, first.1), (&last.0, last.1)],
        1e-7,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    let mapping = [a.vertices[0], b.vertices[0], b.vertices[1], a.vertices[1]];
    let vertex = |p: [f64; 3]| b.source.vertices.iter().position(|v| v.point == p).unwrap();
    let keeps = [
        vertex([2., 0., 10.]),
        vertex([2., 10., 10.]),
        vertex([8., 0., 10.]),
        vertex([2., 0., 0.]),
    ];
    let out = contact_graft::attach_planned(
        &b.source,
        &patch.model,
        &mapping,
        &keeps,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    assert!(out.trims.all_valid);
    assert_eq!(
        out.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert_eq!(out.model.bodies.len(), 1);
    let seams = brep_core::sweep_smoothness::geometric::inspect_edges(
        &out.model,
        &[out.patch_edges[1], out.patch_edges[3]],
        brep_core::sweep_smoothness::geometric::Tolerances {
            position: 1e-7,
            normal: 1e-5,
            curvature: 1e-4,
        },
        100000 - out.cells,
    )
    .unwrap();
    assert!(seams.complete && seams.within_tolerance);
    for (i, f) in b.source.faces.iter().enumerate() {
        assert_eq!(f.surface, out.model.faces[i].surface);
    }
    assert_eq!(source, original);
    (b.source, out)
}
