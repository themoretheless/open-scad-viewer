//! Constructor-owned original-law premises for authored sweep error.
use super::*;
use crate::distance_bounds::Interval as I;
use crate::numerics::interval_vec3::{dot_tight, sub as interval_sub};
use crate::sweeps::progressive_miter::{
    authored_frame_certificate, scalar_certificate::Status, vector_certificate,
};

#[derive(Clone, Debug)]
pub struct InitialCoordinatesReport {
    pub status: Status,
    pub cells: usize,
    pub coordinates: Option<Vec<[[f64; 2]; 3]>>,
    pub reason: Option<&'static str>,
}
pub(super) fn initial_coordinates(
    sweep: &Sweep<'_>,
    max_cells: usize,
) -> Result<InitialCoordinatesReport> {
    check(
        max_cells <= 100000,
        "Initial coordinate work exceeds100000 cells",
    )?;
    let mut out = InitialCoordinatesReport {
        status: Status::Unresolved,
        cells: 0,
        coordinates: None,
        reason: Some("mode-not-authored"),
    };
    let Some((axis, normal)) = sweep.frame_laws else {
        return Ok(out);
    };
    out.reason = Some("path-start-enclosure-unresolved");
    let start =
        vector_certificate::certify_values_traversal(sweep.path, [0., 0.], max_cells, false)?;
    out.cells += start.cells;
    let Some(start) = start.value else {
        return Ok(out);
    };
    // The initial local basis precedes twist and affine transformation.
    let zero_twist = constant_vector_law([0.; 3])?;
    out.reason = Some("initial-frame-enclosure-unresolved");
    let frame = authored_frame_certificate::certify_twisted_values(
        axis,
        normal,
        &zero_twist,
        [0., 0.],
        max_cells - out.cells,
    )?;
    out.cells += frame.cells;
    if frame.status != Status::Certified {
        return Ok(out);
    }
    out.status = Status::Certified;
    out.reason = None;
    out.coordinates = Some(coordinates_in_basis(sweep.profile, start, frame)?);
    Ok(out)
}
fn coordinates_in_basis(
    profile: &Curve,
    start: [[f64; 2]; 3],
    frame: authored_frame_certificate::ValuesReport,
) -> Result<Vec<[[f64; 2]; 3]>> {
    let decode = |v: [[f64; 2]; 3]| -> Result<[I; 3]> {
        Ok([
            I::new(v[0][0], v[0][1])?,
            I::new(v[1][0], v[1][1])?,
            I::new(v[2][0], v[2][1])?,
        ])
    };
    let start = decode(start)?;
    let basis = [
        decode(frame.transverse.unwrap())?,
        decode(frame.binormal.unwrap())?,
        decode(frame.longitudinal.unwrap())?,
    ];
    let mut coordinates = Vec::with_capacity(profile.control_points.len());
    for pole in &profile.control_points {
        let offset = interval_sub(
            [I::point(pole[0]), I::point(pole[1]), I::point(pole[2])],
            start,
        )?;
        let q = [
            dot_tight(offset, basis[0])?,
            dot_tight(offset, basis[1])?,
            dot_tight(offset, basis[2])?,
        ];
        coordinates.push(q.map(|x| [x.lo, x.hi]));
    }
    Ok(coordinates)
}

pub(super) fn guided_initial_coordinates(
    sweep: &Sweep<'_>,
    max_cells: usize,
) -> Result<InitialCoordinatesReport> {
    check(
        max_cells <= 100000,
        "Initial coordinate work exceeds100000 cells",
    )?;
    let mut out = InitialCoordinatesReport {
        status: Status::Unresolved,
        cells: 0,
        coordinates: None,
        reason: Some("mode-not-guided"),
    };
    let Some(guide) = sweep.orientation_guide else {
        return Ok(out);
    };
    out.reason = Some("guided-path-start-enclosure-unresolved");
    let start =
        vector_certificate::certify_values_traversal(sweep.path, [0., 0.], max_cells, false)?;
    out.cells += start.cells;
    let Some(start) = start.value else {
        return Ok(out);
    };
    let zero_twist = constant_vector_law([0.; 3])?;
    out.reason = Some("guided-initial-frame-enclosure-unresolved");
    let frame = authored_frame_certificate::certify_path_guide_values(
        sweep.path,
        guide,
        &zero_twist,
        [0., 0.],
        max_cells - out.cells,
    )?;
    out.cells += frame.cells;
    if frame.status != Status::Certified {
        return Ok(out);
    }
    out.coordinates = Some(coordinates_in_basis(sweep.profile, start, frame)?);
    out.status = Status::Certified;
    out.reason = None;
    Ok(out)
}

