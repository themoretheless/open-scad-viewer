//! Closed guided-frame jets in the ORIGINAL normalized arc station.
//! Periodic unit tangent jets replace raw source speed matching. Both source
//! positions, their regular arc maps and the complete joint frame cover remain
//! mandatory. Nothing here grants retained seams or boundary/solid credit.
use super::super::ideal_volume::{arc::ArcMap, arc_frames, charts};
use super::*;

pub(super) fn certify(
    s: &Sweep<'_>,
    guide: &Curve,
    mut out: ClosedPathFrameSmoothnessReport,
    max_cells: usize,
    max_exact_work: u64,
) -> Result<ClosedPathFrameSmoothnessReport> {
    // For a regular normalized arc curve X, X_s=L*T and
    // X_ss=L^2*T_length. The path frame uses T through order k;
    // the guide offset uses its position through order k, hence T through k-1.
    for (curve, order) in [(s.path, out.order), (guide, out.order - 1)] {
        for endpoints in [false, true] {
            let position = super::super::endpoint_jets::certify_position(
                curve,
                endpoints,
                max_exact_work - out.exact_work,
            )?;
            out.exact_work += position.exact_work;
            if !position.certified {
                out.reason = Some("closed-guided-arc-position-jets-unproved");
                return Ok(out);
            }
            let tangent = super::super::endpoint_jets::certify_direction(
                curve,
                order,
                endpoints,
                true,
                max_exact_work - out.exact_work,
            )?;
            out.exact_work += tangent.exact_work;
            if !tangent.certified {
                out.reason = Some("closed-guided-arc-direction-jets-unproved");
                return Ok(out);
            }
        }
    }
    if !knot_continuity(s.twist, out.order, &mut out.exact_work, max_exact_work)? {
        out.reason = Some("closed-guided-frame-knot-jets-unproved");
        return Ok(out);
    }
    let twist =
        super::super::endpoint_jets::certify(s.twist, out.order, max_exact_work - out.exact_work)?;
    out.exact_work += twist.exact_work;
    if !twist.certified {
        out.reason = Some("closed-guided-frame-endpoint-jets-unproved");
        return Ok(out);
    }
    out.reason = Some("closed-guided-arc-source-speed-unproved");
    let Some(path_arc) = ArcMap::build(s.path, &mut out.cells, max_cells)? else {
        return Ok(out);
    };
    // Literal original coefficient identity proves a common normalized arc
    // clock. Failure consumes work and uses independent source arc maps.
    let identity = super::super::arc_guide::certify(s.path, guide, max_cells - out.cells)?;
    out.cells += identity.cells;
    let shared = identity.scale_upper.is_some();
    let guide_arc = if shared {
        None
    } else {
        let Some(map) = ArcMap::build(guide, &mut out.cells, max_cells)? else {
            return Ok(out);
        };
        Some(map)
    };
    let offset = if shared {
        charts::exact_offset(s.path, guide, &mut out.cells, max_cells)?
    } else {
        None
    };
    let zero = constant_vector_law([0.; 3])?;
    let mut pending = vec![[0., 1.]];
    out.reason = Some("closed-guided-arc-joint-frame-cover-unproved");
    while let Some(station) = pending.pop() {
        if out.cells == max_cells {
            return Ok(out);
        }
        out.cells += 1;
        let Some(p) = path_arc.phase(station, &mut out.cells, max_cells)? else {
            return Ok(out);
        };
        let g = if let Some(map) = &guide_arc {
            let Some(g) = map.phase(station, &mut out.cells, max_cells)? else {
                return Ok(out);
            };
            g
        } else {
            p
        };
        let proof = arc_frames::arc_frame(
            s,
            station,
            p,
            Some(g),
            offset.as_ref(),
            None,
            shared,
            &zero,
            max_cells - out.cells,
        )?;
        if let Some(proof) = proof {
            out.cells += proof.cells;
            if proof.status == Status::Certified {
                continue;
            }
        }
        let mid = crate::distance_bounds::midpoint(station[0], station[1]);
        if !(station[0] < mid && mid < station[1]) {
            return Ok(out);
        }
        pending.push([mid, station[1]]);
        pending.push([station[0], mid]);
    }
    out.closed_source_frame_smoothness_certified = true;
    out.reason = None;
    Ok(out)
}

