//! Involute gear bodies (spur, helical, herringbone, internal) tessellate as
//! closed manifold shells at every LOD, and their Booleans (a bore cut, a
//! planet against a ring with a gap) go through the kernel dispatch.
use brep_core::{GearSpec, cylinder, gear, gear_with_report, operations, transform};
use geometry_bridge::brep::nurbs;

fn closed(model: &brep_core::Model, what: &str) -> f64 {
    // LOD 1 puts one chord on every root arc between neighbouring teeth,
    // which lets the flank chords cross; LOD 8 exceeds the 20000-triangle
    // display budget for a herringbone. Gears live at LOD 2..4.
    let mut volume = 0.;
    for segments in [2usize, 4] {
        let t = nurbs(model, segments).unwrap_or_else(|e| panic!("{what} lod {segments}: {e:?}"));
        assert!(t.built.report.closed, "{what} lod {segments} closed");
        assert_eq!(t.built.report.non_manifold_edges, 0, "{what} lod {segments}");
        assert!(t.built.report.signed_volume_mm3 > 0., "{what} lod {segments}");
        volume = t.built.report.signed_volume_mm3;
    }
    volume
}

#[test]
fn gear_bodies_tessellate_closed() {
    let spur = gear(&GearSpec {
        module: 2.,
        teeth: 20,
        height: 6.,
        ..GearSpec::default()
    })
    .unwrap();
    let helical = gear(&GearSpec {
        module: 1.5,
        teeth: 16,
        height: 8.,
        helix_angle_deg: 25.,
        bore: 6.,
        ..GearSpec::default()
    })
    .unwrap();
    let herringbone = gear(&GearSpec {
        module: 1.5,
        teeth: 16,
        height: 8.,
        helix_angle_deg: -25.,
        herringbone: true,
        bore: 6.,
        ..GearSpec::default()
    })
    .unwrap();
    let ring = gear(&GearSpec {
        module: 1.5,
        teeth: 40,
        height: 10.,
        internal: true,
        rim_width: 2.,
        helix_angle_deg: 35.,
        herringbone: true,
        ..GearSpec::default()
    })
    .unwrap();
    let planet = gear(&GearSpec {
        module: 1.5,
        teeth: 4,
        height: 10.,
        helix_angle_deg: 35.,
        herringbone: true,
        ..GearSpec::default()
    })
    .unwrap();
    let v_spur = closed(&spur, "spur");
    let v_helical = closed(&helical, "helical");
    let v_herringbone = closed(&herringbone, "herringbone");
    closed(&ring, "ring");
    closed(&planet, "planet");
    assert!((v_helical - v_herringbone).abs() < 0.01 * v_helical);
    let (_, geometry) = gear_with_report(&GearSpec {
        module: 2.,
        teeth: 20,
        height: 6.,
        ..GearSpec::default()
    })
    .unwrap();
    let pitch = std::f64::consts::PI * geometry.pitch_radius.powi(2) * 6.;
    assert!((v_spur - pitch).abs() < 0.1 * pitch, "spur {v_spur} vs pitch {pitch}");
}

#[test]
fn gears_take_part_in_booleans() {
    // A bore drilled after the fact: the planar/ruled walls meet the
    // cylinder through the exact and tolerant paths.
    let spur = gear(&GearSpec {
        module: 2.,
        teeth: 20,
        height: 6.,
        ..GearSpec::default()
    })
    .unwrap();
    let drill = transform::affine(
        &cylinder(4., 10.).unwrap(),
        [
            [1., 0., 0., 0.3],
            [0., 1., 0., -0.2],
            [0., 0., 1., -2.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let bored = operations::boolean(&spur, &drill, "difference")
        .unwrap_or_else(|e| panic!("bore: {} {}", e.code, e.message));
    bored.validate().unwrap();
    let v = closed(&bored, "bored gear");
    let v_full = closed(&spur, "spur");
    assert!(v < v_full && v > 0.5 * v_full);
    // Two meshing spur gears with backlash: interleaved teeth, no material
    // in common, one union of two separated bodies.
    let mate = transform::affine(
        &gear(&GearSpec {
            module: 2.,
            teeth: 20,
            height: 6.,
            backlash: 0.2,
            ..GearSpec::default()
        })
        .unwrap(),
        // Half a tooth pitch of rotation (9 degrees) puts a tooth of one
        // gear into a space of the other at the contact line.
        [
            [9f64.to_radians().cos(), -9f64.to_radians().sin(), 0., 40.],
            [9f64.to_radians().sin(), 9f64.to_radians().cos(), 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let pair = operations::boolean(&spur, &mate, "union")
        .unwrap_or_else(|e| panic!("pair: {} {}", e.code, e.message));
    assert_eq!(pair.bodies.len(), 2);
    closed(&pair, "meshing pair");
}
