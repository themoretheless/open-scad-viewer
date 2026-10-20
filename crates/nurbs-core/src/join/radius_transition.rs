//! G2 quarter-skin transition between two authored coaxial circular radii.
//! This is a local transition, not a general multi-edge corner or closed solid.
use crate::{
    Result, check,
    curve::Curve,
    interval_eval::Interval as I,
    skin_radius,
    surface::{Axis, Surface},
    surface_blend, surface_collar, surface_linear_monotonicity,
};
#[derive(Clone, Copy)]
pub struct Options {
    pub origin_mm: [f64; 3],
    pub axis: [f64; 3],
    pub radial: [f64; 3],
    pub radii_mm: [f64; 2],
    pub width_mm: f64,
    /// Position, unit normal and signed world curvature tensor.
    pub tolerances: [f64; 3],
    pub max_cells: usize,
    pub max_coefficients: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            origin_mm: [0.; 3],
            axis: [0., 0., 1.],
            radial: [1., 0., 0.],
            radii_mm: [0.1, 0.3],
            width_mm: 0.1,
            tolerances: [1e-7, 1e-5, 1e-4],
            max_cells: 100000,
            max_coefficients: 2000000,
        }
    }
}
pub struct Report {
    pub surface: Surface,
    pub original_supports: [Surface; 2],
    pub centerline: Curve,
    /// Authored quintic law: [r0,r0,r0,r1,r1,r1] in station V.
    pub radius_law: Curve,
    pub seams: Vec<surface_collar::G2Report>,
    pub original_radii: Vec<skin_radius::Report>,
    pub original_plane_error_upper_mm: Vec<f64>,
    pub original_circular_radius_error_upper_mm: Vec<Option<f64>>,
    pub radius: skin_radius::Report,
    pub station_plane_error_upper_mm: f64,
    /// Radius in the plane perpendicular to the authored axis, not only
    /// distance to the station center in 3D.
    pub circular_radius_error_upper_mm: Option<f64>,
    pub interior: surface_linear_monotonicity::Report,
    pub width_error_upper_mm: f64,
    pub cells: usize,
    pub coefficient_work: usize,
    pub admitted: bool,
}
fn normalize(v: [f64; 3]) -> Result<[f64; 3]> {
    let length = v[0].hypot(v[1]).hypot(v[2]);
    check(
        length.is_finite() && length > 0.,
        "Transition frame needs nonzero finite directions",
    )?;
    Ok(v.map(|x| x / length))
}
fn scalar(r: f64) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![r, 0., 0.]; 2],
        weights: vec![1.; 2],
        periodic: false,
    }
}
fn charge(used: &mut usize, max: usize, n: usize) -> Result<()> {
    if n > max.saturating_sub(*used) {
        return Err(crate::Error::new(
            "RADIUS_TRANSITION_WORK_LIMIT",
            "Transition plane/radius proof work exhausted",
        ));
    }
    *used += n;
    Ok(())
}
// Original homogeneous tensor coefficients of axis·(S-C(V)). Multiplication
// by the actual positive denominator and the linear center elevates V once.
// No rounded projected surface or sampled station grants planarity.
fn plane_bound(
    s: &Surface,
    c: &Curve,
    axis: [f64; 3],
    used: &mut usize,
    max: usize,
) -> Result<f64> {
    let n = s.degree_v;
    check(
        n >= 1
            && s.knots_v == [vec![0.; n + 1], vec![1.; n + 1]].concat()
            && c.degree == 1
            && c.control_points.len() == 2,
        "Station plane proof needs the original unit Bernstein chart and linear center",
    )?;
    charge(used, max, 16)?;
    let mut squared = I::point(0.);
    for v in axis {
        squared = squared.add(I::point(v).mul(I::point(v))?)?;
    }
    let axis_lower = squared.lo.max(0.).sqrt().next_down();
    check(axis_lower > 0., "Station plane axis is not regular")?;
    let weight_lower = s
        .weights
        .iter()
        .flatten()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let mut maximum = 0_f64;
    for i in 0..s.control_points.len() {
        for j in 0..=n + 1 {
            charge(used, max, 4)?;
            let alpha = I::point(j as f64).div(I::point((n + 1) as f64))?;
            let mut coefficient = I::point(0.);
            for (pole, center, factor) in [
                (j.checked_sub(1), 1, alpha),
                ((j <= n).then_some(j), 0, I::point(1.).sub(alpha)?),
            ] {
                let Some(pole) = pole else {
                    continue;
                };
                charge(used, max, 12)?;
                let mut value = I::point(0.);
                for k in 0..3 {
                    value = value.add(
                        I::point(s.control_points[i][pole][k])
                            .sub(I::point(c.control_points[center][k]))?
                            .mul(I::point(axis[k]))?,
                    )?;
                }
                coefficient =
                    coefficient.add(value.mul(I::point(s.weights[i][pole]))?.mul(factor)?)?;
            }
            maximum = maximum.max(coefficient.lo.abs().max(coefficient.hi.abs()));
        }
    }
    charge(used, max, 3)?;
    Ok(I::point(maximum)
        .div(I::point(weight_lower).mul(I::point(axis_lower))?)?
        .hi)
}
fn circular_error(
    radius_error: Option<f64>,
    minimum_radius: f64,
    plane: f64,
    used: &mut usize,
    max: usize,
) -> Result<Option<f64>> {
    let Some(error) = radius_error else {
        return Ok(None);
    };
    charge(used, max, 12)?;
    let length = I::point(minimum_radius).sub(I::point(error))?;
    if length.lo <= plane {
        return Ok(None);
    }
    let projected = length
        .mul(length)?
        .sub(I::point(plane).mul(I::point(plane))?)?;
    if projected.lo <= 0. {
        return Ok(None);
    }
    let lower = projected.lo.sqrt().next_down();
    Ok(Some(
        I::point(error)
            .add(
                I::point(plane)
                    .mul(I::point(plane))?
                    .div(I::point(length.lo).add(I::point(lower))?)?,
            )?
            .hi,
    ))
}
fn run(candidate: Option<&Surface>, o: Options) -> Result<Report> {
    check(
        o.origin_mm
            .iter()
            .chain(&o.axis)
            .chain(&o.radial)
            .all(|v| v.is_finite())
            && o.radii_mm.iter().all(|r| r.is_finite() && *r > 0.)
            && o.width_mm.is_finite()
            && o.width_mm > 0.
            && o.tolerances.iter().all(|t| t.is_finite() && *t > 0.)
            && o.tolerances[1] <= 2.
            && (1..=100000).contains(&o.max_cells)
            && (45..=2000000).contains(&o.max_coefficients),
        "Need positive transition radii/width, finite frame and bounded proof work",
    )?;
    let axis = normalize(o.axis)?;
    let radial = normalize(o.radial)?;
    let dot = (0..3).map(|k| axis[k] * radial[k]).sum::<f64>();
    let projected = std::array::from_fn(|k| radial[k] - dot * axis[k]);
    check(
        projected.iter().map(|v| v * v).sum::<f64>() > 1e-24,
        "Transition radial and axial directions are parallel",
    )?;
    let x = normalize(projected)?;
    let y = [
        axis[1] * x[2] - axis[2] * x[1],
        axis[2] * x[0] - axis[0] * x[2],
        axis[0] * x[1] - axis[1] * x[0],
    ];
    let point = |u: f64, v: f64, z: f64| {
        (0..3)
            .map(|k| o.origin_mm[k] + x[k] * u + y[k] * v + axis[k] * z)
            .collect::<Vec<_>>()
    };
    let center = |z: [f64; 2]| Curve::from_polyline(z.iter().map(|&z| point(0., 0., z)).collect());
    let centerline = center([0., o.width_mm])?;
    let mut length = I::point(0.);
    let mut axial = I::point(0.);
    let mut axis_squared = I::point(0.);
    for k in 0..3 {
        let d = I::point(centerline.control_points[1][k])
            .sub(I::point(centerline.control_points[0][k]))?;
        length = length.add(d.mul(d)?)?;
        axial = axial.add(d.mul(I::point(axis[k]))?)?;
        axis_squared = axis_squared.add(I::point(axis[k]).mul(I::point(axis[k]))?)?;
    }
    let length = I::new(
        length.lo.max(0.).sqrt().next_down().max(0.),
        length.hi.sqrt().next_up(),
    )?;
    let error = length.sub(I::point(o.width_mm))?;
    let axis_length = I::new(
        axis_squared.lo.max(0.).sqrt().next_down(),
        axis_squared.hi.sqrt().next_up(),
    )?;
    let spacing_error = axial.div(axis_length)?.sub(I::point(o.width_mm))?;
    let width_error_upper_mm = error
        .lo
        .abs()
        .max(error.hi.abs())
        .max(spacing_error.lo.abs().max(spacing_error.hi.abs()));
    check(
        width_error_upper_mm <= o.tolerances[0],
        "Requested transition width is not representable within position tolerance",
    )?;
    let support = |r: f64, z: [f64; 2]| Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [[r, 0.], [r, r], [0., r]]
            .iter()
            .map(|p| z.iter().map(|&z| point(p[0], p[1], z)).collect())
            .collect(),
        weights: vec![vec![1.; 2], vec![0.5_f64.sqrt(); 2], vec![1.; 2]],
        periodic_u: false,
        periodic_v: false,
    };
    let original_supports = [
        support(o.radii_mm[0], [-o.width_mm, 0.]),
        support(o.radii_mm[1], [o.width_mm, 2. * o.width_mm]),
    ];
    let surface = if let Some(surface) = candidate {
        surface.validate()?;
        surface.clone()
    } else {
        let mut built = surface_blend::g2(
            &original_supports[0],
            &original_supports[1],
            Default::default(),
        )?;
        check(
            built.patches.len() == 1,
            "Quarter transition proposal is not one original patch",
        )?;
        built.patches.remove(0)
    };
    let radius_law = Curve {
        degree: 5,
        knots: [vec![0.; 6], vec![1.; 6]].concat(),
        control_points: [o.radii_mm[0]; 3]
            .into_iter()
            .chain([o.radii_mm[1]; 3])
            .map(|r| vec![r, 0., 0.])
            .collect(),
        weights: vec![1.; 6],
        periodic: false,
    };
    let mut cells = 0;
    let mut coefficient_work = 44;
    check(
        o.max_coefficients > coefficient_work,
        "Transition width proof work exhausted",
    )?;
    let mut original_radii = Vec::new();
    let mut original_plane_error_upper_mm = Vec::new();
    let mut original_circular = Vec::new();
    for (i, z) in [[-o.width_mm, 0.], [o.width_mm, 2. * o.width_mm]]
        .into_iter()
        .enumerate()
    {
        let centerline = center(z)?;
        let proof = skin_radius::inspect(
            &original_supports[i],
            &centerline,
            &scalar(o.radii_mm[i]),
            Axis::V,
            o.tolerances[0],
            o.max_coefficients - coefficient_work,
        )?;
        coefficient_work += proof.coefficient_work;
        let plane = plane_bound(
            &original_supports[i],
            &centerline,
            axis,
            &mut coefficient_work,
            o.max_coefficients,
        )?;
        original_circular.push(circular_error(
            proof.radial_error_upper_mm,
            o.radii_mm[i],
            plane,
            &mut coefficient_work,
            o.max_coefficients,
        )?);
        original_plane_error_upper_mm.push(plane);
        original_radii.push(proof);
    }
    let mut seams = Vec::new();
    for (i, at, direction, reverse) in [(0, 1., [0., -1.], false), (1, 0., [0., 1.], true)] {
        check(
            cells < o.max_cells && coefficient_work < o.max_coefficients,
            "Transition seam work exhausted",
        )?;
        let pcurve = Curve::from_polyline(vec![vec![0., at], vec![1., at]])?;
        let actual = if reverse {
            surface.edit_axis(Axis::V, |c| c.reverse())?
        } else {
            surface.clone()
        };
        let proof = surface_collar::inspect_g2_opposite_cross(
            &original_supports[i],
            &pcurve,
            direction,
            &actual,
            o.tolerances,
            o.max_cells - cells,
            o.max_coefficients - coefficient_work,
        )?;
        cells += proof.cells;
        coefficient_work += proof.coefficient_work;
        seams.push(proof);
    }
    check(
        coefficient_work < o.max_coefficients && cells < o.max_cells,
        "Transition radius/interior work exhausted",
    )?;
    let radius = skin_radius::inspect(
        &surface,
        &centerline,
        &radius_law,
        Axis::V,
        o.tolerances[0],
        o.max_coefficients - coefficient_work,
    )?;
    coefficient_work += radius.coefficient_work;
    let station_plane_error_upper_mm = plane_bound(
        &surface,
        &centerline,
        axis,
        &mut coefficient_work,
        o.max_coefficients,
    )?;
    let circular_radius_error_upper_mm = circular_error(
        radius.radial_error_upper_mm,
        o.radii_mm[0].min(o.radii_mm[1]),
        station_plane_error_upper_mm,
        &mut coefficient_work,
        o.max_coefficients,
    )?;
    let interior = surface_linear_monotonicity::inspect_candidate(&surface, o.max_cells - cells)?;
    cells += interior.cells;
    let admitted = seams
        .iter()
        .all(|s| s.decision == crate::surface_join::Decision::WithinTolerance)
        && original_radii.iter().all(|r| r.within_tolerance)
        && original_plane_error_upper_mm
            .iter()
            .all(|&e| e <= o.tolerances[0])
        && original_circular
            .iter()
            .all(|e| e.is_some_and(|e| e <= o.tolerances[0]))
        && radius.within_tolerance
        && station_plane_error_upper_mm <= o.tolerances[0]
        && circular_radius_error_upper_mm.is_some_and(|e| e <= o.tolerances[0])
        && interior.certified;
    Ok(Report {
        surface,
        original_supports,
        centerline,
        radius_law,
        seams,
        original_radii,
        original_plane_error_upper_mm,
        original_circular_radius_error_upper_mm: original_circular,
        radius,
        station_plane_error_upper_mm,
        circular_radius_error_upper_mm,
        interior,
        width_error_upper_mm,
        cells,
        coefficient_work,
        admitted,
    })
}
/// Build one coaxial quarter skin, with whole original support G2, authored
/// cross-section radius-law and injectivity admission. No B-rep or general
/// rolling-ball envelope/corner certificate is implied.
pub fn build(o: Options) -> Result<Report> {
    run(None, o)
}
/// Freshly inspect actual supplied coefficients; no construction receipt is
/// accepted as input. The original supports/law come from authored options.
pub fn inspect_candidate(surface: &Surface, o: Options) -> Result<Report> {
    run(Some(surface), o)
}