#[cfg(test)]
use crate as nurbs;
#[cfg(test)]
#[path = "../../../../tests/support/arc_bishop_source.rs"]
mod source;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(feature = "transport")]
    fn geometric_guided_arc_transport_recomputes_source_jets_and_preserves_frame_only_scope() {
        let path = source::path(true);
        let mut guide = path.clone();
        for point in &mut guide.control_points {
            point[2] += 0.25;
        }
        let profile = crate::primitives::line([0., 0., 0.01], [0., 0., 0.02]).unwrap();
        let request = value_codec::json!({
            "op":"surface_progressive_sweep_closed_guided_frame_smoothness",
            "profiles":[profile],"path":path,"orientation_guide":guide,
            "scale":constant_vector_law([1.,0.,0.]).unwrap(),
            "twist":constant_vector_law([0.;3]).unwrap(),"normal":[0.,0.,1.],
            "orientation":"rmf","spacing":"arc_length","length_tolerance":0.001,
            "length_max_cells":100000,"initial_sections":5,"max_sections":17,
            "max_deviation":1.,"order":2,"maxCells":100000,"maxExactWork":1000000
        });
        let input = request.clone();
        let proof = crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(request, input);
        assert_eq!(proof["closedSourceFrameSmoothnessCertified"], true);
        assert_eq!(proof["scope"], "closed-original-guided-frame-only");
        for key in [
            "pathSeamCertified",
            "retainedSeamsCertified",
            "profileJoinsCertified",
            "capJoinsCertified",
            "continuousBound",
            "solidCertified",
        ] {
            assert_eq!(proof[key], false, "{key}");
        }
        for (field, value) in [
            ("maxCells", 0),
            ("maxExactWork", 0),
            ("maxCells", proof["cells"].as_u64().unwrap() - 1),
            ("maxExactWork", proof["exactWork"].as_u64().unwrap() - 1),
        ] {
            let mut short = request.clone();
            short[field] = value_codec::json!(value);
            short["closedSourceFrameSmoothnessCertified"] = value_codec::json!(true);
            assert_eq!(
                crate::transport::dispatch(short).unwrap()["closedSourceFrameSmoothnessCertified"],
                false
            );
        }
        let mut raw = request;
        raw["spacing"] = value_codec::json!("parameter");
        assert_eq!(
            crate::transport::dispatch(raw).unwrap()["closedSourceFrameSmoothnessCertified"],
            false
        );
    }
    #[test]
    fn original_arc_guided_seam_keeps_geometric_jets_when_raw_speeds_differ() {
        let path = source::path(true);
        let mut guide = path.clone();
        for point in &mut guide.control_points {
            point[2] += 0.25;
        }
        let profile = crate::primitives::line([0., 0., 0.01], [0., 0., 0.02]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let options = Options {
            normal: [0., 0., 1.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 5,
            max_sections: 17,
            max_deviation: 1.,
        };
        let audit = |p: &Curve, g: &Curve, spacing, order, cells, work| {
            Sweep::new(&profile, p, &scale, &twist, Options { spacing, ..options })
                .unwrap()
                .with_orientation_guide(g)
                .unwrap()
                .certify_closed_guided_frame_smoothness(order, cells, work)
                .unwrap()
        };
        for order in [1, 2] {
            let proof = audit(&path, &guide, options.spacing, order, 100000, 1000000);
            assert!(
                proof.closed_source_frame_smoothness_certified,
                "{order}/{proof:?}"
            );
            assert!(proof.cells > 0 && proof.exact_work > 0);
            eprintln!("geometric guided source frame order={order}: {proof:?}");
            assert!(
                !audit(&path, &guide, Spacing::Parameter, order, 100000, 1000000)
                    .closed_source_frame_smoothness_certified
            );
            for (cells, work) in [
                (0, 1000000),
                (proof.cells - 1, 1000000),
                (100000, 0),
                (100000, proof.exact_work - 1),
            ] {
                let short = audit(&path, &guide, options.spacing, order, cells, work);
                assert!(!short.closed_source_frame_smoothness_certified, "{short:?}");
                assert!(short.cells <= cells && short.exact_work <= work);
            }
            // Equal endpoint positions cannot conceal a geometric tangent or
            // guide second-jet mismatch, even at one stored binary64 ULP.
            let mut corner = path.clone();
            corner.control_points[1][2] = 2_f64.powi(-20);
            assert!(
                !audit(&corner, &guide, options.spacing, order, 100000, 1000000)
                    .closed_source_frame_smoothness_certified
            );
            if order == 2 {
                let mut damaged = guide.clone();
                damaged.control_points[2][2] = damaged.control_points[2][2].next_up();
                assert!(
                    !audit(&path, &damaged, options.spacing, order, 100000, 1000000)
                        .closed_source_frame_smoothness_certified
                );
            }
        }
    }

    #[test]
    fn original_arc_guided_cover_owns_independent_source_domains() {
        let path = source::path(true);
        let mut guide = path.clone();
        guide.knots = [vec![13.; 10], vec![29.; 10]].concat();
        for point in &mut guide.control_points {
            point[2] += 0.25;
        }
        let profile = crate::primitives::line([0., 0., 0.01], [0., 0., 0.02]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let sweep = Sweep::new(
            &profile,
            &path,
            &scale,
            &twist,
            Options {
                normal: [0., 0., 1.],
                orientation: Orientation::RotationMinimizing,
                spacing: Spacing::ArcLength {
                    tolerance: 0.001,
                    max_cells: 100000,
                },
                initial_sections: 5,
                max_sections: 17,
                max_deviation: 1.,
            },
        )
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
        let proof = sweep
            .certify_closed_guided_frame_smoothness(2, 100000, 1000000)
            .unwrap();
        assert!(proof.closed_source_frame_smoothness_certified, "{proof:?}");
        eprintln!("independent original guide domain: {proof:?}");
        assert!(
            !sweep
                .certify_closed_guided_frame_smoothness(2, proof.cells - 1, 1000000)
                .unwrap()
                .closed_source_frame_smoothness_certified
        );
    }

    #[test]
    fn original_guided_arc_joint_cover_can_replace_a_failed_cartesian_product() {
        let path = source::path(false);
        let mut guide = path.clone();
        for point in &mut guide.control_points {
            point[2] += 0.25;
        }
        let profile = crate::primitives::line([0., 0., 0.01], [0., 0., 0.02]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let sweep = Sweep::new(
            &profile,
            &path,
            &scale,
            &twist,
            Options {
                normal: [0., 0., 1.],
                orientation: Orientation::RotationMinimizing,
                spacing: Spacing::ArcLength {
                    tolerance: 0.001,
                    max_cells: 100000,
                },
                initial_sections: 5,
                max_sections: 17,
                max_deviation: 1.,
            },
        )
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
        let raw = super::super::super::endpoint_jets::certify(&path, 3, 1000000).unwrap();
        assert!(raw.certified);
        let (strong, work) =
            frame::certify_independent_guided_cover(&path, &guide, &twist, 4096).unwrap();
        assert_eq!(strong, Status::Unresolved);
        assert_eq!(work, 4096);
        let proof = sweep
            .certify_closed_guided_frame_smoothness(2, 100000, 1000000)
            .unwrap();
        assert!(proof.closed_source_frame_smoothness_certified, "{proof:?}");
        assert!(proof.cells > work && proof.exact_work > raw.exact_work);
        assert!(
            !sweep
                .certify_closed_guided_frame_smoothness(2, proof.cells - 1, 1000000)
                .unwrap()
                .closed_source_frame_smoothness_certified
        );
        eprintln!("failed Cartesian product and original station cover share work: {proof:?}");
    }

    #[test]
    fn original_arc_guided_closed_body_has_global_embedding_and_g2_differentials() {
        let path = source::path(true);
        let mut guide = path.clone();
        for point in &mut guide.control_points {
            point[2] += 0.25;
        }
        let (profiles, cap, _) = source::material();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let sweep = MultiSweep::new(
            &profiles,
            &path,
            &scale,
            &twist,
            Options {
                normal: [0., 0., 1.],
                orientation: Orientation::RotationMinimizing,
                spacing: Spacing::ArcLength {
                    tolerance: 0.001,
                    max_cells: 100000,
                },
                initial_sections: 5,
                max_sections: 65,
                max_deviation: 0.05,
            },
        )
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
        let boundary = sweep
            .certify_original_closed_boundary_smoothness(
                &[4],
                OriginalBoundarySmoothnessLimits {
                    order: 2,
                    cells: 100000,
                    exact_work: 1000000,
                    products: 1000000,
                },
            )
            .unwrap();
        assert!(
            boundary.differential_compatibility_certified
                && boundary.frame_certified
                && boundary.laws_certified
                && boundary.profiles_certified,
            "{boundary:?}"
        );
        let global = sweep
            .certify_ideal_volume_atlas(
                [&cap, &cap],
                [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                1000000,
                1000000,
            )
            .unwrap();
        assert!(
            global.embedding_certified
                && global.regularity_certified
                && global.parameter_topology_preserved,
            "{global:?}"
        );
        eprintln!("original guided geometric-clock global body: {global:?}; boundary={boundary:?}");
    }
}
