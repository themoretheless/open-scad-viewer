//! Affine transforms, parametric curves/surfaces, splines, formulas, patches, and primitives.
use super::*;

pub(super) fn try_dispatch(v: &Value, op: &str) -> Option<Result<Value>> {
    if let Some(res) = try_dispatch_affine_pipes_and_helicoids(v, op) {
        return Some(res);
    }
    if let Some(res) = try_dispatch_spirals_and_helices(v, op) {
        return Some(res);
    }
    if let Some(res) = try_dispatch_splines_and_section_repairs(v, op) {
        return Some(res);
    }
    try_dispatch_polynomials_and_quadrics(v, op)
}

fn try_dispatch_affine_pipes_and_helicoids(v: &Value, op: &str) -> Option<Result<Value>> {
    let res = match op {
        "curve_affine" => field::<curve::Curve>(v, "curve")
            .and_then(|c| Ok(affine::curve(&c, &field(v, "matrix")?)?))
            .and_then(encode),
        "surface_affine" => field::<surface::Surface>(v, "surface")
            .and_then(|s| Ok(affine::surface(&s, &field(v, "matrix")?)?))
            .and_then(encode),
        "patches_affine" => field::<Vec<surface::Surface>>(v, "patches")
            .and_then(|p| Ok(affine::patches(&p, &field(v, "matrix")?)?))
            .and_then(encode),
        "surface_circle_rectangle_transition" => handle_circle_rectangle_transition(v),
        "surface_ellipse_transition" => handle_ellipse_transition(v),
        "surface_circle_transition" => handle_circle_transition(v),
        "surface_ribbon" => (|| {
            encode(ribbon::checked(
                &field::<curve::Curve>(v, "path")?,
                &field::<curve::Curve>(v, "width")?,
                field(v, "normal")?,
                field(v, "sections")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "surface_variable_pipe" => (|| {
            encode(pipe::checked_variable(
                &field::<curve::Curve>(v, "path")?,
                &field::<curve::Curve>(v, "radius")?,
                field(v, "normal")?,
                field(v, "sections")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "surface_pipe" => (|| {
            encode(pipe::checked(
                &field::<curve::Curve>(v, "path")?,
                field(v, "radius")?,
                field(v, "normal")?,
                field(v, "sections")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "surface_catenoid_patches" => (|| {
            encode(catenoid::approximate_patches(
                field(v, "center")?,
                field(v, "scale")?,
                field(v, "start_z")?,
                field(v, "end_z")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "surface_screw" => (|| {
            encode(screw_surface::approximate(
                &field::<curve::Curve>(v, "profile")?,
                field(v, "origin")?,
                field(v, "axis")?,
                field(v, "height")?,
                field(v, "turns")?,
                field(v, "phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_clothoid" => (|| {
            encode(clothoid::approximate(
                field(v, "center")?,
                field(v, "length")?,
                field(v, "start_curvature")?,
                field(v, "end_curvature")?,
                field(v, "phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_spherical_spiral" => (|| {
            encode(spherical_spiral::approximate(
                field(v, "center")?,
                field(v, "radius")?,
                field(v, "longitude_turns")?,
                field(v, "latitude_turns")?,
                field(v, "longitude_phase_degrees")?,
                field(v, "latitude_phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "surface_extrude_patches" => (|| {
            encode(extrusion_patches::extrude(
                &field::<curve::Curve>(v, "curve")?,
                field(v, "vector")?,
            )?)
        })(),
        "curve_toroidal_spiral" => (|| {
            encode(toroidal_spiral::approximate(
                toroidal_spiral::Torus { center: field(v, "center")?, major_radius: field(v, "major_radius")?, minor_radius: field(v, "minor_radius")? },
                toroidal_spiral::TorusWinding { major_turns: field(v, "major_turns")?, minor_turns: field(v, "minor_turns")?, major_phase_degrees: field(v, "major_phase_degrees")?, minor_phase_degrees: field(v, "minor_phase_degrees")? },
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_torus_knot" => (|| {
            encode(toroidal_spiral::approximate_knot(
                toroidal_spiral::Torus { center: field(v, "center")?, major_radius: field(v, "major_radius")?, minor_radius: field(v, "minor_radius")? },
                field(v, "p")?,
                field(v, "q")?,
                [field(v, "major_phase_degrees")?, field(v, "minor_phase_degrees")?],
                field(v, "max_deviation")?,
            )?)
        })(),
        "surface_helicoid_patches" => (|| {
            encode(helicoid::approximate_patches(
                field(v, "center")?,
                field(v, "inner_radius")?,
                field(v, "outer_radius")?,
                field(v, "height")?,
                field(v, "turns")?,
                field(v, "phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "surface_helicoid" => (|| {
            encode(helicoid::approximate(
                field(v, "center")?,
                field(v, "inner_radius")?,
                field(v, "outer_radius")?,
                field(v, "height")?,
                field(v, "turns")?,
                field(v, "phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "surface_catenoid" => (|| {
            encode(catenoid::approximate(
                field(v, "center")?,
                field(v, "scale")?,
                field(v, "start_z")?,
                field(v, "end_z")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        _ => return None,
    };
    Some(res)
}

fn try_dispatch_spirals_and_helices(v: &Value, op: &str) -> Option<Result<Value>> {
    let res = match op {
        "curve_catenary" => (|| {
            encode(catenary::approximate(
                field(v, "center")?,
                field(v, "scale")?,
                field(v, "start_x")?,
                field(v, "end_x")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_archimedean_spiral" => (|| {
            encode(archimedean_spiral::approximate(
                field(v, "center")?,
                field(v, "start_radius")?,
                field(v, "end_radius")?,
                field(v, "start_degrees")?,
                field(v, "end_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_epicycloid" => (|| {
            encode(circular_rolling::approximate_epicycloid(
                field(v, "center")?,
                field(v, "fixed_radius")?,
                field(v, "rolling_radius")?,
                field(v, "start_radians")?,
                field(v, "end_radians")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_hypocycloid" => (|| {
            encode(circular_rolling::approximate_hypocycloid(
                field(v, "center")?,
                field(v, "fixed_radius")?,
                field(v, "rolling_radius")?,
                field(v, "start_radians")?,
                field(v, "end_radians")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_trochoid" => (|| {
            encode(trochoid::approximate(
                field(v, "center")?,
                field(v, "rolling_radius")?,
                field(v, "tracing_radius")?,
                field(v, "start_radians")?,
                field(v, "end_radians")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_cycloid" => (|| {
            encode(trochoid::approximate_cycloid(
                field(v, "center")?,
                field(v, "radius")?,
                field(v, "start_radians")?,
                field(v, "end_radians")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_lissajous" => (|| {
            encode(lissajous::approximate(
                field(v, "center")?,
                field(v, "amplitudes")?,
                field(v, "frequencies")?,
                field(v, "phases_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_logarithmic_spiral" => (|| {
            encode(logarithmic_spiral::approximate(
                field(v, "center")?,
                field(v, "radius")?,
                field(v, "growth")?,
                field(v, "start_radians")?,
                field(v, "end_radians")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_involute" => (|| {
            encode(involute::approximate(
                field(v, "center")?,
                field(v, "radius")?,
                field(v, "start_radians")?,
                field(v, "end_radians")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_variable_pitch_helix" => (|| {
            encode(helix::approximate_variable_pitch(
                field(v, "center")?,
                field(v, "radius")?,
                field(v, "height")?,
                field(v, "turns")?,
                field(v, "start_pitch")?,
                field(v, "end_pitch")?,
                field(v, "phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_elliptic_helix" => (|| {
            encode(helix::approximate_elliptic(
                field(v, "center")?,
                field(v, "radius_x")?,
                field(v, "radius_y")?,
                field(v, "height")?,
                field(v, "turns")?,
                field(v, "phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_conical_helix" => (|| {
            encode(helix::approximate_conical(
                field(v, "center")?,
                field(v, "start_radius")?,
                field(v, "end_radius")?,
                field(v, "height")?,
                field(v, "turns")?,
                field(v, "phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_helix" => (|| {
            encode(helix::approximate(
                field(v, "center")?,
                field(v, "radius")?,
                field(v, "height")?,
                field(v, "turns")?,
                field(v, "phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        _ => return None,
    };
    Some(res)
}

fn try_dispatch_splines_and_section_repairs(v: &Value, op: &str) -> Option<Result<Value>> {
    let res = match op {
        "surface_boundary_fill" => (|| {
            encode(boundary_fill::fan(
                &field::<Vec<curve::Curve>>(v, "boundaries")?,
                field(v, "center")?,
            )?)
        })(),
        "surface_triangular_patch" => (|| {
            encode(triangular_patch::patch(
                &field(v, "base")?,
                &field(v, "side_a")?,
                &field(v, "side_b")?,
            )?)
        })(),
        "surface_grid_spline" => (|| {
            encode(grid_spline::interpolate(
                &field::<Vec<Vec<[f64; 3]>>>(v, "points")?,
                &field::<Vec<f64>>(v, "parameters_u")?,
                &field::<Vec<f64>>(v, "parameters_v")?,
            )?)
        })(),
        "surface_hermite_patch" => (|| {
            encode(hermite_patch::patch(
                field(v, "corners")?,
                field(v, "tangent_u")?,
                field(v, "tangent_v")?,
                field(v, "twist")?,
            )?)
        })(),
        "curve_closed_spline" => (|| {
            encode(closed_spline::interpolate(
                &field::<Vec<[f64; 3]>>(v, "points")?,
                &field::<Vec<f64>>(v, "parameters")?,
            )?)
        })(),
        "curve_clamped_spline" => (|| {
            encode(natural_spline::clamped(
                &field::<Vec<[f64; 3]>>(v, "points")?,
                &field::<Vec<f64>>(v, "parameters")?,
                field(v, "start_tangent")?,
                field(v, "end_tangent")?,
            )?)
        })(),
        "curve_natural_spline" => (|| {
            encode(natural_spline::interpolate(
                &field::<Vec<[f64; 3]>>(v, "points")?,
                &field::<Vec<f64>>(v, "parameters")?,
            )?)
        })(),
        "curve_hermite" => (|| {
            encode(hermite::interpolate(
                &field::<Vec<[f64; 3]>>(v, "points")?,
                &field::<Vec<[f64; 3]>>(v, "tangents")?,
                &field::<Vec<f64>>(v, "parameters")?,
            )?)
        })(),
        "surface_formula" => (|| {
            encode(formula::surface(
                field(v, "domain")?,
                &formula::tokens(&field::<Vec<Value>>(v, "expressions")?, true)?,
            )?)
        })(),
        "curve_formula" => (|| {
            encode(formula::curve(
                field(v, "domain")?,
                &formula::tokens(&field::<Vec<Value>>(v, "expressions")?, false)?,
            )?)
        })(),
        "surface_sphere" => (|| encode(primitives::sphere(field(v, "center")?, field(v, "radius")?)?))(),
        "surface_cylinder" => (|| {
            encode(primitives::cylinder(
                field(v, "center")?,
                field(v, "radius")?,
                field(v, "height")?,
            )?)
        })(),
        "surface_cone" => (|| {
            encode(primitives::cone(
                field(v, "center")?,
                field(v, "radius")?,
                field(v, "height")?,
            )?)
        })(),
        "surface_bezier" => (|| encode(patches::bezier(field(v, "points")?, optional_field(v, "weights")?)?))(),
        "surface_bilinear" => (|| encode(patches::bilinear(field(v, "corners")?)?))(),
        "surface_plane" => (|| {
            encode(patches::plane(
                field(v, "origin")?,
                field(v, "axisU")?,
                field(v, "axisV")?,
            )?)
        })(),
        "curve_bezier" => (|| encode(paths::bezier(field(v, "points")?, optional_field(v, "weights")?)?))(),
        "curve_repair_circle_section" => (|| {
            let r = crate::section_circle_repair::repair(
                &field::<Vec<curve::Curve>>(v, "curves")?,
                field(v, "quantum")?,
                field(v, "tolerance")?,
                field(v, "maxWork")?,
            )?;
            Ok(json!({
                "curves": r.curves,
                "displacementUpper": r.displacement_upper,
                "work": r.work,
                "reason": r.reason
            }))
        })(),
        "curve_project_section_authored_axis" => (|| {
            let r = crate::section_projection::project_authored_axis(
                &field::<Vec<curve::Curve>>(v, "curves")?,
                &field::<curve::Curve>(v, "frameAxis")?,
                field(v, "traversal")?,
                field(v, "quantum")?,
                field(v, "tolerance")?,
                field(v, "maxWork")?,
            )?;
            Ok(json!({
                "curves": r.curves,
                "displacementUpper": r.displacement_upper,
                "exactPlanar": r.exact_planar,
                "work": r.work,
                "reason": r.reason
            }))
        })(),
        "curve_project_section" => (|| {
            let r = crate::section_projection::project(
                &field::<Vec<curve::Curve>>(v, "curves")?,
                field(v, "axis")?,
                field(v, "coefficients")?,
                field(v, "offset")?,
                field(v, "quantum")?,
                field(v, "tolerance")?,
                field(v, "maxWork")?,
            )?;
            Ok(json!({
                "curves": r.curves,
                "displacementUpper": r.displacement_upper,
                "exactPlanar": r.exact_planar,
                "work": r.work,
                "reason": r.reason
            }))
        })(),
        _ => return None,
    };
    Some(res)
}

fn try_dispatch_polynomials_and_quadrics(v: &Value, op: &str) -> Option<Result<Value>> {
    let res = match op {
        "curve_transition_polyline" => (|| {
            let closed = optional_field::<bool>(v, "closed")?.unwrap_or(false);
            let f = if closed { paths::closed_transition_polyline } else { paths::transition_polyline };
            encode(f(&field::<Vec<[f64; 3]>>(v, "points")?, field(v, "setback")?)?)
        })(),
        "curve_round_polyline" => (|| {
            let closed = optional_field::<bool>(v, "closed")?.unwrap_or(false);
            let f = if closed { paths::closed_round_polyline } else { paths::round_polyline };
            encode(f(&field::<Vec<[f64; 3]>>(v, "points")?, field(v, "radius")?)?)
        })(),
        "curve_compose" => (|| encode(paths::compose(&field::<Vec<curve::Curve>>(v, "curves")?)?))(),
        "curve_line" => (|| encode(primitives::line(field(v, "start")?, field(v, "end")?)?))(),
        "curve_polyline" => (|| {
            encode(primitives::polyline(
                &field::<Vec<[f64; 3]>>(v, "points")?,
                optional_field(v, "closed")?.unwrap_or(false),
            )?)
        })(),
        "curve_circle" => (|| {
            encode(primitives::circle(
                field(v, "center")?,
                field(v, "normal")?,
                field(v, "radius")?,
            )?)
        })(),
        "curve_circle_arc" => (|| {
            encode(primitives::circle_arc(
                field(v, "center")?,
                field(v, "normal")?,
                field(v, "radius")?,
                field(v, "startDegrees")?,
                field(v, "sweepDegrees")?,
            )?)
        })(),
        "curve_rational_polynomial" => (|| {
            encode(polynomial::rational_curve(
                field(v, "domain")?,
                &field::<Vec<[f64; 4]>>(v, "coefficients")?,
            )?)
        })(),
        "surface_rational_polynomial" => (|| {
            encode(polynomial::rational_surface(
                field(v, "domain")?,
                &field::<Vec<Vec<[f64; 4]>>>(v, "coefficients")?,
            )?)
        })(),
        "curve_polynomial_parametric" => (|| {
            encode(polynomial::parametric_curve(
                field(v, "domain")?,
                &field::<Vec<[f64; 3]>>(v, "coefficients")?,
            )?)
        })(),
        "surface_polynomial_parametric" => (|| {
            encode(polynomial::parametric_surface(
                field(v, "domain")?,
                &field::<Vec<Vec<[f64; 3]>>>(v, "coefficients")?,
            )?)
        })(),
        "surface_polynomial_graph" => (|| {
            encode(polynomial::graph(
                field(v, "bounds")?,
                &field::<Vec<Vec<f64>>>(v, "coefficients")?,
            )?)
        })(),
        "surface_hyperboloid_one_sheet" => (|| {
            encode(primitives::hyperboloid_one_sheet(
                field(v, "center")?,
                field(v, "radii")?,
                field(v, "start")?,
                field(v, "end")?,
            )?)
        })(),
        "surface_hyperboloid_two_sheet" => (|| {
            encode(primitives::hyperboloid_two_sheet(
                field(v, "center")?,
                field(v, "radii")?,
                field(v, "start")?,
                field(v, "end")?,
                optional_field(v, "lower")?.unwrap_or(false),
            )?)
        })(),
        "surface_quadratic_patch" => (|| {
            encode(primitives::quadratic_patch(
                field(v, "bounds")?,
                field(v, "coefficients")?,
            )?)
        })(),
        "curve_parabola" | "curve_hyperbola" => (|| {
            let constructor = if op == "curve_parabola" {
                primitives::parabola
            } else {
                primitives::hyperbola
            };
            encode(constructor(
                field(v, "center")?,
                field(v, "axisU")?,
                field(v, "axisV")?,
                field(v, "start")?,
                field(v, "end")?,
            )?)
        })(),
        "surface_elliptic_cylinder" => (|| {
            encode(primitives::elliptic_cylinder(
                field(v, "center")?,
                field(v, "radiusX")?,
                field(v, "radiusY")?,
                field(v, "height")?,
            )?)
        })(),
        "surface_cone_frustum" => (|| {
            encode(primitives::cone_frustum(
                field(v, "center")?,
                field(v, "bottomRadius")?,
                field(v, "topRadius")?,
                field(v, "height")?,
            )?)
        })(),
        "curve_ellipse_arc" => (|| {
            encode(primitives::ellipse_arc(
                field(v, "center")?,
                field(v, "axisU")?,
                field(v, "axisV")?,
                field(v, "startDegrees")?,
                field(v, "sweepDegrees")?,
            )?)
        })(),
        "surface_ellipsoid" => (|| {
            encode(primitives::ellipsoid(
                field(v, "center")?,
                field(v, "radii")?,
            )?)
        })(),
        "surface_torus" => (|| {
            encode(primitives::torus(
                field(v, "center")?,
                field(v, "majorRadius")?,
                field(v, "radialRadius")?,
                field(v, "axialRadius")?,
            )?)
        })(),
        _ => return None,
    };
    Some(res)
}

fn handle_circle_rectangle_transition(v: &Value) -> Result<Value> {
    let circle = circle_transition::CircleSection {
        center: field(v, "circle_center")?,
        normal: field(v, "circle_normal")?,
        seam: field(v, "circle_seam")?,
        radius: field(v, "circle_radius")?,
    };
    let rectangle = circle_rectangle_transition::RectangleSection {
        center: field(v, "rectangle_center")?,
        axis_u: field(v, "rectangle_axis_u")?,
        axis_v: field(v, "rectangle_axis_v")?,
    };
    encode(circle_rectangle_transition::ruled(&circle, &rectangle)?)
}

fn handle_ellipse_transition(v: &Value) -> Result<Value> {
    let start = ellipse_transition::EllipseSection {
        center: field(v, "start_center")?,
        axis_u: field(v, "start_axis_u")?,
        axis_v: field(v, "start_axis_v")?,
    };
    let end = ellipse_transition::EllipseSection {
        center: field(v, "end_center")?,
        axis_u: field(v, "end_axis_u")?,
        axis_v: field(v, "end_axis_v")?,
    };
    encode(ellipse_transition::ruled(&start, &end)?)
}

fn handle_circle_transition(v: &Value) -> Result<Value> {
    let start = circle_transition::CircleSection {
        center: field(v, "start_center")?,
        normal: field(v, "start_normal")?,
        seam: field(v, "start_seam")?,
        radius: field(v, "start_radius")?,
    };
    let end = circle_transition::CircleSection {
        center: field(v, "end_center")?,
        normal: field(v, "end_normal")?,
        seam: field(v, "end_seam")?,
        radius: field(v, "end_radius")?,
    };
    encode(circle_transition::ruled(&start, &end)?)
}
