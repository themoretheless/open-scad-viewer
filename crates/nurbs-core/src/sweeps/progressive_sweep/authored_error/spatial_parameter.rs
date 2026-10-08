//! Parameter stations compare whole original Cartesian images with the
//! actual retained section hull. Closed Bishop correction still uses the
//! original arc-length phase; sampled chord correction is never assumed exact.
use super::*;

pub(super) fn section_interpolation(
    sweep: &Sweep<'_>,
    count: usize,
    steps: usize,
    max_cells: usize,
    shared: Option<&super::super::rmf_transport::OriginalRmfTransportReport>,
) -> Result<SectionInterpolationReport> {
    let mut out = SectionInterpolationReport {
        status: Status::Unresolved,
        cells: 0,
        error_upper: None,
        endpoint_displacement_upper: None,
        cap_endpoint_displacement_upper: None,
        reason: Some("parameter-spatial-rmf-image-unproved"),
    };
    if max_cells == 0 {
        return Ok(out);
    }
    let owned;
    let transport = if let Some(shared) = shared {
        shared
    } else {
        let closed = path_is_closed(sweep.path)?;
        owned = super::super::rmf_transport::certify_original_rmf_transport_shared(
            sweep.path,
            sweep.options.normal,
            steps,
            max_cells,
            closed,
        )?;
        out.cells += owned.cells + owned.exact_work as usize;
        &owned
    };
    if transport.status != Status::Certified {
        out.reason = transport.reason;
        return Ok(out);
    }
    let initial = seed_initial_coordinates(
        sweep,
        max_cells - out.cells,
        Orientation::RotationMinimizing,
    )?;
    out.cells += initial.cells;
    let Some(qs) = initial
        .coordinates
        .filter(|_| initial.status == Status::Certified)
    else {
        out.reason = initial.reason;
        return Ok(out);
    };
    let (sections, _, _, _) = sweep.sections_with_frame_identity(count)?;
    if sections.iter().any(|c| {
        c.degree != sweep.profile.degree
            || c.knots != sweep.profile.knots
            || c.weights != sweep.profile.weights
            || c.periodic != sweep.profile.periodic
            || c.control_points.len() != qs.len()
    }) {
        out.reason = Some("retained-section-basis-correspondence-unproved");
        return Ok(out);
    }
    // These outward ratios contain both the mathematical station and the
    // binary64 fraction stored by the constructor. Domain mapping is enclosed
    // independently by the original-coefficient value certificate.
    let station = |i: usize| -> Result<[f64; 2]> {
        let q = I::point(i as f64).div(I::point((count - 1) as f64))?;
        Ok([q.lo.max(0.), q.hi.min(1.)])
    };
    let mut error = 0_f64;
    let mut endpoint = 0_f64;
    let mut caps = [0_f64; 2];
    for i in 0..count {
        let at = station(i)?;
        let source = vector_certificate::certify_values_traversal(
            sweep.path,
            at,
            max_cells - out.cells,
            false,
        )?;
        out.cells += source.cells;
        let Some(source) = source.value else {
            return Ok(out);
        };
        let relative = super::super::rmf_transport::relative_values(
            &transport,
            sweep.scale,
            sweep.twist,
            sweep.affine_laws,
            &qs,
            at,
            at,
            max_cells - out.cells,
        )?;
        out.cells += relative.cells;
        let Some(values) = relative
            .values
            .filter(|_| relative.status == Status::Certified)
        else {
            out.reason = relative.reason;
            return Ok(out);
        };
        let mut displacement = 0_f64;
        for (j, value) in values.iter().enumerate() {
            let mut delta = [I::point(0.); 3];
            for k in 0..3 {
                delta[k] = I::new(source[k][0], source[k][1])?
                    .add(I::new(value[k][0], value[k][1])?)?
                    .sub(I::point(sections[i].control_points[j][k]))?;
            }
            displacement = displacement.max(crate::numerics::interval_vec3::norm(delta)?.hi);
        }
        endpoint = endpoint.max(displacement);
        if i == 0 {
            caps[0] = displacement;
        }
        if i + 1 == count {
            caps[1] = displacement;
        }
        if i + 1 == count {
            continue;
        }
        let interval = [at[0], station(i + 1)?[1]];
        let source = vector_certificate::certify_values_traversal(
            sweep.path,
            interval,
            max_cells - out.cells,
            false,
        )?;
        out.cells += source.cells;
        let Some(source) = source.value else {
            return Ok(out);
        };
        let relative = super::super::rmf_transport::relative_values(
            &transport,
            sweep.scale,
            sweep.twist,
            sweep.affine_laws,
            &qs,
            interval,
            interval,
            max_cells - out.cells,
        )?;
        out.cells += relative.cells;
        let Some(values) = relative
            .values
            .filter(|_| relative.status == Status::Certified)
        else {
            out.reason = relative.reason;
            return Ok(out);
        };
        for (j, value) in values.iter().enumerate() {
            let mut delta = [I::point(0.); 3];
            for k in 0..3 {
                let a = sections[i].control_points[j][k];
                let b = sections[i + 1].control_points[j][k];
                delta[k] = I::new(source[k][0], source[k][1])?
                    .add(I::new(value[k][0], value[k][1])?)?
                    .sub(I::new(a.min(b), a.max(b))?)?;
            }
            error = error.max(crate::numerics::interval_vec3::norm(delta)?.hi);
        }
    }
    out.status = Status::Certified;
    out.error_upper = Some(error.max(endpoint));
    out.endpoint_displacement_upper = Some(endpoint);
    out.cap_endpoint_displacement_upper = Some(caps);
    out.reason = None;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arc_spatial_rmf_multiple_contours_share_original_transport_and_total_work() {
        let ring = [
            [1., 0., 0.],
            [0.5, 1., 0.5],
            [-0.5, 1., 1.],
            [-1., 0., 0.],
            [-0.5, -1., 0.25],
            [0.5, -1., -0.5],
        ];
        let mut points = ring.iter().map(|p| p.to_vec()).collect::<Vec<_>>();
        points.extend(ring[..3].iter().map(|p| p.to_vec()));
        let path = Curve {
            degree: 3,
            knots: (0..13).map(|k| k as f64).collect(),
            control_points: points,
            weights: vec![1.; 9],
            periodic: true,
        };
        let start = path.evaluate(path.domain()[0]).unwrap();
        let origin = std::array::from_fn(|k| start.point[k]);
        let normal = std::array::from_fn(|k| start.d1.as_ref().unwrap()[k]);
        let profile = crate::primitives::circle(origin, normal, 0.05).unwrap();
        // Independent authored contours exercise aggregate certificate work;
        // coincident inputs do not claim nesting, embedding or a Solid.
        let profiles = vec![profile; 4];
        let scale = constant_vector_law([1.1, 0., 0.]).unwrap();
        let twist = constant_vector_law([0.125, 0., 0.]).unwrap();
        let axes = constant_vector_law([1., 1.25, 0.75]).unwrap();
        let center = constant_vector_law([0.01, -0.02, 0.03]).unwrap();
        let options = Options {
            normal: [1., 0., 0.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::ArcLength {
                tolerance: 0.0001,
                max_cells: 100000,
            },
            initial_sections: 65,
            max_sections: 65,
            max_deviation: 0.25,
        };
        let make = |cells| {
            MultiSweep::new(&profiles, &path, &scale, &twist, options)
                .unwrap()
                .with_affine_laws(&axes, &center)
                .unwrap()
                .with_spatial_rmf_error_limits(4096, cells, 1000000)
                .unwrap()
        };
        let checked = make(100000).preview_at(65).unwrap();
        assert!(
            checked.report.accepted && checked.report.continuous_bound,
            "{:?}",
            checked.report
        );
        assert_eq!(checked.profile_patch_ranges.len(), 4);
        assert!(
            checked.report.error_certificate_cells > 0
                && checked.report.error_certificate_cells <= 100000
        );
        let short = make(checked.report.error_certificate_cells - 1)
            .preview_at(65)
            .unwrap();
        assert!(!short.report.accepted && !short.report.continuous_bound);
        assert!(short.report.continuous_error_upper.is_none());
        assert!(short.report.error_certificate_cells <= checked.report.error_certificate_cells - 1);
        let zero = make(0).preview_at(65).unwrap();
        assert!(!zero.report.accepted && !zero.report.continuous_bound);
        eprintln!(
            "shared arc spatial contours={} cells={} upper={:?}",
            profiles.len(),
            checked.report.error_certificate_cells,
            checked.report.continuous_error_upper
        );
    }
    #[test]
    fn closed_parameter_spatial_rmf_charges_holonomy_laws_and_copied_seam() {
        let vertices: [[f64; 3]; 4] = [[1., 0., 0.], [0., 1., 1.], [-1., 0., 0.], [0., -1., 0.5]];
        let tangents: [[f64; 3]; 4] = [
            [0., 0.25, 0.25],
            [-0.25, 0., -0.25],
            [0., -0.25, 0.25],
            [0.25, 0.125, -0.25],
        ];
        let mut poles = vec![vertices[0].to_vec()];
        for i in 0..4 {
            let j = (i + 1) % 4;
            poles.push((0..3).map(|k| vertices[i][k] + tangents[i][k]).collect());
            poles.push((0..3).map(|k| vertices[j][k] - tangents[j][k]).collect());
            poles.push(vertices[j].to_vec());
        }
        let path = Curve {
            degree: 3,
            knots: vec![
                0., 0., 0., 0., 0.25, 0.25, 0.25, 0.5, 0.5, 0.5, 0.75, 0.75, 0.75, 1., 1., 1., 1.,
            ],
            control_points: poles,
            weights: vec![1.; 13],
            periodic: false,
        };
        let profile = crate::primitives::line([1.1, 0., 0.], [1.2, 0., 0.]).unwrap();
        let scale = Curve {
            degree: 2,
            knots: vec![23., 23., 23., 29., 29., 29.],
            control_points: vec![vec![1.1, 0., 0.], vec![1.2, 0., 0.], vec![1.1, 0., 0.]],
            weights: vec![1., 2., 1.],
            periodic: false,
        };
        let twist = constant_vector_law([0.125, 0., 0.]).unwrap();
        let axes = constant_vector_law([1., 1.25, 0.75]).unwrap();
        let center = constant_vector_law([0.01, -0.02, 0.03]).unwrap();
        let options = Options {
            normal: [1., 0., 0.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::Parameter,
            initial_sections: 129,
            max_sections: 129,
            max_deviation: 0.25,
        };
        let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
            .unwrap()
            .with_affine_laws(&axes, &center)
            .unwrap()
            .with_spatial_rmf_error_limits(4096, 100000, 1000000)
            .unwrap();
        let proof = sweep
            .rmf_spatial_patch_error_bound(129, 4096, 100000, 1000000)
            .unwrap();
        assert_eq!(proof.status, Status::Certified, "{proof:?}");
        assert!(proof.within_budget);
        let upper = proof.error_upper.unwrap();
        assert!(upper > 0. && upper <= 0.25);
        let level = sweep.preview_at(129).unwrap();
        assert!(level.report.continuous_bound && level.report.accepted);
        assert!(level.report.closed_path);
        assert_eq!(level.report.continuous_error_upper, Some(upper));
        let profiles = vec![profile.clone(), profile.clone()];
        let multi = MultiSweep::new(&profiles, &path, &scale, &twist, options)
            .unwrap()
            .with_affine_laws(&axes, &center)
            .unwrap()
            .with_spatial_rmf_error_limits(4096, 100000, 1000000)
            .unwrap()
            .preview_at(129)
            .unwrap();
        assert!(multi.report.accepted && multi.report.continuous_bound);
        assert!(
            multi.report.error_certificate_cells < 2 * proof.cells,
            "Original shared frame must be charged once, not independently per profile"
        );
        let limited = MultiSweep::new(&profiles, &path, &scale, &twist, options)
            .unwrap()
            .with_affine_laws(&axes, &center)
            .unwrap()
            .with_spatial_rmf_error_limits(4096, multi.report.error_certificate_cells - 1, 1000000)
            .unwrap()
            .preview_at(129)
            .unwrap();
        assert!(!limited.report.accepted && !limited.report.continuous_bound);
        assert!(limited.report.continuous_error_upper.is_none());
        assert!(limited.report.error_certificate_cells <= multi.report.error_certificate_cells - 1);
        let short = sweep
            .rmf_spatial_patch_error_bound(129, 4096, proof.cells - 1, 1000000)
            .unwrap();
        assert_eq!(short.status, Status::Unresolved);
        assert!(short.error_upper.is_none() && short.patches.is_none());
        let zero = sweep
            .rmf_spatial_patch_error_bound(129, 4096, 0, 1000000)
            .unwrap();
        assert_eq!(zero.status, Status::Unresolved);
        assert!(zero.error_upper.is_none());
        let mut damaged = path.clone();
        damaged.control_points[11][0] = damaged.control_points[11][0].next_up();
        let wrong = Sweep::new(&profile, &damaged, &scale, &twist, options)
            .unwrap()
            .with_affine_laws(&axes, &center)
            .unwrap();
        assert_eq!(
            wrong
                .rmf_spatial_patch_error_bound(129, 4096, 100000, 1000000)
                .unwrap()
                .status,
            Status::Unresolved
        );
        let request = value_codec::json!({"op":"surface_progressive_sweep_spatial_rmf_error","profile":profile,"path":path,"scale":scale,"twist":twist,
            "axis_scale":axes,"center_law":center,"normal":[1.,0.,0.],"orientation":"rmf","spacing":"parameter",
            "initial_sections":129,"max_sections":129,"max_deviation":0.25,"preview_sections":129,"transport_steps":4096,"maxCells":100000,"maxProducts":1000000});
        let public = crate::transport::dispatch(request).unwrap();
        assert_eq!(public["continuousBound"], value_codec::json!(true));
        assert_eq!(public["errorUpper"].as_f64(), Some(upper));
        assert_eq!(public["solidCertified"], value_codec::json!(false));
        assert_eq!(
            public["scope"],
            value_codec::json!(
                "retained-patches-relative-to-original-spatial-rmf-parameter-transport"
            )
        );
        // Independent Bernstein path, RK4 Bishop frame and fine polygonal
        // arc phase. Numerical comparisons validate, but never admit a body.
        let position = |u: f64| -> V {
            let segment = ((u * 4.).floor() as usize).min(3);
            let t = u * 4. - segment as f64;
            let p = &path.control_points[segment * 3..segment * 3 + 4];
            std::array::from_fn(|k| {
                (1. - t).powi(3) * p[0][k]
                    + 3. * t * (1. - t).powi(2) * p[1][k]
                    + 3. * t * t * (1. - t) * p[2][k]
                    + t.powi(3) * p[3][k]
            })
        };
        let normals = super::super::super::rmf_transport::tests::oracle(&path, 4096);
        let mut cumulative = vec![0.];
        for i in 0..4096 {
            cumulative.push(
                cumulative[i]
                    + norm(sub(
                        position((i + 1) as f64 / 4096.),
                        position(i as f64 / 4096.),
                    )),
            );
        }
        let total = cumulative[4096];
        let end = normals[4096];
        let holonomy = ((end[2] - end[1]) / 2f64.sqrt()).atan2(end[0]);
        assert!(holonomy.abs() > 1e-3);
        for patch in proof.patches.as_ref().unwrap() {
            for step in 0..=8 {
                for q in [0., 0.5, 1.] {
                    let low = patch.knots_v[patch.degree_v];
                    let high = patch.knots_v[patch.control_points[0].len()];
                    let t = low + (high - low) * step as f64 / 8.;
                    let i = ((t * 4096.).floor() as usize).min(4095);
                    let f = t * 4096. - i as f64;
                    let phase = 0.125
                        + holonomy * ((1. - f) * cumulative[i] + f * cumulative[i + 1]) / total;
                    let segment = ((t * 4.).floor() as usize).min(3);
                    let u = t * 4. - segment as f64;
                    let p = &path.control_points[segment * 3..segment * 3 + 4];
                    let velocity: V = std::array::from_fn(|k| {
                        3. * (1. - u).powi(2) * (p[1][k] - p[0][k])
                            + 6. * u * (1. - u) * (p[2][k] - p[1][k])
                            + 3. * u * u * (p[3][k] - p[2][k])
                    });
                    let tangent = velocity.map(|x| x / norm(velocity));
                    let n: V =
                        std::array::from_fn(|k| (1. - f) * normals[i][k] + f * normals[i + 1][k]);
                    let cross = math_core::cross(tangent, n);
                    let dot = math_core::dot(tangent, n);
                    let normal: V = std::array::from_fn(|k| {
                        n[k] * phase.cos()
                            + cross[k] * phase.sin()
                            + tangent[k] * dot * (1. - phase.cos())
                    });
                    let side = math_core::cross(tangent, normal);
                    let scale = (1.1 * (1. - t).powi(2) + 4. * 1.2 * t * (1. - t) + 1.1 * t * t)
                        / (1. + 2. * t * (1. - t));
                    let x = scale * (0.1 + 0.1 * q) + 0.01;
                    let origin = position(t);
                    let expected: V = std::array::from_fn(|k| {
                        origin[k] + x * normal[k] - 0.02 * side[k] + 0.03 * tangent[k]
                    });
                    let actual = patch.evaluate(q, t).unwrap().point;
                    assert!(norm(std::array::from_fn(|k| actual[k] - expected[k])) <= upper);
                }
            }
        }
        eprintln!(
            "parameter closed spatial RMF upper={upper} cells={}",
            proof.cells
        );
    }
}
