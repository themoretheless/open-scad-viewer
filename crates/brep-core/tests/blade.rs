use brep_core::blade::{self, Section};
fn station(z: f64, chord: f64, twist: f64) -> Section {
    Section {
        leading_edge: [0., 0., z],
        chord,
        thickness_control: 0.12 * chord,
        twist_radians: twist,
    }
}
fn oracle(s: Section, t: f64, sign: f64) -> [f64; 3] {
    let x = s.chord * (1.95 * t * t - 0.95 * t * t * t);
    let y = sign * s.thickness_control * (1.5 * t - 1.95 * t * t + 0.45 * t * t * t);
    let (sn, cs) = s.twist_radians.sin_cos();
    [
        s.leading_edge[0] + cs * x - sn * y,
        s.leading_edge[1] + sn * x + cs * y,
        s.leading_edge[2],
    ]
}
#[test]
fn authored_profile_matches_independent_cubic_equations() {
    let mut s = station(4., 3., 0.37);
    s.leading_edge[0] = 2.;
    s.leading_edge[1] = -1.;
    let curves = blade::profile(s).unwrap();
    for t in [0., 0.13, 0.42, 0.81, 1.] {
        for (i, sign) in [(0, -1.), (1, 1.)] {
            let p = curves[i]
                .evaluate(if i == 0 { t } else { 1. - t })
                .unwrap()
                .point;
            let q = oracle(s, t, sign);
            for k in 0..3 {
                assert!((p[k] - q[k]).abs() < 5e-14);
            }
        }
    }
    assert_eq!(
        curves[0].control_points.last(),
        curves[1].control_points.first()
    );
    assert_eq!(
        curves[1].control_points.last(),
        curves[0].control_points.first()
    );
}
#[test]
fn twisted_tapered_blade_has_closed_owned_edges_and_actual_ruled_walls() {
    let sections = [
        station(0., 3., 0.),
        station(4., 2.5, 0.12),
        station(9., 1.7, 0.3),
    ];
    let model = blade::build(&sections).unwrap();
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(model.bodies.len(), 1);
    assert_eq!(model.faces.len(), 10);
    assert!(model.shells[0].closed);
    for edge in 0..model.edges.len() {
        assert_eq!(
            model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == edge)
                .count(),
            2
        );
    }
    for j in 0..2 {
        for side in 0..4 {
            let surface = &model.faces[j * 4 + side].surface;
            for (u, v) in [(0.19, 0.27), (0.63, 0.81)] {
                let local = 0.5 * (side % 2) as f64 + 0.5 * u;
                let t = if side < 2 { local } else { 1. - local };
                let sign = if side < 2 { -1. } else { 1. };
                let p = oracle(sections[j], t, sign);
                let q = oracle(sections[j + 1], t, sign);
                let r = surface.evaluate(u, v).unwrap().point;
                for k in 0..3 {
                    assert!((r[k] - ((1. - v) * p[k] + v * q[k])).abs() < 1e-12);
                }
            }
        }
    }
}
#[test]
fn malformed_and_collapsed_blades_are_refused() {
    let s = station(0., 2., 0.);
    assert!(blade::build(&[s]).is_err());
    assert!(blade::build(&[s, s]).is_err());
    assert!(blade::build(&[station(2., 2., 0.), s]).is_err());
    for bad in [
        Section { chord: 0., ..s },
        Section {
            chord: f64::NAN,
            ..s
        },
        Section {
            thickness_control: 3.,
            ..s
        },
        Section {
            thickness_control: -1.,
            ..s
        },
        Section {
            twist_radians: f64::INFINITY,
            ..s
        },
        Section {
            leading_edge: [1e7, 0., 0.],
            ..s
        },
    ] {
        assert!(blade::profile(bad).is_err());
    }
    assert!(blade::build(&vec![s; 130]).is_err());
}