pub(super) fn control_trajectory(
    sweep: &Sweep<'_>,
    profile_control: usize,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<authored_frame_certificate::TrajectoryReport> {
    check(
        profile_control < sweep.profile.control_points.len(),
        "Profile control index outside sweep",
    )?;
    check(
        max_cells <= 100000,
        "Control trajectory work exceeds100000 cells",
    )?;
    let unresolved = |cells, reason| authored_frame_certificate::TrajectoryReport {
        traversal,
        status: Status::Unresolved,
        cells,
        jet: None,
        single_span: false,
        reason: Some(reason),
    };
    if sweep.options.spacing != Spacing::Parameter {
        return Ok(unresolved(0, "arc-length-correspondence-unproved"));
    }
    let [a, b] = sweep.path.domain();
    let p = sweep.path.evaluate(a)?.point;
    let q = sweep.path.evaluate(b)?.point;
    if closed_extent(sweep.path, [p[0], p[1], p[2]], [q[0], q[1], q[2]])? {
        return Ok(unresolved(0, "closed-frame-correction-unproved"));
    }
    let initial = initial_coordinates(sweep, max_cells)?;
    if initial.status != Status::Certified {
        return Ok(unresolved(initial.cells, initial.reason.unwrap()));
    }
    let (axis, normal) = sweep.frame_laws.unwrap();
    let mut report = authored_frame_certificate::certify_control_trajectory(
        sweep.path,
        sweep.scale,
        sweep.twist,
        axis,
        normal,
        sweep.affine_laws,
        initial.coordinates.unwrap()[profile_control],
        traversal,
        max_cells - initial.cells,
    )?;
    report.cells += initial.cells;
    Ok(report)
}

#[derive(Clone, Debug)]
pub struct SectionInterpolationReport {
    pub status: Status,
    pub cells: usize,
    pub error_upper: Option<f64>,
    pub endpoint_displacement_upper: Option<f64>,
    pub reason: Option<&'static str>,
}
pub(super) fn section_interpolation(
    sweep: &Sweep<'_>,
    count: usize,
    max_cells: usize,
) -> Result<SectionInterpolationReport> {
    check(
        count >= sweep.options.initial_sections && count <= sweep.options.max_sections,
        "Section count outside configured sweep",
    )?;
    check(
        max_cells <= 100000,
        "Section interpolation work exceeds100000 cells",
    )?;
    let mut out = SectionInterpolationReport {
        status: Status::Unresolved,
        cells: 0,
        error_upper: None,
        endpoint_displacement_upper: None,
        reason: Some("mode-not-authored"),
    };
    let Some((axis, normal)) = sweep.frame_laws else {
        return Ok(out);
    };
    if sweep.options.spacing != Spacing::Parameter {
        out.reason = Some("arc-length-correspondence-unproved");
        return Ok(out);
    }
    if path_is_closed(sweep.path)? {
        out.reason = Some("closed-frame-correction-unproved");
        return Ok(out);
    }
    let initial = initial_coordinates(sweep, max_cells)?;
    out.cells += initial.cells;
    if initial.status != Status::Certified {
        out.reason = initial.reason;
        return Ok(out);
    }
    let coordinates = initial.coordinates.unwrap();
    let sections = sweep.sections(count)?.0;
    // Positive, unchanged rational profile bases make the profile error a
    // convex combination of corresponding control-trajectory errors.
    check(
        sections.iter().all(|c| {
            c.degree == sweep.profile.degree
                && c.knots == sweep.profile.knots
                && c.weights == sweep.profile.weights
                && c.periodic == sweep.profile.periodic
                && c.control_points.len() == coordinates.len()
        }),
        "Retained section basis correspondence changed",
    )?;
    let mut endpoint_errors = vec![vec![0.; coordinates.len()]; count];
    let mut endpoint_upper = 0_f64;
    for (station, section) in sections.iter().enumerate() {
        let t = station as f64 / (count - 1) as f64;
        for (control, &q) in coordinates.iter().enumerate() {
            let value = authored_frame_certificate::certify_control_value(
                sweep.path,
                sweep.scale,
                sweep.twist,
                axis,
                normal,
                sweep.affine_laws,
                q,
                [t, t],
                max_cells - out.cells,
            )?;
            out.cells += value.cells;
            if value.status != Status::Certified {
                out.reason = value.reason;
                return Ok(out);
            }
            let p = &section.control_points[control];
            let upper = value
                .retained_displacement_upper([p[0], p[1], p[2]])?
                .unwrap();
            endpoint_errors[station][control] = upper;
            endpoint_upper = endpoint_upper.max(upper);
        }
    }
    let mut error = 0_f64;
    for station in 0..count - 1 {
        let interval = [
            station as f64 / (count - 1) as f64,
            (station + 1) as f64 / (count - 1) as f64,
        ];
        for (control, &q) in coordinates.iter().enumerate() {
            let jet = authored_frame_certificate::certify_control_trajectory(
                sweep.path,
                sweep.scale,
                sweep.twist,
                axis,
                normal,
                sweep.affine_laws,
                q,
                interval,
                max_cells - out.cells,
            )?;
            out.cells += jet.cells;
            if jet.status != Status::Certified {
                out.reason = jet.reason;
                return Ok(out);
            }
            let smooth_upper = jet.linear_error_upper([
                endpoint_errors[station][control],
                endpoint_errors[station + 1][control],
            ])?;
            let upper = if let Some(upper) = smooth_upper {
                upper
            } else {
                // A knot transition cannot use a global second-derivative
                // remainder. Enclose the original value image and the actual
                // retained control segment on this same station interval.
                let value = authored_frame_certificate::certify_control_value(
                    sweep.path,
                    sweep.scale,
                    sweep.twist,
                    axis,
                    normal,
                    sweep.affine_laws,
                    q,
                    interval,
                    max_cells - out.cells,
                )?;
                out.cells += value.cells;
                let a = &sections[station].control_points[control];
                let b = &sections[station + 1].control_points[control];
                let Some(upper) = value.retained_segment_displacement_upper([
                    [a[0], a[1], a[2]],
                    [b[0], b[1], b[2]],
                ])?
                else {
                    out.reason = value.reason.or(Some("section-knot-value-bound-unproved"));
                    return Ok(out);
                };
                upper
            };
            error = error.max(upper);
        }
    }
    out.status = Status::Certified;
    out.error_upper = Some(error);
    out.endpoint_displacement_upper = Some(endpoint_upper);
    out.reason = None;
    Ok(out)
}

#[derive(Clone, Debug)]
pub struct PatchErrorReport {
    pub status: Status,
    pub cells: usize,
    pub products: usize,
    pub section_error_upper: Option<f64>,
    pub decomposition_error_upper: Option<f64>,
    pub error_upper: Option<f64>,
    pub within_budget: bool,
    pub patches: Option<Vec<Surface>>,
    pub reason: Option<&'static str>,
}
pub(super) fn patch_error(
    sweep: &Sweep<'_>,
    count: usize,
    max_cells: usize,
    max_products: usize,
) -> Result<PatchErrorReport> {
    check(
        max_products <= 1000000,
        "Patch decomposition work exceeds1000000 products",
    )?;
    let section = section_interpolation(sweep, count, max_cells)?;
    patch_error_from_section(sweep, count, max_products, section)
}
pub(super) fn guided_patch_error(
    sweep: &Sweep<'_>,
    count: usize,
    max_cells: usize,
    max_products: usize,
) -> Result<PatchErrorReport> {
    check(
        max_products <= 1000000,
        "Patch decomposition work exceeds1000000 products",
    )?;
    let section = guided_section_interpolation(sweep, count, max_cells)?;
    patch_error_from_section(sweep, count, max_products, section)
}
pub(super) fn contact_patch_error(
    sweep: &Sweep<'_>, count: usize, max_cells: usize, max_products: usize,
) -> Result<PatchErrorReport> {
    check(max_products <= 1000000, "Patch decomposition work exceeds1000000 products")?;
    let section = contact_section_interpolation(sweep, count, max_cells)?;
    patch_error_from_section(sweep, count, max_products, section)
}
fn patch_error_from_section(
    sweep: &Sweep<'_>,
    count: usize,
    max_products: usize,
    section: SectionInterpolationReport,
) -> Result<PatchErrorReport> {
    let mut out = PatchErrorReport {
        status: Status::Unresolved,
        cells: section.cells,
        products: 0,
        section_error_upper: section.error_upper,
        decomposition_error_upper: None,
        error_upper: None,
        within_budget: false,
        patches: None,
        reason: section.reason,
    };
    if section.status != Status::Certified {
        return Ok(out);
    }
    let sections = sweep.sections(count)?.0;
    let rows = sections
        .iter()
        .map(profile_parts)
        .collect::<Result<Vec<_>>>()?;
    let mut decomposition = 0_f64;
    if sweep.profile.control_points.len() > 32 {
        let spans = (sweep.profile.degree..sweep.profile.control_points.len())
            .filter(|&i| sweep.profile.knots[i] < sweep.profile.knots[i + 1])
            .collect::<Vec<_>>();
        for (source, row) in sections.iter().zip(&rows) {
            if row.len() != spans.len() {
                out.reason = Some("decomposition-domain-cover-unproved");
                return Ok(out);
            }
            for (part, &span) in row.iter().zip(&spans) {
                let proof = crate::curve_decomposition_certificate::inspect(
                    source,
                    span,
                    part,
                    max_products - out.products,
                )?;
                out.products += proof.products;
                let Some(error) = proof.error_upper else {
                    out.reason = proof.reason;
                    return Ok(out);
                };
                decomposition = decomposition.max(error);
            }
        }
    }
    // Constant rational U weights across stations make V interpolation a
    // convex interpolation of the corresponding decomposed section curves.
    let first = &rows[0];
    if rows.iter().any(|row| {
        row.len() != first.len()
            || row.iter().zip(first).any(|(a, b)| {
                a.degree != b.degree
                    || a.knots != b.knots
                    || a.weights != b.weights
                    || a.periodic != b.periodic
            })
    }) {
        out.reason = Some("decomposed-station-basis-correspondence-unproved");
        return Ok(out);
    }
    let error = I::point(section.error_upper.unwrap())
        .add(I::point(decomposition))?
        .hi;
    let retained = patches(&sections)?;
    out.status = Status::Certified;
    out.error_upper = Some(error);
    out.decomposition_error_upper = Some(decomposition);
    out.within_budget = error <= sweep.options.max_deviation;
    out.patches = Some(retained);
    out.reason = None;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owned_dense_profile_patch_error_includes_decomposition_and_product_budget() {
        let profile = Curve {
            degree: 1,
            knots: std::iter::once(0.)
                .chain((0..=32).map(|i| i as f64))
                .chain(std::iter::once(32.))
                .collect(),
            control_points: (0..=32)
                .map(|i| vec![1. + i as f64 / 32., (i % 2) as f64 / 64., 0.])
                .collect(),
            weights: (0..=32).map(|i| if i % 2 == 0 { 1. } else { 2. }).collect(),
            periodic: false,
        };
        let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
        let axis = constant_vector_law([0., 0., 1.]).unwrap();
        let normal = constant_vector_law([1., 0., 0.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.25, 0., 0.]).unwrap();
        let options = Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 3,
            max_sections: 3,
            max_deviation: 0.01,
        };
        let sweep =
            Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, options).unwrap();
        let report = sweep.authored_patch_error_bound(3, 10000, 1000).unwrap();
        assert_eq!(report.status, Status::Certified);
        assert_eq!(report.products, 384);
        assert!(report.within_budget && report.error_upper.unwrap() < 1e-9);
        assert!(report.decomposition_error_upper.unwrap() > 0.);
        let patches = report.patches.unwrap();
        assert_eq!(patches.len(), 32);
        let (s, c) = 0.25_f64.sin_cos();
        for (span, patch) in patches.iter().enumerate() {
            let a = patch.knots_u[patch.degree_u];
            let b = patch.knots_u[patch.control_points.len()];
            for f in [0., 0.375, 1.] {
                let p = profile.evaluate(span as f64 + f).unwrap().point;
                for t in [0., 0.375, 1.] {
                    let got = patch.evaluate(a + (b - a) * f, t).unwrap().point;
                    let expected = [p[0] * c - p[1] * s, p[0] * s + p[1] * c, 10. * t];
                    assert!(
                        norm(std::array::from_fn(|k| got[k] - expected[k]))
                            <= report.error_upper.unwrap()
                    );
                }
            }
        }
        let refused = sweep.authored_patch_error_bound(3, 10000, 383).unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert!(
            refused.patches.is_none() && refused.error_upper.is_none() && refused.products <= 383
        );
    }
    #[test]
    fn owned_section_bound_covers_varying_joint_laws_and_refinement() {
        let mut profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        profile.weights = vec![1., 2.];
        let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
        let axis = constant_vector_law([0., 0., 1.]).unwrap();
        let normal = crate::primitives::line([1., 0., 0.], [1., 1., 0.]).unwrap();
        let scale = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        let twist = crate::primitives::line([0., 0., 0.], [0.25, 0., 0.]).unwrap();
        let axes = crate::primitives::line([1.; 3], [2., 1., 1.]).unwrap();
        let center = crate::primitives::line([0.; 3], [0.5, 0., 0.]).unwrap();
        let options = Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 3,
            max_sections: 9,
            max_deviation: 0.01,
        };
        let sweep = Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, options)
            .unwrap()
            .with_affine_laws(&axes, &center)
            .unwrap();
        let coarse = sweep
            .authored_section_interpolation_bound(3, 10000)
            .unwrap();
        let fine = sweep
            .authored_section_interpolation_bound(9, 10000)
            .unwrap();
        assert_eq!(coarse.status, Status::Certified);
        assert_eq!(fine.status, Status::Certified);
        assert!(fine.error_upper.unwrap() < coarse.error_upper.unwrap());
        let sections = sweep.sections(3).unwrap().0;
        for i in 0..=32 {
            let t = i as f64 / 32.;
            let station = (i / 16).min(1);
            let fraction = t * 2. - station as f64;
            for u in [0., 0.375, 1.] {
                let a = sections[station].evaluate(u).unwrap().point;
                let b = sections[station + 1].evaluate(u).unwrap().point;
                let amplitude = (1. + 3. * u) / (1. + u) * (1. + t).powi(2) + 0.5 * t;
                let angle = t.atan() + 0.25 * t;
                let ideal = [amplitude * angle.cos(), amplitude * angle.sin(), 10. * t];
                let delta =
                    std::array::from_fn(|k| (1. - fraction) * a[k] + fraction * b[k] - ideal[k]);
                assert!(norm(delta) <= coarse.error_upper.unwrap());
            }
        }
    }
    #[test]
    fn authored_initial_coordinates_enclose_original_profile_in_independent_domains() {
        let mut profile = crate::primitives::line([1., 2., 3.], [2., 2., 3.]).unwrap();
        profile.weights = vec![1., 2.];
        let mut path = crate::primitives::line([0.5, 1., -1.], [0.5, 1., 9.]).unwrap();
        path.knots = vec![17., 17., 19., 19.];
        let mut axis = constant_vector_law([2., 0., 0.]).unwrap();
        axis.knots = vec![2., 2., 5., 5.];
        let mut normal = constant_vector_law([7., 1., 0.]).unwrap();
        normal.knots = vec![31., 31., 41., 41.];
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.25, 0., 0.]).unwrap();
        let options = Options {
            normal: [0., 1., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 3,
            max_sections: 3,
            max_deviation: 0.01,
        };
        let sweep =
            Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, options).unwrap();
        let report = sweep.authored_initial_coordinates(100).unwrap();
        assert_eq!(report.status, Status::Certified);
        assert_eq!(report.cells, 10);
        let coordinates = report.coordinates.unwrap();
        for (q, expected) in coordinates.iter().zip([[1., 4., 0.5], [1., 4., 1.5]]) {
            for k in 0..3 {
                assert!(q[k][0] <= expected[k] && expected[k] <= q[k][1]);
            }
        }
        let trajectory = sweep.authored_control_trajectory(0, [0., 1.], 100).unwrap();
        assert_eq!(trajectory.status, Status::Certified);
        assert_eq!(trajectory.cells, 21);
        let jet = trajectory.jet.unwrap();
        let (s, c) = 0.25_f64.sin_cos();
        for i in 0..=16 {
            let t = i as f64 / 16.;
            let expected = [1., 1. + c - 4. * s, -1. + 10. * t + s + 4. * c];
            for k in 0..3 {
                assert!(jet.value[k][0] <= expected[k] && expected[k] <= jet.value[k][1]);
                let derivative = if k == 2 { 10. } else { 0. };
                assert!(jet.first[k][0] <= derivative && derivative <= jet.first[k][1]);
            }
        }
        let limited = sweep.authored_control_trajectory(0, [0., 1.], 20).unwrap();
        assert_eq!(limited.status, Status::Unresolved);
        assert!(limited.jet.is_none() && limited.cells <= 20);
        let bound = sweep.authored_section_interpolation_bound(3, 1000).unwrap();
        assert_eq!(bound.status, Status::Certified);
        assert_eq!(bound.cells, 120);
        assert!(bound.error_upper.unwrap() < 1e-10);
        assert!(bound.endpoint_displacement_upper.unwrap() < 1e-10);
        let incomplete = sweep
            .authored_section_interpolation_bound(3, bound.cells - 1)
            .unwrap();
        assert_eq!(incomplete.status, Status::Unresolved);
        assert!(
            incomplete.error_upper.is_none() && incomplete.endpoint_displacement_upper.is_none()
        );
        assert!(incomplete.cells < bound.cells);
        assert!(sweep.authored_control_trajectory(2, [0., 1.], 100).is_err());
        for budget in [0, 9] {
            let refused = sweep.authored_initial_coordinates(budget).unwrap();
            assert_eq!(refused.status, Status::Unresolved);
            assert!(refused.coordinates.is_none() && refused.cells <= budget);
        }
        let plain = Sweep::new(&profile, &path, &scale, &twist, options).unwrap();
        assert_eq!(
            plain.authored_initial_coordinates(100).unwrap().reason,
            Some("mode-not-authored")
        );
        let arc_options = Options {
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100,
            },
            ..options
        };
        let arc = Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, arc_options)
            .unwrap();
        assert_eq!(
            arc.authored_control_trajectory(0, [0., 1.], 100)
                .unwrap()
                .reason,
            Some("arc-length-correspondence-unproved")
        );
    }
}

