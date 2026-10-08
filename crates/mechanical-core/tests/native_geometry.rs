use mechanical_core::{
    GearOptions, PlanetaryOptions, ThreadOptions, gear_profile, planetary_profiles, thread_mesh,
    thread_radius_at,
};
use std::collections::BTreeMap;

#[test]
fn profiles_keep_hole_winding_and_planetary_phase() {
    let gear = gear_profile(&GearOptions {
        bore: 4.,
        ..Default::default()
    })
    .unwrap();
    let area = |ring: &[[f64; 2]]| {
        ring.iter()
            .enumerate()
            .map(|(i, p)| {
                let q = ring[(i + 1) % ring.len()];
                p[0] * q[1] - p[1] * q[0]
            })
            .sum::<f64>()
            / 2.
    };
    assert!(area(&gear.loops[0]) > 0.);
    assert!(area(&gear.loops[1]) < 0.);
    let set = planetary_profiles(&PlanetaryOptions {
        gear: GearOptions::default(),
        sun_teeth: 24,
        planet_teeth: 24,
        planet_count: 3,
        carrier_angle: 10.,
    })
    .unwrap();
    assert_eq!(set.parts.len(), 5);
    assert_eq!(set.ring_teeth, 72);
    assert_eq!(set.sun_to_carrier_ratio, 4.);
    assert_eq!(set.sun_angle_deg, 40.);
    assert_eq!(set.parts[0].rotation_deg, 40.);
    for planet in set.parts.iter().filter(|p| p.role == "planet") {
        assert!((planet.origin[0].hypot(planet.origin[1]) - 24.).abs() < 1e-9);
    }
    assert!(
        gear_profile(&GearOptions {
            module: f64::NAN,
            ..Default::default()
        })
        .is_err()
    );
    assert!(
        planetary_profiles(&PlanetaryOptions {
            gear: GearOptions::default(),
            sun_teeth: usize::MAX,
            planet_teeth: usize::MAX,
            planet_count: 3,
            carrier_angle: 0.,
        })
        .is_err()
    );
}

#[test]
fn thread_mesh_has_closed_oriented_edges_and_multistart_lead() {
    for internal in [false, true] {
        for left_handed in [false, true] {
            let options = ThreadOptions {
                internal,
                left_handed,
                starts: 2,
                ..Default::default()
            };
            let mesh = thread_mesh(&options).unwrap();
            assert!(mesh.indices.len() / 3 <= 3500);
            assert!(mesh.positions.iter().flatten().all(|x| x.is_finite()));
            let mut edges = BTreeMap::<_, Vec<_>>::new();
            for t in mesh.indices.as_chunks::<3>().0 {
                for i in 0..3 {
                    let (a, b) = (t[i], t[(i + 1) % 3]);
                    assert!(a < mesh.positions.len() && b < mesh.positions.len());
                    edges.entry((a.min(b), a.max(b))).or_default().push(a > b);
                }
            }
            assert!(
                edges
                    .values()
                    .all(|uses| uses.len() == 2 && uses[0] != uses[1])
            );
            let sign = if left_handed { -1. } else { 1. };
            let a = thread_radius_at(&options, 0.3, 0.4).unwrap();
            let b = thread_radius_at(
                &options,
                0.3 + 1.,
                0.4 + sign * options.starts as f64 * options.pitch / std::f64::consts::TAU,
            )
            .unwrap();
            assert!((a - b).abs() < 1e-12);
        }
    }
    assert!(
        thread_mesh(&ThreadOptions {
            pitch: 0.,
            ..Default::default()
        })
        .is_err()
    );
}
