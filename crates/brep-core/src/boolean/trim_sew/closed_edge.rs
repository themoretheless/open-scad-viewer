//! A coincident endpoint key can represent a genuine closed NURBS edge.
//! Full original curve authority, exact C0 closure and whole-span nonzero
//! tangents are mandatory. Endpoint proximity cannot create this admission.
use super::*;
pub(super) fn verify(c: &Curve) -> Result<()> {
    if !nurbs_core::curve_partition_agreement::verify_source_closure(c)? {
        return Err(refuse(
            "BREP_SEW_INVALID",
            "Closed sew edge lacks exact original source closure",
        ));
    }
    if c.control_points[0].len() != 3 {
        return Err(refuse(
            "BREP_SEW_INVALID",
            "Closed sew authority must be three-dimensional",
        ));
    }
    let proof = nurbs_core::curve_regularity::inspect(c, 4096)?;
    if !proof.spanwise_regular {
        return Err(refuse(
            "BREP_SEW_INVALID",
            "Closed sew edge regularity remains unproven within 4096 cells",
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn curve() -> Curve {
        Curve {
            degree: 1,
            knots: vec![-1., 0., 1., 2., 3., 4., 5.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 0., 0.],
                vec![1., 1., 0.],
                vec![0., 1., 0.],
                vec![0., 0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: true,
        }
    }
    #[test]
    fn equal_endpoints_do_not_make_a_regular_periodic_edge_degenerate() {
        assert!(verify(&curve()).is_ok());
    }
    #[test]
    fn constant_or_unproved_periodic_definitions_cannot_authorize_a_closed_sew_key() {
        let mut c = curve();
        c.control_points.fill(vec![0.; 3]);
        assert!(verify(&c).is_err());
        let mut c = curve();
        c.knots[0] = (-1f64).next_up();
        c.validate().unwrap();
        assert!(verify(&c).is_err());
        let mut c = curve();
        c.periodic = false;
        assert!(verify(&c).is_ok());
        let mut c = curve();
        for p in &mut c.control_points {
            p.truncate(2);
        }
        assert!(verify(&c).is_err());
    }
}
#[cfg(test)]
mod replay_tests {
    use super::*;
    #[test]
    fn closed_correspondence_replays_actual_curve_clock_and_endpoint_binding() {
        let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let context = model.tolerance_context().unwrap();
        let c = Curve {
            degree: 1,
            knots: vec![-1., 0., 1., 2., 3., 4., 5.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 0., 0.],
                vec![1., 1., 0.],
                vec![0., 1., 0.],
                vec![0., 0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: true,
        };
        let proof = prove_boundary_correspondence(
            &context,
            BoundaryCorrespondenceInput {
                curve_a: &c,
                curve_b: &c,
                endpoints_a: [[0.; 3]; 2],
                endpoints_b: [[0.; 3]; 2],
                orientation: ParameterOrientation::Reversed,
                seam_shift: 0,
                shell: 0,
                uses: [
                    BoundaryUse {
                        face: 0,
                        wire: 0,
                        cyclic_index: 0,
                        reversed: false,
                    },
                    BoundaryUse {
                        face: 1,
                        wire: 1,
                        cyclic_index: 0,
                        reversed: true,
                    },
                ],
            },
        )
        .unwrap();
        let key = SewEdgeKey {
            a: [0; 3],
            b: [0; 3],
            mid: None,
            definition: Some(authority_key(&proof.authority)),
            shell: Some(0),
        };
        let entry = CorrespondenceLedgerEntry {
            key,
            correspondence: proof,
        };
        assert!(
            exact_sew_correspondences(&SewSnapshot::default(), &context, &[entry.clone()])
                .unwrap()
                .1
                .complete
        );
        let mut bad = entry.clone();
        bad.correspondence.parameter_domain_bits = [1f64.to_bits(), 5f64.to_bits()];
        assert!(exact_sew_correspondences(&SewSnapshot::default(), &context, &[bad]).is_err());
        let mut bad = entry;
        bad.correspondence.endpoints_bits =
            [[[100f64.to_bits(), 0f64.to_bits(), 0f64.to_bits()]; 2]; 2];
        assert!(exact_sew_correspondences(&SewSnapshot::default(), &context, &[bad]).is_err());
    }
}

#[cfg(test)]
mod ordinary_tests {
    use super::*;
    #[test]
    fn closed_clamped_storage_needs_current_nonconstant_curve_authority() {
        let c = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 2., 3., 4., 4.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 0., 0.],
                vec![1., 1., 0.],
                vec![0., 1., 0.],
                vec![0., 0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: false,
        };
        assert!(verify(&c).is_ok());
        let mut bad = c.clone();
        bad.control_points.fill(vec![0.; 3]);
        assert!(verify(&bad).is_err());
        let mut bad = c;
        bad.control_points.last_mut().unwrap()[0] = f64::from_bits(1);
        assert!(verify(&bad).is_err());
    }
}
