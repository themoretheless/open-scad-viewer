//! Original-law contact fit values, including point restrictions. No derivatives
//! or contact identities are inferred; anchor ownership is a separate premise.
use super::*;

#[derive(Clone, Debug)]
pub struct ContactFitValueReport {
    pub status: Status,
    pub cells: usize,
    pub value: Option<[f64; 2]>,
}

/// Original contact control value with explicit coordinate/anchor premises.
pub fn certify_contact_control_value(
    path: &Curve,
    guide: &Curve,
    scale: &Curve,
    twist: &Curve,
    affine: Option<(&Curve, &Curve)>,
    q: [[f64; 2]; 3],
    anchor: [f64; 2],
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<ControlValueReport> {
    twist.validate()?;
    check(
        twist.control_points.iter().all(|p| p[0] == 0.),
        "Contact control requires zero twist",
    )?;
    let fit = certify_contact_fit_value(path, guide, scale, affine, anchor, traversal, max_cells)?;
    if fit.status != Status::Certified {
        return Ok(ControlValueReport {
            status: Status::Unresolved,
            cells: fit.cells,
            value: None,
            reason: Some("contact-fit-value-unresolved"),
        });
    }
    let mut frame =
        certify_path_guide_values(path, guide, twist, traversal, max_cells - fit.cells)?;
    frame.cells += fit.cells;
    let fit = fit.value.unwrap();
    super::trajectory::control_value_with_fit(
        path,
        scale,
        affine,
        q,
        traversal,
        max_cells,
        frame,
        Some(I::new(fit[0], fit[1])?),
    )
}

pub fn certify_contact_fit_value(
    path: &Curve,
    guide: &Curve,
    scale: &Curve,
    affine: Option<(&Curve, &Curve)>,
    anchor: [f64; 2],
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<ContactFitValueReport> {
    let anchor = I::new(anchor[0], anchor[1])?;
    check(anchor.lo > 0., "Contact anchor width must be positive")?;
    super::super::validate_law(scale, true)?;
    let mut out = ContactFitValueReport {
        status: Status::Unresolved,
        cells: 0,
        value: None,
    };
    let p = vector_certificate::certify_values_traversal(path, traversal, max_cells, false)?;
    out.cells += p.cells;
    let Some(p) = p.value else {
        return Ok(out);
    };
    let g = vector_certificate::certify_values_traversal(
        guide,
        traversal,
        max_cells - out.cells,
        false,
    )?;
    out.cells += g.cells;
    let Some(g) = g.value else {
        return Ok(out);
    };
    let offset = sub(decode(g)?, decode(p)?)?;
    let squared = square(offset[0])?
        .add(square(offset[1])?)?
        .add(square(offset[2])?)?;
    let width = I::new(
        squared.lo.max(0.).sqrt().next_down().max(0.),
        squared.hi.sqrt().next_up(),
    )?;
    if width.lo <= 0. {
        return Ok(out);
    }
    let charge = (scale.degree..scale.control_points.len())
        .filter(|&i| scale.knots[i] < scale.knots[i + 1])
        .count();
    if charge > max_cells - out.cells {
        return Ok(out);
    }
    let uniform = super::super::scalar_certificate::value_traversal(scale, traversal, charge)?;
    out.cells += charge;
    let Some(uniform) = uniform else {
        return Ok(out);
    };
    let mut axis = I::point(1.);
    let mut center = I::point(0.);
    if let Some((axes, centers)) = affine {
        let a = vector_certificate::certify_values_traversal(
            axes,
            traversal,
            max_cells - out.cells,
            true,
        )?;
        out.cells += a.cells;
        let Some(a) = a.value else {
            return Ok(out);
        };
        axis = I::new(a[0][0], a[0][1])?;
        let c = vector_certificate::certify_values_traversal(
            centers,
            traversal,
            max_cells - out.cells,
            false,
        )?;
        out.cells += c.cells;
        let Some(c) = c.value else {
            return Ok(out);
        };
        center = I::new(c[0][0], c[0][1])?;
    }
    let denominator = I::new(uniform[0], uniform[1])?
        .mul(axis)?
        .mul(anchor)?
        .add(center)?;
    if denominator.lo <= 0. {
        return Ok(out);
    }
    let value = width.div(denominator)?;
    out.status = Status::Certified;
    out.value = Some([value.lo, value.hi]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contact_fit_point_values_enclose_independent_joint_formula_tightly() {
        let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
        let guide = crate::primitives::line([2., 0., 0.], [2., 0., 10.]).unwrap();
        let mut scale = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        scale.knots = vec![7., 7., 9., 9.];
        let mut axes = crate::primitives::line([1.; 3], [2., 1., 1.]).unwrap();
        axes.knots = vec![17., 17., 19., 19.];
        let mut center = crate::primitives::line([0.; 3], [0.5, 0., 0.]).unwrap();
        center.knots = vec![23., 23., 29., 29.];
        for t in [0.0_f64, 0.13, 0.375, 0.5, 0.87, 1.] {
            let report = certify_contact_fit_value(
                &path,
                &guide,
                &scale,
                Some((&axes, &center)),
                [2., 2.],
                [t, t],
                100,
            )
            .unwrap();
            assert_eq!(report.status, Status::Certified);
            let bounds = report.value.unwrap();
            let expected = 2. / (2. * (1. + t).powi(2) + 0.5 * t);
            assert!(bounds[0] <= expected && expected <= bounds[1]);
            assert!(bounds[1] - bounds[0] < 1e-9);
            let mut twist = scale.clone();
            twist.control_points = vec![vec![0.; 3]; 2];
            let value = certify_contact_control_value(
                &path,
                &guide,
                &scale,
                &twist,
                Some((&axes, &center)),
                [[1., 1.], [3., 3.], [4., 4.]],
                [2., 2.],
                [t, t],
                200,
            )
            .unwrap();
            assert_eq!(value.status, Status::Certified);
            let xyz = value.value.unwrap();
            let ideal = [
                expected * ((1. + t).powi(2) + 0.5 * t),
                3. * (1. + t),
                4. + 14. * t,
            ];
            for k in 0..3 {
                assert!(xyz[k][0] <= ideal[k] && ideal[k] <= xyz[k][1]);
                assert!(xyz[k][1] - xyz[k][0] < 1e-9);
            }
            let limited = certify_contact_control_value(
                &path,
                &guide,
                &scale,
                &twist,
                Some((&axes, &center)),
                [[1., 1.], [3., 3.], [4., 4.]],
                [2., 2.],
                [t, t],
                value.cells - 1,
            )
            .unwrap();
            assert_eq!(limited.status, Status::Unresolved);
            assert!(limited.value.is_none());
            let short = certify_contact_fit_value(
                &path,
                &guide,
                &scale,
                Some((&axes, &center)),
                [2., 2.],
                [t, t],
                report.cells - 1,
            )
            .unwrap();
            assert_eq!(short.status, Status::Unresolved);
            assert!(short.value.is_none() && short.cells <= report.cells - 1);
        }
    }
}