#[test]
fn original_knot_transition_uses_owned_value_bound_and_shared_work() {
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let mut path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    path.knots = vec![2., 2., 3.2, 5., 5.];
    path.control_points = vec![vec![0., 0., 0.], vec![0., 0., 3.], vec![0., 0., 10.]];
    path.weights = vec![1.; 3];
    let constant = |p: [f64; 3]| {
        let mut curve = profile.clone();
        curve.control_points = vec![p.to_vec(); 2];
        curve
    };
    let axis = constant([0., 0., 1.]);
    let normal = constant([1., 0., 0.]);
    let scale = constant([1., 0., 0.]);
    let twist = constant([0.; 3]);
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::Fixed,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 100.,
    };
    let sweep =
        Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, options).unwrap();
    let report = sweep
        .authored_section_interpolation_bound(3, 10000)
        .unwrap();
    assert_eq!(report.status, Status::Certified);
    assert!(report.error_upper.unwrap() > 0.5);
    let sections = sweep.sections(3).unwrap().0;
    for i in 0..=100 {
        let t = i as f64 / 100.;
        let expected = path.evaluate(2. + 3. * t).unwrap().point[2];
        let station = if t < 0.5 { 0 } else { 1 };
        let f = 2. * t - station as f64;
        let got = (1. - f) * sections[station].control_points[0][2]
            + f * sections[station + 1].control_points[0][2];
        assert!((got - expected).abs() <= report.error_upper.unwrap());
    }
    let exhausted = sweep
        .authored_section_interpolation_bound(3, report.cells - 1)
        .unwrap();
    assert_eq!(exhausted.status, Status::Unresolved);
    assert!(exhausted.error_upper.is_none());
    assert!(exhausted.cells < report.cells);
}

