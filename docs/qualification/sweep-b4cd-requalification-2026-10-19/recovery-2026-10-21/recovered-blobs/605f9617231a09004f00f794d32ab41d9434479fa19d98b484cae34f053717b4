//! Original reference-profile anchor enclosure; no sampled point is a premise.
use super::*;
use crate::distance_bounds::Interval as I;
use crate::numerics::interval_vec3::{dot_tight, sub as subtract};
use crate::sweeps::progressive_miter::{
    authored_frame_certificate, scalar_certificate::Status, vector_certificate,
};

#[derive(Clone, Debug)]
pub struct ContactAnchorReport {
    pub status: Status,
    pub cells: usize,
    pub point: Option<[[f64; 2]; 3]>,
    pub coordinates: Option<[[f64; 2]; 3]>,
    pub reason: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub struct ContactFitReport {
    pub status: Status,
    pub cells: usize,
    pub fit: Option<authored_frame_certificate::GuideWidthReport>,
    pub reason: Option<&'static str>,
}

pub(super) fn control_trajectory(
    sweep: &Sweep<'_>,
    control: usize,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<authored_frame_certificate::TrajectoryReport> {
    check(
        control < sweep.profile.control_points.len(),
        "Contact control outside original profile",
    )?;
    check(
        max_cells <= 100000
            && traversal.iter().all(|t| t.is_finite())
            && traversal[0] >= 0.
            && traversal[0] <= traversal[1]
            && traversal[1] <= 1.,
        "Invalid contact control certificate request",
    )?;
    let unresolved = |cells, reason| authored_frame_certificate::TrajectoryReport {
        traversal,
        status: Status::Unresolved,
        cells,
        jet: None,
        single_span: false,
        reason: Some(reason),
    };
    let Some(guide) = sweep
        .orientation_guide
        .filter(|_| sweep.contact_source.is_some())
    else {
        return Ok(unresolved(0, "mode-not-contact"));
    };
    if sweep.options.spacing != Spacing::Parameter {
        return Ok(unresolved(0, "arc-length-correspondence-unproved"));
    }
    let anchor = certify(sweep, max_cells)?;
    let mut cells = anchor.cells;
    if anchor.status != Status::Certified {
        return Ok(unresolved(cells, anchor.reason.unwrap()));
    }
    let initial = authored_error::guided_initial_coordinates(sweep, max_cells - cells)?;
    cells += initial.cells;
    if initial.status != Status::Certified {
        return Ok(unresolved(cells, initial.reason.unwrap()));
    }
    let mut report = authored_frame_certificate::certify_contact_control_trajectory(
        sweep.path,
        guide,
        sweep.scale,
        sweep.twist,
        sweep.affine_laws,
        initial.coordinates.unwrap()[control],
        anchor.coordinates.unwrap()[0],
        traversal,
        max_cells - cells,
    )?;
    report.cells += cells;
    Ok(report)
}

pub(super) fn control_value(
    sweep: &Sweep<'_>,
    control: usize,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<authored_frame_certificate::ControlValueReport> {
    check(
        control < sweep.profile.control_points.len(),
        "Contact control outside original profile",
    )?;
    check(
        max_cells <= 100000
            && traversal.iter().all(|t| t.is_finite())
            && traversal[0] >= 0.
            && traversal[0] <= traversal[1]
            && traversal[1] <= 1.,
        "Invalid contact control certificate request",
    )?;
    let unresolved = |cells, reason| authored_frame_certificate::ControlValueReport {
        status: Status::Unresolved,
        cells,
        value: None,
        reason: Some(reason),
    };
    let Some(guide) = sweep
        .orientation_guide
        .filter(|_| sweep.contact_source.is_some())
    else {
        return Ok(unresolved(0, "mode-not-contact"));
    };
    if sweep.options.spacing != Spacing::Parameter {
        return Ok(unresolved(0, "arc-length-correspondence-unproved"));
    }
    let anchor = certify(sweep, max_cells)?;
    let mut cells = anchor.cells;
    if anchor.status != Status::Certified {
        return Ok(unresolved(cells, anchor.reason.unwrap()));
    }
    let initial = authored_error::guided_initial_coordinates(sweep, max_cells - cells)?;
    cells += initial.cells;
    if initial.status != Status::Certified {
        return Ok(unresolved(cells, initial.reason.unwrap()));
    }
    let mut report = authored_frame_certificate::certify_contact_control_value(
        sweep.path,
        guide,
        sweep.scale,
        sweep.twist,
        sweep.affine_laws,
        initial.coordinates.unwrap()[control],
        anchor.coordinates.unwrap()[0],
        traversal,
        max_cells - cells,
    )?;
    report.cells += cells;
    Ok(report)
}

pub(super) fn fit(
    sweep: &Sweep<'_>,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<ContactFitReport> {
    check(
        max_cells <= 100000
            && traversal.iter().all(|t| t.is_finite())
            && traversal[0] >= 0.
            && traversal[0] <= traversal[1]
            && traversal[1] <= 1.,
        "Invalid contact fit certificate request",
    )?;
    let mut out = ContactFitReport {
        status: Status::Unresolved,
        cells: 0,
        fit: None,
        reason: Some("mode-not-contact"),
    };
    let Some(guide) = sweep
        .orientation_guide
        .filter(|_| sweep.contact_source.is_some())
    else {
        return Ok(out);
    };
    if sweep.options.spacing != Spacing::Parameter {
        out.reason = Some("arc-length-correspondence-unproved");
        return Ok(out);
    }
    let anchor = certify(sweep, max_cells)?;
    out.cells += anchor.cells;
    out.reason = anchor.reason;
    if anchor.status != Status::Certified {
        return Ok(out);
    }
    out.reason = Some("contact-fit-enclosure-unresolved");
    let mut fitted = authored_frame_certificate::certify_contact_fit(
        sweep.path,
        guide,
        sweep.scale,
        sweep.affine_laws,
        anchor.coordinates.unwrap()[0],
        traversal,
        max_cells - out.cells,
    )?;
    out.cells += fitted.cells;
    if fitted.status != Status::Certified {
        return Ok(out);
    }
    fitted.cells = out.cells;
    out.status = Status::Certified;
    out.reason = None;
    out.fit = Some(fitted);
    Ok(out)
}

pub(super) fn certify(sweep: &Sweep<'_>, max_cells: usize) -> Result<ContactAnchorReport> {
    check(
        max_cells <= 100000,
        "Contact anchor work exceeds100000 cells",
    )?;
    let mut out = ContactAnchorReport {
        status: Status::Unresolved,
        cells: 0,
        point: None,
        coordinates: None,
        reason: Some("mode-not-contact"),
    };
    let Some((source, parameter)) = sweep.contact_source else {
        return Ok(out);
    };
    let Some(guide) = sweep.orientation_guide else {
        return Ok(out);
    };
    let [a, b] = source.domain();
    // Outward inverse mapping encloses the original parameter. Its forward
    // mapping in the value certificate therefore contains the source point.
    let fraction = I::point(parameter)
        .sub(I::point(a))?
        .div(I::point(b).sub(I::point(a))?)?
        .intersect(0., 1.)?;
    out.reason = Some("contact-source-point-unresolved");
    let point = vector_certificate::certify_values_traversal(
        source,
        [fraction.lo, fraction.hi],
        max_cells,
        false,
    )?;
    out.cells += point.cells;
    let Some(point) = point.value else {
        return Ok(out);
    };
    out.reason = Some("contact-path-start-unresolved");
    let start = vector_certificate::certify_values_traversal(
        sweep.path,
        [0., 0.],
        max_cells - out.cells,
        false,
    )?;
    out.cells += start.cells;
    let Some(start) = start.value else {
        return Ok(out);
    };
    let zero = constant_vector_law([0.; 3])?;
    out.reason = Some("contact-initial-frame-unresolved");
    let frame = authored_frame_certificate::certify_path_guide_values(
        sweep.path,
        guide,
        &zero,
        [0., 0.],
        max_cells - out.cells,
    )?;
    out.cells += frame.cells;
    if frame.status != Status::Certified {
        return Ok(out);
    }
    let decode = |v: [[f64; 2]; 3]| -> Result<[I; 3]> {
        Ok([
            I::new(v[0][0], v[0][1])?,
            I::new(v[1][0], v[1][1])?,
            I::new(v[2][0], v[2][1])?,
        ])
    };
    let offset = subtract(decode(point)?, decode(start)?)?;
    let coordinates = [
        dot_tight(offset, decode(frame.transverse.unwrap())?)?,
        dot_tight(offset, decode(frame.binormal.unwrap())?)?,
        dot_tight(offset, decode(frame.longitudinal.unwrap())?)?,
    ];
    out.reason = Some("contact-anchor-positivity-unresolved");
    if coordinates[0].lo <= 0. {
        return Ok(out);
    }
    out.status = Status::Certified;
    out.reason = None;
    out.point = Some(point);
    out.coordinates = Some(coordinates.map(|x| [x.lo, x.hi]));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_rational_interior_anchor_encloses_translated_coordinates_and_shares_work() {
        let mut profile = crate::primitives::line([1.5, 1., -1.], [2.5, 1., -1.]).unwrap();
        profile.knots = vec![7., 7., 9., 9.];
        profile.weights = vec![1., 2.];
        let path = crate::primitives::line([0.5, 1., -1.], [0.5, 1., 9.]).unwrap();
        let guide =
            crate::primitives::line([0.5 + 5. / 3., 1., -1.], [0.5 + 5. / 3., 1., 9.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let options = Options {
            normal: [1., 0., 0.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::Parameter,
            initial_sections: 3,
            max_sections: 3,
            max_deviation: 0.01,
        };
        let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
            .unwrap()
            .with_contact_guide(&guide, 8.)
            .unwrap();
        let report = sweep.contact_anchor_bound(100).unwrap();
        assert_eq!(report.status, Status::Certified);
        let q = report.coordinates.unwrap();
        for (bounds, expected) in q.into_iter().zip([5. / 3., 0., 0.]) {
            assert!(bounds[0] <= expected && expected <= bounds[1]);
            assert!(bounds[1] - bounds[0] < 1e-10);
        }
        let point = report.point.unwrap();
        assert!(point[0][0] <= 0.5 + 5. / 3. && 0.5 + 5. / 3. <= point[0][1]);
        let trajectory = sweep
            .contact_control_trajectory(0, [0.25, 0.5], 200)
            .unwrap();
        for t in [0.0_f64, 0.13, 0.375, 0.5, 0.87, 1.] {
            let report = sweep.contact_control_value(0, [t, t], 200).unwrap();
            assert_eq!(report.status, Status::Certified);
            let value = report.value.unwrap();
            let expected = [1.5, 1., -1. + 10. * t];
            for k in 0..3 {
                assert!(value[k][0] <= expected[k] && expected[k] <= value[k][1]);
                assert!(value[k][1] - value[k][0] < 1e-9);
            }
            let short = sweep
                .contact_control_value(0, [t, t], report.cells - 1)
                .unwrap();
            assert_eq!(short.status, Status::Unresolved);
            assert!(short.value.is_none() && short.cells <= report.cells - 1);
        }
        assert_eq!(trajectory.status, Status::Certified);
        let original = trajectory.jet.unwrap();
        for t in [0.25_f64, 0.375, 0.5] {
            let expected = [1.5, 1., -1. + 10. * t];
            for k in 0..3 {
                assert!(original.value[k][0] <= expected[k] && expected[k] <= original.value[k][1]);
                let derivative = if k == 2 { 10. } else { 0. };
                assert!(original.first[k][0] <= derivative && derivative <= original.first[k][1]);
                assert!(original.second[k][0] <= 0. && 0. <= original.second[k][1]);
            }
        }
        let exhausted = sweep
            .contact_control_trajectory(0, [0.25, 0.5], trajectory.cells - 1)
            .unwrap();
        assert_eq!(exhausted.status, Status::Unresolved);
        assert!(exhausted.jet.is_none() && exhausted.cells <= trajectory.cells - 1);
        assert!(
            sweep
                .contact_control_trajectory(2, [0.25, 0.5], 200)
                .is_err()
        );
        let fitted = sweep.contact_fit_jet([0.25, 0.5], 100).unwrap();
        assert_eq!(fitted.status, Status::Certified);
        let jet = fitted.fit.unwrap();
        assert_eq!(jet.cells, fitted.cells);
        assert!(jet.value.unwrap()[0] <= 1. && 1. <= jet.value.unwrap()[1]);
        assert!(jet.first.unwrap()[0] <= 0. && 0. <= jet.first.unwrap()[1]);
        assert!(jet.second.unwrap()[0] <= 0. && 0. <= jet.second.unwrap()[1]);
        let limited = sweep
            .contact_fit_jet([0.25, 0.5], fitted.cells - 1)
            .unwrap();
        assert_eq!(limited.status, Status::Unresolved);
        assert!(limited.fit.is_none() && limited.cells <= fitted.cells - 1);
        let short = sweep.contact_anchor_bound(report.cells - 1).unwrap();
        assert_eq!(short.status, Status::Unresolved);
        assert!(short.point.is_none() && short.coordinates.is_none());
        assert!(short.cells <= report.cells - 1);
        let plain = sweep.with_orientation_guide(&guide).unwrap();
        assert_eq!(
            plain.contact_anchor_bound(100).unwrap().reason,
            Some("mode-not-contact")
        );
    }
}
