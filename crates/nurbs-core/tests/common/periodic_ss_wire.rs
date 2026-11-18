use nurbs_core::{ss_intersection as ss, surface::Surface};
#[path = "periodic_extrusion.rs"]
mod fixture;
pub fn wire() -> ss::OriginalSsRootWire {
    wire_at_height(1.)
}

pub fn wire_at_height(height: f64) -> ss::OriginalSsRootWire {
    let section = section_at_height(height);
    let wire = section.to_root_wire(Default::default(), 64).unwrap();
    assert_eq!(wire.edges().last().unwrap().end().phase(), [1, 0, 0, 0]);
    wire
}

pub fn section_at_height(height: f64) -> ss::QualifiedPeriodicPlaneSS {
    let plane = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![-2., -2., height], vec![-2., 2., height]],
            vec![vec![2., -2., height], vec![2., 2., height]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let source = ss::FiberSources::new(&fixture::source(1), &plane).unwrap();
    ss::qualify_periodic_plane_ss(&source, Default::default()).unwrap()
}