#[test]
fn knot_fallback_covers_every_authored_transport_law() {
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let constant = |p: [f64; 3]| {
        let mut c = profile.clone();
        c.control_points = vec![p.to_vec(); 2];
        c
    };
    let piecewise = |a: [f64; 3], b: [f64; 3], c: [f64; 3], domain: [f64; 2]| Curve {
        degree: 1,
        knots: vec![
            domain[0],
            domain[0],
            domain[0] + 0.4 * (domain[1] - domain[0]),
            domain[1],
            domain[1],
        ],
        control_points: vec![a.to_vec(), b.to_vec(), c.to_vec()],
        weights: vec![1., 0.5, 2.],
        periodic: false,
    };
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::Fixed,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 100.,
    };
    for law_index in 0..7 {
        let mut scale = constant([1., 0., 0.]);
        let mut twist = constant([0.; 3]);
        let mut axis = constant([0., 0., 1.]);
        let mut normal = constant([1., 0., 0.]);
        let mut axes = constant([1.; 3]);
        let mut center = constant([0.; 3]);
        match law_index {
            0 => scale = piecewise([1., 0., 0.], [1.3, 0., 0.], [2., 0., 0.], [2., 5.]),
            1 => twist = piecewise([0.; 3], [0.1, 0., 0.], [0.3, 0., 0.], [7., 9.]),
            2 => axis = piecewise([0., 0., 1.], [0., 0.2, 1.], [0., 0.5, 1.], [17., 19.]),
            3 => normal = piecewise([1., 0., 0.], [1., 0.2, 0.], [1., 1., 0.], [23., 29.]),
            4 => axes = piecewise([1.; 3], [1.3, 1., 1.], [2., 1., 1.], [31., 41.]),
            _ => center = piecewise([0.; 3], [0.1, 0., 0.], [0.5, 0., 0.], [43., 47.]),
        }
        if law_index == 6 {
            scale = piecewise([1., 0., 0.], [1.3, 0., 0.], [2., 0., 0.], [2., 5.]);
            twist = piecewise([0.; 3], [0.1, 0., 0.], [0.3, 0., 0.], [7., 9.]);
            axis = piecewise([0., 0., 1.], [0., 0.2, 1.], [0., 0.5, 1.], [17., 19.]);
            normal = piecewise([1., 0., 0.], [1., 0.2, 0.], [1., 1., 0.], [23., 29.]);
            axes = piecewise([1.; 3], [1.3, 1., 1.], [2., 1., 1.], [31., 41.]);
        }
        let sweep = Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, options)
            .unwrap()
            .with_affine_laws(&axes, &center)
            .unwrap();
        let report = sweep
            .authored_section_interpolation_bound(3, 10000)
            .unwrap();
        assert_eq!(
            report.status,
            Status::Certified,
            "law {law_index}: {:?}",
            report.reason
        );
        assert!(report.error_upper.unwrap().is_finite());
        let retained = sweep.sections(3).unwrap().0;
        let original = sweep.sections(101).unwrap().0;
        for (i, section) in original.iter().enumerate() {
            let t = i as f64 / 100.;
            let station = if t < 0.5 { 0 } else { 1 };
            let f = 2. * t - station as f64;
            for k in 0..2 {
                let delta: [f64; 3] = std::array::from_fn(|a| {
                    section.control_points[k][a]
                        - ((1. - f) * retained[station].control_points[k][a]
                            + f * retained[station + 1].control_points[k][a])
                });
                assert!(
                    norm(delta) <= report.error_upper.unwrap(),
                    "law {law_index}"
                );
            }
        }
        let refused = sweep.authored_section_interpolation_bound(3, 0).unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert!(refused.error_upper.is_none());
    }
}

