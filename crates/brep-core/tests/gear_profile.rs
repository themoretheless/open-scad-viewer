use brep_core::gear::{GearSpec, profile_with_report};
#[test]
fn exact_profile_joins_preserve_flanks_and_include_the_full_wire_seam() {
    for (teeth, internal, bore) in [
        (3, false, 0.),
        (8, false, 0.5),
        (20, false, 0.),
        (40, true, 0.),
    ] {
        let spec = GearSpec {
            teeth,
            internal,
            bore,
            ..GearSpec::default()
        };
        let (wires, r) = profile_with_report(&spec).unwrap();
        assert!(r.profile_join_adjustment_upper > 0. && r.profile_join_adjustment_upper <= 1e-7);
        assert!(r.flank_rounding_certified && r.flank_deviation <= 1e-6);
        assert!(r.arc_radius_rounding_certified);
        assert!(r.transition_rounding_certified, "{r:?}");
        assert!(r.transition_error_upper.unwrap() < 1e-6);
        assert!(r.profile_error_upper.unwrap() <= 1e-6);
        assert!(r.arc_radius_error_upper.unwrap() < 1e-7);
        let pitch = spec.module * spec.teeth as f64 / 2.;
        let (tip, root) = if internal {
            (pitch - spec.module, pitch + spec.module + spec.clearance)
        } else {
            (pitch + spec.module, pitch - spec.module - spec.clearance)
        };
        let targets = [tip, root, spec.bore / 2., root + spec.rim_width];
        for c in wires.iter().flatten().filter(|c| c.degree == 2) {
            let p = &c.control_points[0];
            let initial = p[0].hypot(p[1]);
            let target = *targets
                .iter()
                .min_by(|a, b| (initial - **a).abs().total_cmp(&(initial - **b).abs()))
                .unwrap();
            for i in 0..=20 {
                let p = c.evaluate(i as f64 / 20.).unwrap().point;
                assert!(
                    (p[0].hypot(p[1]) - target).abs() <= r.arc_radius_error_upper.unwrap() + 1e-11
                );
            }
        }
        for wire in &wires {
            for i in 0..wire.len() {
                let next = &wire[(i + 1) % wire.len()];
                let c = &wire[i];
                assert_eq!(c.control_points.last().unwrap(), &next.control_points[0]);
                let a = c.evaluate(c.domain()[1]).unwrap().point;
                let b = next.evaluate(next.domain()[0]).unwrap().point;
                assert!((a[0] - b[0]).hypot(a[1] - b[1]) < 1e-12);
            }
        }
    }
}
#[test]
fn represented_small_gear_profile_has_independent_exact_certificate() {
    for teeth in [3, 8] {
        let (wires, _) = profile_with_report(&GearSpec {
            teeth,
            ..GearSpec::default()
        })
        .unwrap();
        let certificate = brep_core::profile_certificate::certify(&wires, 1e-7).unwrap();
        let verified = brep_core::profile_certificate::verify(&wires, 1e-7, &certificate).unwrap();
        assert_eq!(verified.nesting_depth(), &[0]);
    }
}

#[test]
fn spinner_sun_profile_roundtrips_through_lossless_sector_pattern() {
    use nurbs_core::involute::LosslessSectorPattern;
    let spec = GearSpec { module: 1.5, teeth: 32, clearance: 0.1, herringbone: true,
        helix_angle_deg: -35., ..GearSpec::default() };
    let (wires,_) = profile_with_report(&spec).unwrap();
    let outline = &wires[0];
    let width = outline.len()/spec.teeth;
    let pattern = LosslessSectorPattern::encode(outline,width).unwrap();
    let mut raw = 0;
    for (i,sector) in outline.chunks_exact(width).enumerate() {
        for (a,b) in sector.iter().zip(pattern.sector(i).unwrap()) {
            assert_eq!(a.degree,b.degree);
            assert_eq!(a.periodic,b.periodic);
            let bits = |c: &nurbs_core::curve::Curve| c.control_points.iter().flatten()
                .chain(&c.weights).chain(&c.knots).map(|x| x.to_bits()).collect::<Vec<_>>();
            assert_eq!(bits(a),bits(&b));
            if i > 0 { raw += bits(a).len()*8; }
        }
    }
    eprintln!("sun profile: {} segments, {} seed pieces, {} residual bytes vs {} repeated numeric bytes",
        outline.len(),width,pattern.residual_bytes(),raw);
    assert!(pattern.residual_bytes() < raw);
    assert!(pattern.sector(spec.teeth).is_err());
    assert!(LosslessSectorPattern::encode(outline,0).is_err());
}

#[test]
fn internal_spinner_ring_packs_reverse_sectors_without_reordering_wire() {
    use nurbs_core::involute::LosslessSectorPattern;
    let spec = GearSpec { module: 1.5, teeth: 40, clearance: 0.1, internal: true,
        helix_angle_deg: 35., herringbone: true, ..GearSpec::default() };
    let (wires,_) = profile_with_report(&spec).unwrap();
    let ring = &wires[1];
    let width = ring.len()/spec.teeth;
    let forward = LosslessSectorPattern::encode(ring,width).unwrap();
    let packed = LosslessSectorPattern::encode_bidirectional(ring,width).unwrap();
    for (i, original) in ring.chunks_exact(width).enumerate() {
        for (a,b) in original.iter().zip(packed.sector(i).unwrap()) {
            let bits = |c: &nurbs_core::curve::Curve| c.control_points.iter().flatten().chain(&c.weights)
                .chain(&c.knots).map(|v| v.to_bits()).collect::<Vec<_>>();
            assert_eq!(bits(a),bits(&b));
            assert_eq!((a.degree,a.periodic),(b.degree,b.periodic));
        }
    }
    eprintln!("ring {} segments: forward {} adaptive {} residual bytes", ring.len(),forward.residual_bytes(),packed.residual_bytes());
    assert!(packed.residual_bytes() < forward.residual_bytes());
    assert!(LosslessSectorPattern::encode_bidirectional(ring,0).is_err());
}
