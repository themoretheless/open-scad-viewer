    use super::*;

    #[test]
    fn min_wall_mm_is_rotation_invariant_for_tilted_thin_strips() {
        // A 20 mm x 1.2 mm thin strip rotated by 37 degrees
        let angle = 37.0_f64.to_radians();
        let (s, c) = angle.sin_cos();
        let rot = |x: f64, y: f64| [x * c - y * s, x * s + y * c];
        let rect = vec![
            rot(0.0, 0.0),
            rot(20.0, 0.0),
            rot(20.0, 1.2),
            rot(0.0, 1.2),
        ];
        let sec = LayerSection {
            z_mm: 0.0,
            contours: vec![rect],
        };
        let wall = min_wall_mm(&sec);
        assert!((wall - 1.2).abs() < 1e-4, "expected 1.2 mm, got {wall}");
    }

    #[test]
    fn wall_thickness_3d_detects_thin_box_walls() {
        // Closed box: 10 x 10 x 1.5 mm
        let positions = vec![
            0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 10.0, 10.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 1.5, 10.0,
            0.0, 1.5, 10.0, 10.0, 1.5, 0.0, 10.0, 1.5,
        ];
        let indices = vec![
            0, 2, 1, 0, 3, 2, // bottom (-Z)
            4, 5, 6, 4, 6, 7, // top (+Z)
            0, 1, 5, 0, 5, 4, // front (-Y)
            2, 3, 7, 2, 7, 6, // back (+Y)
            0, 4, 7, 0, 7, 3, // left (-X)
            1, 2, 6, 1, 6, 5, // right (+X)
        ];
        let rep = analyze_wall_thickness_3d(&positions, &indices, 2.0).unwrap();
        assert!((rep.min_thickness_mm - 1.5).abs() < 1e-6);
        assert!(!rep.thin_triangle_indices.is_empty());
    }

    #[test]
    fn euler_buckling_rayleigh_frequency_and_fatigue() {
        let (p_cr, fos) = euler_buckling_safety_factor(200_000.0, 1000.0, 500.0, 1.0, 2000.0).unwrap();
        assert!(p_cr > 7800.0 && p_cr < 8000.0);
        assert!(fos > 3.9);

        // Single DOF 1 mm static deflection under gravity -> f = (1/2pi)*sqrt(9.80665 / 0.001) ≈ 15.76 Hz
        let f1 = estimate_fundamental_frequency_hz(&[1.0], &[1.0]).unwrap();
        assert!((f1 - 15.756).abs() < 0.05);

        let n_goodman = fatigue_safety_factor(50.0, 150.0, 200.0, 350.0, 500.0, FatigueCriterion::Goodman).unwrap();
        let n_soderberg = fatigue_safety_factor(50.0, 150.0, 200.0, 350.0, 500.0, FatigueCriterion::Soderberg).unwrap();
        let n_gerber = fatigue_safety_factor(50.0, 150.0, 200.0, 350.0, 500.0, FatigueCriterion::Gerber).unwrap();
        assert!(n_soderberg < n_goodman && n_goodman < n_gerber);
        assert!((n_goodman - 2.2222).abs() < 1e-3);

        // M8 x 1.25 class 8.8 bolt (proof strength 600 MPa) with 12 kN preload and 5 kN external load
        let bolt = bolted_joint_analysis(8.0, 1.25, 12_000.0, 5_000.0, 600.0).unwrap();
        assert!((bolt.tensile_stress_area_mm2 - 36.6).abs() < 0.1);
        assert!(bolt.proof_safety_factor > 1.5);
        assert!(bolt.joint_separation_safety_factor > 2.5);
        assert!((bolt.recommended_tightening_torque_nm - 19.2).abs() < 0.5);
    }