#[test]
fn guided_initial_coordinates_own_translated_profile_before_twist_and_affine() {
    let mut profile = crate::primitives::line([1., 2., 3.], [2., 2., 3.]).unwrap();
    profile.weights = vec![1., 2.];
    let mut path = crate::primitives::line([0.5, 1., -1.], [0.5, 1., 9.]).unwrap();
    path.knots = vec![2., 2., 5., 5.];
    let mut guide = crate::primitives::line([0.5, 2., -1.], [0.5, 2., 9.]).unwrap();
    guide.knots = vec![31., 31., 41., 41.];
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.25, 0., 0.]).unwrap();
    let axes = constant_vector_law([1., 2., 3.]).unwrap();
    let center = constant_vector_law([0.5, 0., 0.]).unwrap();
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 1.,
    };
    let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap();
    let report = sweep.guided_initial_coordinates(100).unwrap();
    assert_eq!(report.status, Status::Certified);
    let coordinates = report.coordinates.unwrap();
    for (q, expected) in coordinates.iter().zip([[1., -0.5, 4.], [1., -1.5, 4.]]) {
        for k in 0..3 {
            assert!(q[k][0] <= expected[k] && expected[k] <= q[k][1]);
            assert!(q[k][1] - q[k][0] < 1e-10);
        }
    }
    let refused = sweep.guided_initial_coordinates(report.cells - 1).unwrap();
    assert_eq!(refused.status, Status::Unresolved);
    assert!(refused.coordinates.is_none());
}

pub(super) fn guided_control_value(
    sweep: &Sweep<'_>,
    profile_control: usize,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<authored_frame_certificate::ControlValueReport> {
    check(
        profile_control < sweep.profile.control_points.len(),
        "Profile control outside source",
    )?;
    check(
        max_cells <= 100000,
        "Guided control work exceeds100000 cells",
    )?;
    let unresolved = |cells, reason| authored_frame_certificate::ControlValueReport {
        status: Status::Unresolved,
        cells,
        value: None,
        reason: Some(reason),
    };
    let Some(guide) = sweep.orientation_guide else {
        return Ok(unresolved(0, "mode-not-guided"));
    };
    if sweep.options.spacing != Spacing::Parameter {
        return Ok(unresolved(0, "arc-length-correspondence-unproved"));
    }
    if sweep.contact_point.is_some() {
        return Ok(unresolved(0, "contact-width-law-unproved"));
    }
    let initial = guided_initial_coordinates(sweep, max_cells)?;
    if initial.status != Status::Certified {
        return Ok(unresolved(initial.cells, initial.reason.unwrap()));
    }
    let mut report = authored_frame_certificate::certify_path_guide_control_value(
        sweep.path,
        guide,
        sweep.scale,
        sweep.twist,
        sweep.affine_laws,
        initial.coordinates.unwrap()[profile_control],
        traversal,
        max_cells - initial.cells,
    )?;
    report.cells += initial.cells;
    Ok(report)
}

#[test]
fn guided_control_owns_initial_coordinates_and_whole_request_budget() {
    let profile = crate::primitives::line([1., 2., 3.], [2., 2., 3.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let guide = crate::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.25, 0., 0.]).unwrap();
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 1.,
    };
    let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
    let report = sweep.guided_control_value(0, [0.25, 0.5], 1000).unwrap();
    assert_eq!(report.status, Status::Certified);
    let value = report.value.unwrap();
    for t in [0.25, 0.375, 0.5] {
        let expected = [
            0.25_f64.cos() - 2. * 0.25_f64.sin(),
            0.25_f64.sin() + 2. * 0.25_f64.cos(),
            3. + 10. * t,
        ];
        for k in 0..3 {
            assert!(value[k][0] <= expected[k] && expected[k] <= value[k][1]);
        }
    }
    let refused = sweep
        .guided_control_value(0, [0.25, 0.5], report.cells - 1)
        .unwrap();
    assert_eq!(refused.status, Status::Unresolved);
    assert!(refused.value.is_none());
    assert!(sweep.guided_control_value(2, [0.25, 0.5], 1000).is_err());
}

#[test]
fn guided_control_does_not_promote_contact_or_arc_length_correspondence() {
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let guide = crate::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 1.,
    };
    let contact = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .with_contact_guide(&guide, 0.)
        .unwrap();
    let report = contact.guided_control_value(0, [0., 1.], 1000).unwrap();
    assert_eq!(report.reason, Some("contact-width-law-unproved"));
    assert!(report.value.is_none() && report.cells == 0);
    let arc_options = super::Options {
        spacing: Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 1000,
        },
        ..options
    };
    let arc = Sweep::new(&profile, &path, &scale, &twist, arc_options)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
    let report = arc.guided_control_value(0, [0., 1.], 1000).unwrap();
    assert_eq!(report.reason, Some("arc-length-correspondence-unproved"));
    assert!(report.value.is_none() && report.cells == 0);
}

pub(super) fn guided_section_interpolation(
    sweep: &Sweep<'_>,
    count: usize,
    max_cells: usize,
) -> Result<SectionInterpolationReport> {
    guided_section_interpolation_mode(sweep,count,max_cells,false)
}

pub(super) fn contact_section_interpolation(sweep: &Sweep<'_>,count: usize,max_cells:usize)->Result<SectionInterpolationReport> {
    guided_section_interpolation_mode(sweep,count,max_cells,true)
}

fn guided_section_interpolation_mode(
    sweep: &Sweep<'_>, count: usize, max_cells: usize, contact: bool,
) -> Result<SectionInterpolationReport> {
    check(
        count >= sweep.options.initial_sections && count <= sweep.options.max_sections,
        "Section count outside configured sweep",
    )?;
    check(
        max_cells <= 100000,
        "Guided section work exceeds100000 cells",
    )?;
    let mut out = SectionInterpolationReport {
        status: Status::Unresolved,
        cells: 0,
        error_upper: None,
        endpoint_displacement_upper: None,
        reason: Some("mode-not-guided"),
    };
    let Some(guide) = sweep.orientation_guide else {
        return Ok(out);
    };
    if sweep.options.spacing != Spacing::Parameter {
        out.reason = Some("arc-length-correspondence-unproved");
        return Ok(out);
    }
    // A guide supplies every normal directly: sections apply no RMF closure
    // correction. Endpoint displacement below also encloses the copied seam.
    if sweep.contact_point.is_some() && !contact {
        out.reason = Some("contact-width-law-unproved");
        return Ok(out);
    }
    let anchor = if contact {
        if sweep.contact_source.is_none() {out.reason=Some("mode-not-contact");return Ok(out);}
        let report=sweep.contact_anchor_bound(max_cells)?;
        out.cells+=report.cells;
        if report.status!=Status::Certified {out.reason=report.reason;return Ok(out);}
        Some(report.coordinates.unwrap()[0])
    } else {None};
    let certify_value=|q, interval, budget| {
        if let Some(anchor)=anchor {
            authored_frame_certificate::certify_contact_control_value(sweep.path,guide,sweep.scale,sweep.twist,sweep.affine_laws,q,anchor,interval,budget)
        } else {
            authored_frame_certificate::certify_path_guide_control_value(sweep.path,guide,sweep.scale,sweep.twist,sweep.affine_laws,q,interval,budget)
        }
    };
    let certify_jet=|q, interval, budget| {
        if let Some(anchor)=anchor {
            authored_frame_certificate::certify_contact_control_trajectory(sweep.path,guide,sweep.scale,sweep.twist,sweep.affine_laws,q,anchor,interval,budget)
        } else {
            authored_frame_certificate::certify_path_guide_control_trajectory(sweep.path,guide,sweep.scale,sweep.twist,sweep.affine_laws,q,interval,budget)
        }
    };
    let initial = guided_initial_coordinates(sweep, max_cells-out.cells)?;
    out.cells += initial.cells;
    if initial.status != Status::Certified {
        out.reason = initial.reason;
        return Ok(out);
    }
    let coordinates = initial.coordinates.unwrap();
    let sections = sweep.sections(count)?.0;
    check(
        sections.iter().all(|c| {
            c.degree == sweep.profile.degree
                && c.knots == sweep.profile.knots
                && c.weights == sweep.profile.weights
                && c.periodic == sweep.profile.periodic
                && c.control_points.len() == coordinates.len()
        }),
        "Retained guided basis correspondence changed",
    )?;
    let mut endpoint_errors = vec![vec![0.; coordinates.len()]; count];
    let mut endpoint_upper = 0_f64;
    for (station, section) in sections.iter().enumerate() {
        let t = station as f64 / (count - 1) as f64;
        for (control, &q) in coordinates.iter().enumerate() {
            let value = certify_value(q, [t, t], max_cells-out.cells)?;
            out.cells += value.cells;
            let p = &section.control_points[control];
            let Some(upper) = value.retained_displacement_upper([p[0], p[1], p[2]])? else {
                out.reason = value.reason;
                return Ok(out);
            };
            endpoint_errors[station][control] = upper;
            endpoint_upper = endpoint_upper.max(upper);
        }
    }
    let mut error = 0_f64;
    for station in 0..count - 1 {
        let interval = [
            station as f64 / (count - 1) as f64,
            (station + 1) as f64 / (count - 1) as f64,
        ];
        for (control, &q) in coordinates.iter().enumerate() {
            let jet = certify_jet(q, interval, max_cells-out.cells)?;
            out.cells += jet.cells;
            let upper = if let Some(upper) = jet.linear_error_upper([
                endpoint_errors[station][control],
                endpoint_errors[station + 1][control],
            ])? {
                upper
            } else {
                let value = certify_value(q, interval, max_cells-out.cells)?;
                out.cells += value.cells;
                let a = &sections[station].control_points[control];
                let b = &sections[station + 1].control_points[control];
                let Some(upper) = value.retained_segment_displacement_upper([
                    [a[0], a[1], a[2]],
                    [b[0], b[1], b[2]],
                ])?
                else {
                    out.reason = value.reason;
                    return Ok(out);
                };
                upper
            };
            error = error.max(upper);
        }
    }
    out.endpoint_displacement_upper = Some(endpoint_upper);
    out.status = Status::Certified;
    out.error_upper = Some(error);
    out.reason = None;
    Ok(out)
}

#[test]
fn guided_section_value_bound_owns_retained_segments_and_refines() {
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let guide = crate::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 9,
        max_deviation: 100.,
    };
    let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
    let coarse = sweep.guided_section_interpolation_bound(3, 10000).unwrap();
    let fine = sweep.guided_section_interpolation_bound(9, 10000).unwrap();
    assert_eq!(coarse.status, Status::Certified);
    assert_eq!(fine.status, Status::Certified);
    assert!(coarse.error_upper.unwrap() < 1e-9);
    assert!(fine.error_upper.unwrap() < 1e-9);
    assert!(coarse.endpoint_displacement_upper.unwrap() < 1e-10);
    let refused = sweep
        .guided_section_interpolation_bound(3, coarse.cells - 1)
        .unwrap();
    assert_eq!(refused.status, Status::Unresolved);
    assert!(refused.error_upper.is_none());
}

#[test]
fn contact_retained_section_bound_includes_station_rounding_and_joint_fit_remainder() {
    let mut profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    profile.weights=vec![1.,2.];
    let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let guide=crate::primitives::line([2.,0.,0.],[2.,0.,10.]).unwrap();
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let axes=crate::primitives::line([1.;3],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.5,0.,0.]).unwrap();
    let options=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,initial_sections:3,max_sections:33,max_deviation:0.01};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_contact_guide(&guide,1.).unwrap().with_affine_laws(&axes,&center).unwrap();
    let coarse=sweep.contact_section_interpolation_bound(3,10000).unwrap();
    let fine=sweep.contact_section_interpolation_bound(33,10000).unwrap();
    assert_eq!(coarse.status,Status::Certified);
    assert_eq!(fine.status,Status::Certified);
    assert!(fine.error_upper.unwrap()<coarse.error_upper.unwrap());
    assert!(fine.error_upper.unwrap()<0.01,"{:?}",fine.error_upper);
    assert!(fine.endpoint_displacement_upper.unwrap()<1e-9);
    let patch=sweep.contact_patch_error_bound(33,10000,1000).unwrap();
    assert_eq!(patch.status,Status::Certified);
    assert!(patch.within_budget);
    let level=sweep.level(33).unwrap();
    assert!(level.report.accepted && level.report.continuous_bound);
    assert_eq!(level.report.continuous_error_upper,patch.error_upper);
    assert_eq!(level.report.known_profile_error_upper,patch.error_upper);
    let coarse_level=sweep.level(3).unwrap();
    assert!(coarse_level.report.continuous_bound);
    assert!(!coarse_level.report.accepted);
    let retained=sweep.sections(3).unwrap().0;
    for t in [0.0_f64,0.13,0.375,0.5,0.87,1.] {
        let station=if t<0.5 {0} else {1};
        let f=2.*t-station as f64;
        for control in 0..2 {
            let q=1.+control as f64;
            let ideal=[2.*(q*(1.+t).powi(2)+0.5*t)/(2.*(1.+t).powi(2)+0.5*t),0.,10.*t];
            let delta: [f64;3]=std::array::from_fn(|k|(1.-f)*retained[station].control_points[control][k]+f*retained[station+1].control_points[control][k]-ideal[k]);
            assert!(norm(delta)<=coarse.error_upper.unwrap());
        }
    }
    let short=sweep.contact_section_interpolation_bound(33,fine.cells-1).unwrap();
    assert_eq!(short.status,Status::Unresolved);
    assert!(short.error_upper.is_none() && short.endpoint_displacement_upper.is_none());
}

#[test]
fn guided_dense_profile_patch_bound_composes_original_decomposition() {
    let profile = Curve {
        degree: 1,
        knots: std::iter::once(0.)
            .chain((0..=32).map(|i| i as f64))
            .chain(std::iter::once(32.))
            .collect(),
        control_points: (0..=32)
            .map(|i| vec![1. + i as f64 / 32., (i % 2) as f64 / 64., 0.])
            .collect(),
        weights: (0..=32).map(|i| if i % 2 == 0 { 1. } else { 2. }).collect(),
        periodic: false,
    };
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let guide = crate::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.25, 0., 0.]).unwrap();
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 100.,
    };
    let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
    let report = sweep.guided_patch_error_bound(3, 10000, 1000).unwrap();
    assert_eq!(report.status, Status::Certified);
    assert_eq!(report.products, 384);
    assert_eq!(report.patches.as_ref().unwrap().len(), 32);
    assert!(report.decomposition_error_upper.unwrap() > 0.);
    assert!(report.error_upper.unwrap() >= report.section_error_upper.unwrap());
    assert!(report.within_budget);
    let refused = sweep.guided_patch_error_bound(3, 10000, 383).unwrap();
    assert_eq!(refused.status, Status::Unresolved);
    assert!(refused.error_upper.is_none() && refused.patches.is_none());
    assert!(refused.products <= 383);
    let zero_twist = constant_vector_law([0.; 3]).unwrap();
    let contact = Sweep::new(&profile, &path, &scale, &zero_twist, options)
        .unwrap().with_contact_guide(&guide, 0.).unwrap();
    let fitted = contact.contact_patch_error_bound(3, 10000, 1000).unwrap();
    assert_eq!(fitted.status, Status::Certified);
    assert_eq!(fitted.products, 384);
    assert_eq!(fitted.patches.as_ref().unwrap().len(), 32);
    assert!(fitted.decomposition_error_upper.unwrap() > 0.);
    assert!(fitted.error_upper.unwrap() >= fitted.section_error_upper.unwrap());
    assert!(fitted.within_budget);
    let limited = contact.contact_patch_error_bound(3, 10000, 383).unwrap();
    assert_eq!(limited.status, Status::Unresolved);
    assert!(limited.patches.is_none() && limited.error_upper.is_none());
    assert!(limited.products <= 383);
}

#[test]
fn guided_original_rational_stations_have_point_safe_control_enclosures() {
    let profile = crate::primitives::line([1., 2., 3.], [2., 2., 3.]).unwrap();
    let mut path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    path.weights = vec![1., 2.];
    path.knots = vec![2., 2., 5., 5.];
    let guide = crate::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = crate::primitives::line([0.; 3], [0.25, 0., 0.]).unwrap();
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 1.,
    };
    let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
    for t in [0.13_f64, 0.375, 0.5, 0.87] {
        let report = sweep.guided_control_value(0, [t, t], 1000).unwrap();
        assert_eq!(report.status, Status::Certified);
        let expected = [
            (0.25 * t).cos() - 2. * (0.25 * t).sin(),
            (0.25 * t).sin() + 2. * (0.25 * t).cos(),
            3. + 20. * t / (1. + t),
        ];
        let value = report.value.unwrap();
        for k in 0..3 {
            assert!(value[k][0] <= expected[k] && expected[k] <= value[k][1]);
            assert!(value[k][1] - value[k][0] < 1e-9);
        }
        let refused = sweep
            .guided_control_value(0, [t, t], report.cells - 1)
            .unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert!(refused.value.is_none());
    }
}

#[test]
fn guided_tight_joint_law_patch_error_refines_to_small_tolerance() {
    let mut profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    profile.weights = vec![1., 2.];
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let guide = crate::primitives::line([1., 0., 0.], [1., 1., 10.]).unwrap();
    let scale = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let twist = crate::primitives::line([0.; 3], [0.25, 0., 0.]).unwrap();
    let axes = crate::primitives::line([1.; 3], [2., 1., 1.]).unwrap();
    let center = crate::primitives::line([0.; 3], [0.5, 0., 0.]).unwrap();
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 33,
        max_deviation: 0.01,
    };
    let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap();
    let coarse = sweep.guided_patch_error_bound(3, 10000, 1000).unwrap();
    let fine = sweep.guided_patch_error_bound(33, 10000, 1000).unwrap();
    assert_eq!(coarse.status, Status::Certified);
    assert_eq!(fine.status, Status::Certified);
    assert!(fine.error_upper.unwrap() < coarse.error_upper.unwrap());
    assert!(fine.within_budget, "fine bound {:?}", fine.error_upper);
    let coarse_level = sweep.level(3).unwrap();
    assert!(coarse_level.report.continuous_bound);
    assert_eq!(coarse_level.report.continuous_error_upper, coarse.error_upper);
    assert!(!coarse_level.report.accepted);
    let fine_level = sweep.level(33).unwrap();
    assert!(fine_level.report.accepted);
    assert!(fine_level.report.continuous_bound);
    assert_eq!(fine_level.report.continuous_error_upper, fine.error_upper);
    assert_eq!(fine_level.report.known_profile_error_upper, fine.error_upper);
    assert_eq!(fine_level.report.error_certificate_cells, fine.cells);
    assert_eq!(fine_level.report.decomposition_products, fine.products);
    let exhausted = sweep.level_with_error_budget(33, 0, 1000).unwrap();
    assert!(!exhausted.report.continuous_bound);
    assert!(exhausted.report.continuous_error_upper.is_none());
    let mut refining = sweep;
    let mut levels = Vec::new();
    while let Some(level) = refining.next() {
        levels.push(level.unwrap().report);
    }
    assert!(levels.len() > 1);
    assert!(levels.last().unwrap().accepted);
    assert!(levels.iter().all(|report| report.continuous_bound));
    let patch = &coarse.patches.as_ref().unwrap()[0];
    for t in [0.0_f64, 0.13, 0.375, 0.5, 0.87, 1.] {
        for u in [0., 0.375, 1.] {
            let q = (1. + 3. * u) / (1. + u);
            let amplitude = q * (1. + t).powi(2) + 0.5 * t;
            let angle = t.atan() + 0.25 * t;
            let ideal = [amplitude * angle.cos(), amplitude * angle.sin(), 10. * t];
            let got = patch.evaluate(u, t).unwrap().point;
            let delta: [f64; 3] = std::array::from_fn(|k| got[k] - ideal[k]);
            assert!(norm(delta) <= coarse.error_upper.unwrap());
        }
    }
}


#[test]
fn closed_guided_and_contact_retained_bounds_include_copied_seam() {
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],5.).unwrap();
    let guide=crate::primitives::circle([0.,0.,1.],[0.,0.,1.],5.).unwrap();
    let profile=crate::primitives::line([5.,0.,1.],[5.,0.,2.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let opts=Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,initial_sections:5,max_sections:33,max_deviation:100.};
    for contact in [false,true] {
        let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap();
        let sweep=if contact {sweep.with_contact_guide(&guide,0.).unwrap()} else {sweep.with_orientation_guide(&guide).unwrap()};
        let sections=sweep.sections(33).unwrap().0;
        assert_eq!(sections.first().unwrap().control_points,sections.last().unwrap().control_points);
        let bound=if contact {sweep.contact_patch_error_bound(33,100000,1000)} else {sweep.guided_patch_error_bound(33,100000,1000)}.unwrap();
        assert_eq!(bound.status,Status::Certified,"contact={contact}: {bound:?}");
        assert!(bound.error_upper.unwrap().is_finite());
        for i in 0..32 {
            for fraction in [0.125,0.5,0.875] {
                let t=(i as f64+fraction)/32.;
                let p=path.evaluate(path.domain()[0]+t*(path.domain()[1]-path.domain()[0])).unwrap().point;
                for control in 0..2 {
                    let expected=[p[0],p[1],1.+control as f64];
                    let delta=std::array::from_fn(|k|(1.-fraction)*sections[i].control_points[control][k]+fraction*sections[i+1].control_points[control][k]-expected[k]);
                    assert!(norm(delta)<=bound.error_upper.unwrap());
                }
            }
        }

        let section=if contact {sweep.contact_section_interpolation_bound(33,100000)} else {sweep.guided_section_interpolation_bound(33,100000)}.unwrap();
        assert!(section.endpoint_displacement_upper.unwrap()<1e-8);
        let short=if contact {sweep.contact_patch_error_bound(33,bound.cells-1,1000)} else {sweep.guided_patch_error_bound(33,bound.cells-1,1000)}.unwrap();
        assert_eq!(short.status,Status::Unresolved);
        assert!(short.error_upper.is_none());
    }
}
