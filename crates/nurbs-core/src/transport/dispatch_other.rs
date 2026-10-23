use super::*;

pub(super) fn dispatch_other(v:Value,op:String)->Result<Value> {
    if matches!(op.as_str(), "sweep_error_upper_compose") { return sweep_requests::dispatch(v, op); }
    if matches!(op.as_str(), "curve_decomposition_audit") { return certificate_requests::dispatch(v, op); }
    if matches!(op.as_str(), "sweep_exact_seam_set_audit" | "sweep_projective_seam_set_audit") { return sweep_requests::dispatch(v, op); }
    if matches!(op.as_str(), "surface_auto_guided_loft" | "surface_auto_guided_loft_cartesian") { return loft_requests::dispatch(v, op); }
    if matches!(op.as_str(), "surface_guided_loft_cartesian") { return loft_requests::dispatch(v, op); }
    if matches!(op.as_str(), "surface_natural_loft_checked") { return loft_requests::dispatch(v, op); }
    if matches!(op.as_str(), "surface_boundary_v_tangent_certify") { return certificate_requests::dispatch(v, op); }
    if matches!(op.as_str(), "surface_gordon_cartesian") { return loft_requests::dispatch(v, op); }

    if op == "surface_progressive_sweep_spatial_rmf_error" || op == "surface_progressive_sweep" || op == "surface_progressive_sweep_profiles" || op=="surface_progressive_sweep_level" || op=="surface_progressive_sweep_cap_domains" || op=="surface_progressive_sweep_cap_projection" || op=="surface_progressive_sweep_station_seams" || op=="surface_progressive_sweep_profile_join" || op=="surface_progressive_sweep_decomposition_joins" || op=="surface_progressive_sweep_decomposition_smoothness" || op=="surface_progressive_sweep_frame_smoothness" || op=="surface_progressive_sweep_closed_frame_smoothness" || op=="surface_progressive_sweep_closed_path_frame_smoothness" || op=="surface_progressive_sweep_closed_guided_frame_smoothness" { return sweep_requests::dispatch(v, op); }
    if op=="curve_affine" {return encode(affine::curve(&field::<curve::Curve>(&v,"curve")?,&field(&v,"matrix")?)?);}
    if op=="surface_affine" {return encode(affine::surface(&field::<surface::Surface>(&v,"surface")?,&field(&v,"matrix")?)?);}
    if op=="patches_affine" {return encode(affine::patches(&field::<Vec<surface::Surface>>(&v,"patches")?,&field(&v,"matrix")?)?);}
    if op == "surface_circle_rectangle_transition" {
        let circle=circle_transition::CircleSection {center:field(&v,"circle_center")?,normal:field(&v,"circle_normal")?,seam:field(&v,"circle_seam")?,radius:field(&v,"circle_radius")?};
        let rectangle=circle_rectangle_transition::RectangleSection {center:field(&v,"rectangle_center")?,axis_u:field(&v,"rectangle_axis_u")?,axis_v:field(&v,"rectangle_axis_v")?};
        return encode(circle_rectangle_transition::ruled(&circle,&rectangle)?);
    }
    if op == "surface_ellipse_transition" {
        let start=ellipse_transition::EllipseSection {center:field(&v,"start_center")?,axis_u:field(&v,"start_axis_u")?,axis_v:field(&v,"start_axis_v")?};
        let end=ellipse_transition::EllipseSection {center:field(&v,"end_center")?,axis_u:field(&v,"end_axis_u")?,axis_v:field(&v,"end_axis_v")?};
        return encode(ellipse_transition::ruled(&start,&end)?);
    }
    if op == "surface_circle_transition" {
        let start=circle_transition::CircleSection {center:field(&v,"start_center")?,normal:field(&v,"start_normal")?,seam:field(&v,"start_seam")?,radius:field(&v,"start_radius")?};
        let end=circle_transition::CircleSection {center:field(&v,"end_center")?,normal:field(&v,"end_normal")?,seam:field(&v,"end_seam")?,radius:field(&v,"end_radius")?};
        return encode(circle_transition::ruled(&start,&end)?);
    }
    if op == "surface_ribbon" {return encode(ribbon::checked(&field::<curve::Curve>(&v,"path")?,&field::<curve::Curve>(&v,"width")?,field(&v,"normal")?,field(&v,"sections")?,field(&v,"max_deviation")?)?);}
    if op == "surface_variable_pipe" {return encode(pipe::checked_variable(&field::<curve::Curve>(&v,"path")?,&field::<curve::Curve>(&v,"radius")?,field(&v,"normal")?,field(&v,"sections")?,field(&v,"max_deviation")?)?);}
    if op == "surface_pipe" {return encode(pipe::checked(&field::<curve::Curve>(&v,"path")?,field(&v,"radius")?,field(&v,"normal")?,field(&v,"sections")?,field(&v,"max_deviation")?)?);}
    if op == "surface_catenoid_patches" {return encode(catenoid::approximate_patches(field(&v,"center")?,field(&v,"scale")?,field(&v,"start_z")?,field(&v,"end_z")?,field(&v,"max_deviation")?)?);}
    if op == "surface_screw" {return encode(screw_surface::approximate(&field::<curve::Curve>(&v,"profile")?,field(&v,"origin")?,field(&v,"axis")?,field(&v,"height")?,field(&v,"turns")?,field(&v,"phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "curve_clothoid" {return encode(clothoid::approximate(field(&v,"center")?,field(&v,"length")?,field(&v,"start_curvature")?,field(&v,"end_curvature")?,field(&v,"phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "curve_spherical_spiral" {return encode(spherical_spiral::approximate(field(&v,"center")?,field(&v,"radius")?,field(&v,"longitude_turns")?,field(&v,"latitude_turns")?,field(&v,"longitude_phase_degrees")?,field(&v,"latitude_phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "surface_extrude_patches" {return encode(extrusion_patches::extrude(&field::<curve::Curve>(&v,"curve")?,field(&v,"vector")?)?);}
    if op == "curve_toroidal_spiral" {return encode(toroidal_spiral::approximate(field(&v,"center")?,field(&v,"major_radius")?,field(&v,"minor_radius")?,field(&v,"major_turns")?,field(&v,"minor_turns")?,field(&v,"major_phase_degrees")?,field(&v,"minor_phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "curve_torus_knot" {return encode(toroidal_spiral::approximate_knot(field(&v,"center")?,field(&v,"major_radius")?,field(&v,"minor_radius")?,field(&v,"p")?,field(&v,"q")?,field(&v,"major_phase_degrees")?,field(&v,"minor_phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "surface_helicoid_patches" {return encode(helicoid::approximate_patches(field(&v,"center")?,field(&v,"inner_radius")?,field(&v,"outer_radius")?,field(&v,"height")?,field(&v,"turns")?,field(&v,"phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "surface_helicoid" {return encode(helicoid::approximate(field(&v,"center")?,field(&v,"inner_radius")?,field(&v,"outer_radius")?,field(&v,"height")?,field(&v,"turns")?,field(&v,"phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "surface_catenoid" {return encode(catenoid::approximate(field(&v,"center")?,field(&v,"scale")?,field(&v,"start_z")?,field(&v,"end_z")?,field(&v,"max_deviation")?)?);}
    if op == "curve_catenary" {return encode(catenary::approximate(field(&v,"center")?,field(&v,"scale")?,field(&v,"start_x")?,field(&v,"end_x")?,field(&v,"max_deviation")?)?);}
    if op == "curve_archimedean_spiral" {return encode(archimedean_spiral::approximate(field(&v,"center")?,field(&v,"start_radius")?,field(&v,"end_radius")?,field(&v,"start_degrees")?,field(&v,"end_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "curve_epicycloid" {return encode(circular_rolling::approximate_epicycloid(field(&v,"center")?,field(&v,"fixed_radius")?,field(&v,"rolling_radius")?,field(&v,"start_radians")?,field(&v,"end_radians")?,field(&v,"max_deviation")?)?);}
    if op == "curve_hypocycloid" {return encode(circular_rolling::approximate_hypocycloid(field(&v,"center")?,field(&v,"fixed_radius")?,field(&v,"rolling_radius")?,field(&v,"start_radians")?,field(&v,"end_radians")?,field(&v,"max_deviation")?)?);}
    if op == "curve_trochoid" {return encode(trochoid::approximate(field(&v,"center")?,field(&v,"rolling_radius")?,field(&v,"tracing_radius")?,field(&v,"start_radians")?,field(&v,"end_radians")?,field(&v,"max_deviation")?)?);}
    if op == "curve_cycloid" {return encode(trochoid::approximate_cycloid(field(&v,"center")?,field(&v,"radius")?,field(&v,"start_radians")?,field(&v,"end_radians")?,field(&v,"max_deviation")?)?);}
    if op == "curve_lissajous" {return encode(lissajous::approximate(field(&v,"center")?,field(&v,"amplitudes")?,field(&v,"frequencies")?,field(&v,"phases_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "curve_logarithmic_spiral" {return encode(logarithmic_spiral::approximate(field(&v,"center")?,field(&v,"radius")?,field(&v,"growth")?,field(&v,"start_radians")?,field(&v,"end_radians")?,field(&v,"max_deviation")?)?);}
    if op == "curve_involute" {return encode(involute::approximate(field(&v,"center")?,field(&v,"radius")?,field(&v,"start_radians")?,field(&v,"end_radians")?,field(&v,"max_deviation")?)?);}
    if op == "curve_variable_pitch_helix" {return encode(helix::approximate_variable_pitch(field(&v,"center")?,field(&v,"radius")?,field(&v,"height")?,field(&v,"turns")?,field(&v,"start_pitch")?,field(&v,"end_pitch")?,field(&v,"phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "curve_elliptic_helix" {return encode(helix::approximate_elliptic(field(&v,"center")?,field(&v,"radius_x")?,field(&v,"radius_y")?,field(&v,"height")?,field(&v,"turns")?,field(&v,"phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "curve_conical_helix" {return encode(helix::approximate_conical(field(&v,"center")?,field(&v,"start_radius")?,field(&v,"end_radius")?,field(&v,"height")?,field(&v,"turns")?,field(&v,"phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "curve_helix" {return encode(helix::approximate(field(&v,"center")?,field(&v,"radius")?,field(&v,"height")?,field(&v,"turns")?,field(&v,"phase_degrees")?,field(&v,"max_deviation")?)?);}
    if op == "surface_twist_sweep" { return sweep_requests::dispatch(v, op); }
    if op == "surface_two_guide_sweep" { return sweep_requests::dispatch(v, op); }
    if op == "surface_boundary_fill" {return encode(boundary_fill::fan(&field::<Vec<curve::Curve>>(&v,"boundaries")?,field(&v,"center")?)?);}
    if op == "surface_triangular_patch" {return encode(triangular_patch::patch(&field(&v,"base")?,&field(&v,"side_a")?,&field(&v,"side_b")?)?);}
    if op == "surface_gordon" { return loft_requests::dispatch(v, op); }
    if op == "surface_closed_loft" { return loft_requests::dispatch(v, op); }
    if op == "surface_loft_match_ends" { return loft_requests::dispatch(v, op); }
    if op == "surface_guided_loft" { return loft_requests::dispatch(v, op); }
    if op == "surface_control_tangent_loft" { return loft_requests::dispatch(v, op); }
    if op == "surface_clamped_loft" { return loft_requests::dispatch(v, op); }
    if op == "surface_natural_loft" { return loft_requests::dispatch(v, op); }
    if op == "surface_grid_spline" { return encode(grid_spline::interpolate(&field::<Vec<Vec<[f64;3]>>>(&v,"points")?,&field::<Vec<f64>>(&v,"parameters_u")?,&field::<Vec<f64>>(&v,"parameters_v")?)?); }
    if op == "surface_hermite_patch" { return encode(hermite_patch::patch(field(&v,"corners")?,field(&v,"tangent_u")?,field(&v,"tangent_v")?,field(&v,"twist")?)?); }
    if op == "curve_closed_spline" { return encode(closed_spline::interpolate(&field::<Vec<[f64;3]>>(&v,"points")?,&field::<Vec<f64>>(&v,"parameters")?)?); }
    if op == "curve_clamped_spline" { return encode(natural_spline::clamped(&field::<Vec<[f64;3]>>(&v,"points")?,&field::<Vec<f64>>(&v,"parameters")?,field(&v,"start_tangent")?,field(&v,"end_tangent")?)?); }
    if op == "curve_natural_spline" { return encode(natural_spline::interpolate(&field::<Vec<[f64;3]>>(&v,"points")?,&field::<Vec<f64>>(&v,"parameters")?)?); }
    if op == "curve_hermite" { return encode(hermite::interpolate(&field::<Vec<[f64;3]>>(&v,"points")?,&field::<Vec<[f64;3]>>(&v,"tangents")?,&field::<Vec<f64>>(&v,"parameters")?)?); }
    if op == "surface_formula" { return encode(formula::surface(field(&v,"domain")?,&formula::tokens(&field::<Vec<Value>>(&v,"expressions")?,true)?)?); }
    if op == "curve_formula" { return encode(formula::curve(field(&v,"domain")?,&formula::tokens(&field::<Vec<Value>>(&v,"expressions")?,false)?)?); }
    if op == "surface_sphere" { return encode(primitives::sphere(field(&v,"center")?,field(&v,"radius")?)?); }
    if op == "surface_cylinder" { return encode(primitives::cylinder(field(&v,"center")?,field(&v,"radius")?,field(&v,"height")?)?); }
    if op == "surface_cone" { return encode(primitives::cone(field(&v,"center")?,field(&v,"radius")?,field(&v,"height")?)?); }
    if op=="surface_bezier" {return encode(patches::bezier(field(&v,"points")?,optional_field(&v,"weights")?)?)}
    if op=="surface_bilinear" {return encode(patches::bilinear(field(&v,"corners")?)?)}
    if op=="surface_plane" {return encode(patches::plane(field(&v,"origin")?,field(&v,"axisU")?,field(&v,"axisV")?)?)}
    if op=="curve_bezier" {return encode(paths::bezier(field(&v,"points")?,optional_field(&v,"weights")?)?)}
    if op=="sweep_repair_circle_sections" { return sweep_requests::dispatch(v, op); }
    if op=="curve_repair_circle_section" {
        let r=crate::section_circle_repair::repair(&field::<Vec<curve::Curve>>(&v,"curves")?,field(&v,"quantum")?,field(&v,"tolerance")?,field(&v,"maxWork")?)?;
        return Ok(json!({"curves":r.curves,"displacementUpper":r.displacement_upper,"work":r.work,"reason":r.reason}));
    }
    if op == "sweep_retained_patch_regularity" { return sweep_requests::dispatch(v, op); }
    if op == "sweep_frenet_frame_regularity" { return sweep_requests::dispatch(v, op); }
    if op == "sweep_fixed_normal_frame_regularity" { return sweep_requests::dispatch(v, op); }
    if op == "sweep_authored_frame_regularity" { return sweep_requests::dispatch(v, op); }
    if op=="sweep_correct_miter_sections" { return sweep_requests::dispatch(v, op); }
    if op=="sweep_project_miter_caps" { return sweep_requests::dispatch(v, op); }
    if op=="sweep_project_sections" { return sweep_requests::dispatch(v, op); }
    if op=="curve_project_section_authored_axis" {
        let r=crate::section_projection::project_authored_axis(&field::<Vec<curve::Curve>>(&v,"curves")?,&field::<curve::Curve>(&v,"frameAxis")?,field(&v,"traversal")?,field(&v,"quantum")?,field(&v,"tolerance")?,field(&v,"maxWork")?)?;
        return Ok(json!({"curves":r.curves,"displacementUpper":r.displacement_upper,"exactPlanar":r.exact_planar,"work":r.work,"reason":r.reason}));
    }
    if op=="curve_project_section" {
        let r=crate::section_projection::project(&field::<Vec<curve::Curve>>(&v,"curves")?,field(&v,"axis")?,field(&v,"coefficients")?,field(&v,"offset")?,field(&v,"quantum")?,field(&v,"tolerance")?,field(&v,"maxWork")?)?;
        return Ok(json!({"curves":r.curves,"displacementUpper":r.displacement_upper,"exactPlanar":r.exact_planar,"work":r.work,"reason":r.reason}));
    }
    if op=="curve_miter_sections" { return sweep_requests::dispatch(v, op); }
    if op=="curve_transition_polyline" {return encode((if optional_field::<bool>(&v,"closed")?.unwrap_or(false) {paths::closed_transition_polyline} else {paths::transition_polyline})(&field::<Vec<[f64;3]>>(&v,"points")?,field(&v,"setback")?)?)}
    if op=="curve_round_polyline" {return encode((if optional_field::<bool>(&v,"closed")?.unwrap_or(false) {paths::closed_round_polyline} else {paths::round_polyline})(&field::<Vec<[f64;3]>>(&v,"points")?,field(&v,"radius")?)?)}
    if op=="curve_compose" {return encode(paths::compose(&field::<Vec<curve::Curve>>(&v,"curves")?)?)}
    if op=="curve_line" {return encode(primitives::line(field(&v,"start")?,field(&v,"end")?)?)}
    if op=="curve_polyline" {return encode(primitives::polyline(&field::<Vec<[f64;3]>>(&v,"points")?,optional_field(&v,"closed")?.unwrap_or(false))?)}
    if op=="curve_circle" {return encode(primitives::circle(field(&v,"center")?,field(&v,"normal")?,field(&v,"radius")?)?)}
    if op=="curve_circle_arc" {return encode(primitives::circle_arc(field(&v,"center")?,field(&v,"normal")?,field(&v,"radius")?,field(&v,"startDegrees")?,field(&v,"sweepDegrees")?)?)}
    if op == "curve_rational_polynomial" {
        return encode(polynomial::rational_curve(field(&v,"domain")?,&field::<Vec<[f64;4]>>(&v,"coefficients")?)?);
    }
    if op == "surface_rational_polynomial" {
        return encode(polynomial::rational_surface(field(&v,"domain")?,&field::<Vec<Vec<[f64;4]>>>(&v,"coefficients")?)?);
    }
    if op == "curve_polynomial_parametric" {
        return encode(polynomial::parametric_curve(field(&v,"domain")?,&field::<Vec<[f64;3]>>(&v,"coefficients")?)?);
    }
    if op == "surface_polynomial_parametric" {
        return encode(polynomial::parametric_surface(field(&v,"domain")?,&field::<Vec<Vec<[f64;3]>>>(&v,"coefficients")?)?);
    }
    if op == "surface_polynomial_graph" {
        return encode(polynomial::graph(field(&v,"bounds")?,&field::<Vec<Vec<f64>>>(&v,"coefficients")?)?);
    }
    if op == "surface_hyperboloid_one_sheet" {
        return encode(primitives::hyperboloid_one_sheet(field(&v,"center")?,field(&v,"radii")?,field(&v,"start")?,field(&v,"end")?)?);
    }
    if op == "surface_hyperboloid_two_sheet" {
        return encode(primitives::hyperboloid_two_sheet(field(&v,"center")?,field(&v,"radii")?,field(&v,"start")?,field(&v,"end")?,optional_field(&v,"lower")?.unwrap_or(false))?);
    }
    if op == "surface_quadratic_patch" {
        return encode(primitives::quadratic_patch(field(&v,"bounds")?,field(&v,"coefficients")?)?);
    }
    if op == "curve_parabola" || op == "curve_hyperbola" {
        let constructor=if op=="curve_parabola"{primitives::parabola}else{primitives::hyperbola};
        return encode(constructor(field(&v,"center")?,field(&v,"axisU")?,field(&v,"axisV")?,field(&v,"start")?,field(&v,"end")?)?);
    }
    if op == "surface_elliptic_cylinder" {
        return encode(primitives::elliptic_cylinder(field(&v,"center")?,field(&v,"radiusX")?,field(&v,"radiusY")?,field(&v,"height")?)?);
    }
    if op == "surface_cone_frustum" {
        return encode(primitives::cone_frustum(field(&v,"center")?,field(&v,"bottomRadius")?,field(&v,"topRadius")?,field(&v,"height")?)?);
    }
    if op == "curve_ellipse_arc" {
        return encode(primitives::ellipse_arc(field(&v,"center")?,field(&v,"axisU")?,field(&v,"axisV")?,field(&v,"startDegrees")?,field(&v,"sweepDegrees")?)?);
    }
    if op == "surface_ellipsoid" {
        return encode(primitives::ellipsoid(field(&v,"center")?,field(&v,"radii")?)?);
    }
    if op == "surface_torus" {
        return encode(primitives::torus(field(&v,"center")?,field(&v,"majorRadius")?,field(&v,"radialRadius")?,field(&v,"axialRadius")?)?);
    }
    if op == "curve_chain_diagnostics" {
        return Ok(crate::curve_offset_diagnostics::inspect_curves(&field::<Vec<curve::Curve>>(&v,"curves")?, optional_field::<usize>(&v,"maxPairs")?.unwrap_or(1_000_000))?.to_value());
    }
    if op == "curve_offset_trimmed_bevel" {
        let wire=curve_offset_wire::bevel_wire(&field(&v,"curve")?,field(&v,"distance")?,field(&v,"toleranceMm")?,field(&v,"maxCells")?)?;
        crate::check(wire.closed, "Trimmed offset requires a closed source.")?;
        let rule=match field::<String>(&v,"fillRule")?.as_str() {
            "nonzero" => crate::chord_winding::FillRule::NonZero,
            "evenodd" => crate::chord_winding::FillRule::EvenOdd,
            _ => { crate::check(false,"Choose nonzero or evenodd fill.")?; unreachable!() }
        };
        let graph=crate::chord_arrangement::split(&wire.edges,true,field(&v,"intersectionToleranceMm")?,optional_field::<usize>(&v,"maxPairs")?.unwrap_or(1_000_000))?;
        let loops=crate::chord_fill_selection::boundary_curves(&graph,rule,optional_field::<usize>(&v,"maxWitnessChecks")?.unwrap_or(1_000_000))?;
        let intersection_error=graph.vertices.iter().map(|p|p.error_upper_mm).fold(0.,f64::max);
        return Ok(json!({"loops":encode(loops)?,"report":{
            "accepted":true,"method":"represented-bevel-offset-fill/1",
            "fillRule":v["fillRule"].clone(),"regionTrimmed":true,
            "topologyScope":"represented-reconstructed-chord-graph",
            "regionTopologyCertified":false,"originalOffsetTopologyCertified":false,
            "sourceWireErrorUpperMm":wire.error_upper_mm,
            "intersectionConstructionErrorUpperMm":intersection_error
        }}));
    }
    if op == "curve_offset_bevel_wire" {
        let result=curve_offset_wire::bevel_wire(&field(&v,"curve")?,field(&v,"distance")?,field(&v,"toleranceMm")?,field(&v,"maxCells")?)?;
        let diagnostics=crate::curve_offset_diagnostics::inspect_chain(&result.edges,result.closed,optional_field::<usize>(&v,"maxPairs")?.unwrap_or(1_000_000))?.to_value();
        let cells:Vec<_>=result.edges.iter().zip(&result.roles).map(|(edge,role)|{
            let source=match role {curve_offset_wire::Role::Source(domain)=>json!({"kind":"source-offset","domain":domain}),curve_offset_wire::Role::Bevel(knot)=>json!({"kind":"bevel","sourceKnot":knot})};
            json!({"domain":edge.domain,"stationDomain":edge.domain,"source":source,"errorUpperMm":edge.error_upper_mm})
        }).collect();
        return Ok(json!({"curves":encode(result.curves)?,"report":{"accepted":true,"closed":result.closed,"wholeWire":true,"wholeCurve":false,"method":"outward-source-offset-bevel-wire/1","errorUpperMm":result.error_upper_mm,"toleranceMm":v["toleranceMm"].clone(),"cells":cells,"chainDiagnostics":diagnostics,"offsetRegularityCertified":false,"regionTopologyCertified":false,"regionTrimmed":false}}));
    }
    if op == "curve_offset_bounded" {
        let result = curve_offset::approximate_curve(&field(&v,"curve")?,field(&v,"distance")?,
            field(&v,"toleranceMm")?,field(&v,"maxCells")?)?;
        let cells: Vec<_> = result.segments.iter().map(|s|json!({
            "domain":s.domain,"errorUpperMm":s.error_upper_mm
        })).collect();
        let diagnostics = if result.segments.is_empty() {Value::Null} else {
            crate::curve_offset_diagnostics::inspect_chain(&result.segments,result.closed,
                optional_field::<usize>(&v,"maxPairs")?.unwrap_or(1_000_000))?.to_value()
        };
        return Ok(json!({"curves":encode(result.curves)?,"report":{
            "closed":result.closed,
            "accepted":true,"errorUpperMm":result.error_upper_mm,
            "toleranceMm":v["toleranceMm"].clone(),"wholeCurve":true,
            "method":"outward-rational-jets-chord-bound/1","cells":cells,
            "chainDiagnostics":diagnostics,
            "offsetRegularityCertified":false,
            "regionTopologyCertified":false
        }}));
    }
    if op == "curve_certify_foundation" { return certificate_requests::dispatch(v, op); }
    if op == "surface_certify_foundation" { return certificate_requests::dispatch(v, op); }
    if op == "surface_offset_evaluate" {
        let e = surface_offset::evaluate(
            &field(&v, "surface")?,
            field(&v, "parameters")?,
            field(&v, "distance")?,
        )?;
        return Ok(
            json!({"method":"normal-offset-numerical-jets","point":e.point,"du":e.du,"dv":e.dv,
            "sourceUnitNormal":e.source_unit_normal,"certified":false,"topologyAuthority":false}),
        );
    }
    if op == "surface_offset_bounds" {
        let r = surface_offset::bounds(
            &field(&v, "surface")?,
            field(&v, "domain")?,
            field(&v, "distance")?,
            field(&v, "maxSpans")?,
        )?;
        return Ok(
            json!({"method":"interval-source-normal-offset","scope":"incident-span-offset-images",
            "image":r.image,"unitNormals":r.unit_normals,"normalSpanVisits":r.spans,"reason":r.reason,
            "offsetRegularityCertified":false,"continuityCertified":false,"topologyAuthority":false}),
        );
    }
    if op == "surface_offset_jacobian_bounds" {
        let r = surface_offset::jacobian_bounds(
            &field(&v, "surface")?,
            field(&v, "domain")?,
            field(&v, "distance")?,
            field(&v, "maxSpans")?,
        )?;
        return Ok(
            json!({"method":"interval-source-normal-offset-jets","scope":"incident-span-offset-jets",
            "image":r.image,"derivatives":r.derivatives,"normalSpanVisits":r.spans,"reason":r.reason,
            "offsetRegularityCertified":false,"continuityCertified":false,"topologyAuthority":false}),
        );
    }
    if op == "surface_offset_candidates" {
        let a: Surface = field(&v, "a")?;
        let b: Surface = field(&v, "b")?;
        let r = surface_offset::intersection_candidates(
            [&a, &b],
            field(&v, "domains")?,
            field(&v, "distances")?,
            field(&v, "parameterTolerance")?,
            field(&v, "maxBoxes")?,
            field(&v, "maxSpans")?,
        )?;
        return Ok(
            json!({"method":"interval-offset-pair-exclusion","scope":"untrimmed-offset-carriers",
            "candidateBoxes":r.boxes,"pendingBoxes":r.pending,"visitedBoxes":r.visited_boxes,
            "excludedBoxes":r.excluded_boxes,"normalSpanVisits":r.normal_span_visits,"reason":r.reason,
            "rootExistenceProven":false,"wholeCurveComplete":false,"trimMembershipProven":false,"topologyAuthority":false}),
        );
    }
    if op == "surface_offset_contact_qualification" {
        let a: Surface = field(&v,"a")?;let b: Surface = field(&v,"b")?;
        let candidate: Surface = field(&v,"candidate")?;
        let pcurves: [crate::curve::Curve;2] = field(&v,"sourcePcurves")?;
        let first_loops: Vec<Vec<crate::curve::Curve>> = field(&v,"firstLoops")?;
        let second_loops: Vec<Vec<crate::curve::Curve>> = field(&v,"secondLoops")?;
        let report=crate::offset_contact_trims::certify_with_tangency(&candidate,[&pcurves[0],&pcurves[1]],[&a,&b],[&first_loops,&second_loops],
            field(&v,"distances")?,field(&v,"fixedAxis")?,field(&v,"fixedInterval")?,field(&v,"firstOther")?,field(&v,"secondDomain")?,field(&v,"maxSpans")?,
            field(&v,"toleranceMm")?,field(&v,"toleranceUv")?,crate::offset_contact_trims::Limits {
                fit_cells:field(&v,"maxFitCells")?,uv_cells:field(&v,"maxUvCells")?,agreement_cells:field(&v,"maxAgreementCells")?,root_refinements:field(&v,"rootRefinements")?,
                trim:crate::trimmed_offset_contact::Limits {max_pairs:field(&v,"maxPairs")?,max_cells:field(&v,"maxTrimCells")?,max_domain_cells:field(&v,"maxDomainCells")?},
            },field(&v,"maxSineSquared")?,crate::offset_contact_trims::TangentLimits {
                position_cells:field(&v,"maxTangentPositionCells")?,normal_cells:field(&v,"maxNormalCells")?,normal_spans:field(&v,"maxNormalSpans")?,
            })?;
        return Ok(json!({"method":"offset-contact-qualification-delivery","request":v,"candidateSurface":candidate,"qualification":report.to_value()}));
    }
    if op == "surface_offset_envelope_fit" {
        let a: Surface = field(&v,"a")?;let b: Surface = field(&v,"b")?;
        let candidate: Option<Surface> = v.get("candidate").filter(|x|!x.is_null()).map(|_|field(&v,"candidate")).transpose()?;
        return crate::offset_envelope_fit::deliver(candidate.as_ref(),[&a,&b],field(&v,"distances")?,
            field(&v,"fixedAxis")?,field(&v,"fixedInterval")?,field(&v,"firstOther")?,
            field(&v,"secondDomain")?,field(&v,"maxSpans")?,field(&v,"toleranceMm")?,field(&v,"maxCells")?);
    }
    if op == "surface_offset_envelope" {
        let a: Surface = field(&v,"a")?;let b: Surface = field(&v,"b")?;
        return Ok(crate::offset_envelope::certify([&a,&b],field(&v,"distances")?,
            field(&v,"fixedAxis")?,field(&v,"fixedInterval")?,field(&v,"firstOther")?,
            field(&v,"secondDomain")?,field(&v,"maxSpans")?,field(&v,"maxCells")?)?.to_value());
    }
    if op == "surface_offset_contact_tangent" {
        let a: Surface = field(&v,"a")?;let b: Surface = field(&v,"b")?;
        return Ok(crate::offset_contact_tangent::certify([&a,&b],field(&v,"distances")?,
            field(&v,"fixedAxis")?,field(&v,"fixedInterval")?,field(&v,"firstOther")?,
            field(&v,"secondDomain")?,field(&v,"maxSpans")?)?.to_value());
    }
    if op == "surface_offset_source_boundary" {
        let a: Surface = field(&v,"a")?;let b: Surface = field(&v,"b")?;
        let first_loops: Vec<Vec<crate::curve::Curve>> = field(&v,"firstLoops")?;
        let second_loops: Vec<Vec<crate::curve::Curve>> = field(&v,"secondLoops")?;
        let first_world: Vec<Vec<crate::offset_source_boundary::Coedge>> = field(&v,"firstCoedges")?;
        let second_world: Vec<Vec<crate::offset_source_boundary::Coedge>> = field(&v,"secondCoedges")?;
        return Ok(crate::offset_source_boundary::certify([&a,&b],[&first_loops,&second_loops],[&first_world,&second_world],
            field(&v,"distances")?,field(&v,"fixedAxis")?,field(&v,"fixedInterval")?,field(&v,"firstOther")?,
            field(&v,"secondDomain")?,field(&v,"maxSpans")?,field(&v,"toleranceUv")?,
            crate::trimmed_offset_contact::Limits{max_pairs:field(&v,"maxPairs")?,max_cells:field(&v,"maxCells")?,max_domain_cells:field(&v,"maxDomainCells")?},
            field(&v,"toleranceMm")?,field(&v,"maxExactWork")?,field(&v,"maxAgreementCells")?)?.to_value());
    }
    if op == "surface_offset_trimmed_contact_band" {
        let a: Surface = field(&v,"a")?;
        let b: Surface = field(&v,"b")?;
        let first_loops: Vec<Vec<crate::curve::Curve>> = field(&v,"firstLoops")?;
        let second_loops: Vec<Vec<crate::curve::Curve>> = field(&v,"secondLoops")?;
        return Ok(crate::trimmed_offset_contact::certify([&a,&b],[&first_loops,&second_loops],
            field(&v,"distances")?,field(&v,"fixedAxis")?,field(&v,"fixedInterval")?,
            field(&v,"firstOther")?,field(&v,"secondDomain")?,field(&v,"maxSpans")?,field(&v,"toleranceUv")?,
            crate::trimmed_offset_contact::Limits{max_pairs:field(&v,"maxPairs")?,max_cells:field(&v,"maxCells")?,max_domain_cells:field(&v,"maxDomainCells")?})?.to_value());
    }
    if op == "surface_offset_contact_band" {
        let a: Surface = field(&v, "a")?;
        let b: Surface = field(&v, "b")?;
        let r = surface_offset::certify_contact_band(
            [&a, &b], field(&v, "distances")?, field(&v, "fixedAxis")?,
            field(&v, "fixedInterval")?, field(&v, "firstOther")?,
            field(&v, "secondDomain")?, field(&v, "maxSpans")?,
        )?;
        let (status, witness) = match r {
            surface_offset::ContactBand::Excluded => ("excluded", Value::Null),
            surface_offset::ContactBand::Unresolved => ("unresolved", Value::Null),
            surface_offset::ContactBand::ContinuousBranch(w) => (
                "continuous-branch", json!({"firstUV":w.first_uv,"secondUV":w.second_uv,
                "centerIntervalMm":w.point,"contractionUpper":w.contraction_upper}),
            ),
        };
        return Ok(json!({"method":"interval-offset-band-krawczyk","scope":"parameter-band-within-tube",
            "status":status,"witness":witness,"rootForEveryParameterProven":status=="continuous-branch",
            "uniqueWithinTube":status=="continuous-branch","continuousBranchProven":status=="continuous-branch",
            "wholeCurveComplete":false,"trimMembershipProven":false,"topologyAuthority":false}));
    }
    if op == "surface_offset_contact_section" {
        let a: Surface = field(&v, "a")?;
        let b: Surface = field(&v, "b")?;
        let r = surface_offset::certify_contact_section(
            [&a, &b],
            field(&v, "distances")?,
            field(&v, "fixedAxis")?,
            field(&v, "fixed")?,
            field(&v, "firstOther")?,
            field(&v, "secondDomain")?,
            field(&v, "maxSpans")?,
        )?;
        let (status, witness) = match r {
            surface_contact::Verdict::Excluded => ("excluded", Value::Null),
            surface_contact::Verdict::Unresolved => ("unresolved", Value::Null),
            surface_contact::Verdict::Witness(w) => (
                "unique-contact",
                json!({"firstUV":w.first_uv,"secondUV":w.second_uv,
                "centerIntervalMm":w.point,"contractionUpper":w.contraction_upper}),
            ),
        };
        return Ok(
            json!({"method":"interval-offset-section-krawczyk","scope":"fixed-parameter-offset-section",
            "status":status,"witness":witness,"rootExistenceProven":status=="unique-contact",
            "uniqueInSection":status=="unique-contact","wholeCurveComplete":false,"trimMembershipProven":false,"topologyAuthority":false}),
        );
    }
    if op == "surface_distance" {
        return Ok(surface_distance::distance(
            &field(&v, "a")?,
            &field(&v, "b")?,
            field(&v, "toleranceMm")?,
            field(&v, "maxCells")?,
        )?
        .to_value());
    }
    if op == "curve_distance" {
        return Ok(curve_distance::distance(
            &field(&v, "a")?,
            &field(&v, "b")?,
            field(&v, "toleranceMm")?,
            field(&v, "maxCells")?,
        )?
        .to_value());
    }
    if op == "curve_trim_screen_point" {
        return trim_point::trim_at_screen_point(
            &field(&v, "curve")?,
            &field::<Vec<f64>>(&v, "point")?,
            &field(&v, "matrix")?,
            &field::<String>(&v, "keep")?,
            field(&v, "radius")?,
        );
    }
    if op == "curve_trim_point" {
        return trim_point::trim_at_point(
            &field(&v, "curve")?,
            &field::<Vec<f64>>(&v, "point")?,
            &field::<String>(&v, "keep")?,
            field(&v, "maxDistance")?,
        );
    }
    if op == "curve_project_certified" { return certificate_requests::dispatch(v, op); }
    if op == "surface_project_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_interpolate_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_approximate_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_reparameterize_exact" {
        return foundation::reparameterize_curve(
            &field(&v, "curve")?,
            field(&v, "domain")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_insert_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_elevate_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_remove_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_rebuild_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_reduce_certified" { return certificate_requests::dispatch(v, op); }
    if op == "surface_rebuild_certified" { return certificate_requests::dispatch(v, op); }
    if op == "surface_remove_certified" || op == "surface_reduce_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_periodic_edit_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_periodic_split_certified" { return certificate_requests::dispatch(v, op); }
    if op == "surface_periodic_edit_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_reparameterization_certify" { return certificate_requests::dispatch(v, op); }
    if op == "curve_reparameterized_evaluate" {
        return foundation::evaluate_reparameterized_curve(
            &field(&v, "curve")?,
            &v["mapping"],
            field(&v, "u")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_fit_certified" { return certificate_requests::dispatch(v, op); }
    if op == "surface_interpolate_certified" || op == "surface_fit_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_materialize_reparameterization_bounded" {
        return foundation::materialize_reparameterized_curve_bounded(
            &field(&v, "curve")?, &v["mapping"], field(&v, "errorBudget")?,
            field(&v, "maxCells")?, field(&v, "maxMapEvaluations")?,
        );
    }
    if op == "curve_materialize_reparameterization" {
        return foundation::materialize_reparameterized_curve(
            &field(&v, "curve")?,
            &v["mapping"],
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_fit_cloud_certified" { return certificate_requests::dispatch(v, op); }
    if op == "surface_fit_cloud_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_curve_intersect_certified" { return certificate_requests::dispatch(v, op); }
    if op == "curve_surface_intersect_certified" { return certificate_requests::dispatch(v, op); }
    if op == "surface_surface_intersect_certified" { return certificate_requests::dispatch(v, op); }
    if op == "surface_surface_verify_coverage" {
        return ss_intersection::verify_ss_coverage(&v["report"]);
    }
    if op == "nurbs_intersection_resource_probe" {
        return intersection::resource_boundary_probe(field(&v, "degree")?, field(&v, "controls")?);
    }
    if op == "nurbs_ss_resource_probe" {
        return ss_intersection::ss_resource_probe(
            field(&v, "degreeU")?,
            field(&v, "degreeV")?,
            field(&v, "controls")?,
        );
    }
    if op == "nurbs_deform_curve" {
        return encode(edit::deform_curve(
            &field(&v, "curve")?,
            &field(&v, "deformation")?,
        )?);
    }
    if op == "nurbs_deform_surface" {
        return encode(edit::deform_surface(
            &field(&v, "surface")?,
            &field(&v, "deformation")?,
        )?);
    }
    if op == "nurbs_brush_curve" {
        return encode(edit::brush_curve(
            &field(&v, "curve")?,
            &field(&v, "brush")?,
        )?);
    }
    if op == "nurbs_sculpt_curve" {
        return encode(edit::sculpt_curve(
            &field(&v, "curve")?,
            &field(&v, "brush")?,
        )?);
    }
    if op == "nurbs_sculpt_surface" {
        return encode(edit::sculpt_surface(
            &field(&v, "surface")?,
            &field(&v, "brush")?,
        )?);
    }
    if op == "nurbs_brush_surface" {
        return encode(edit::brush_surface(
            &field(&v, "surface")?,
            &field(&v, "brush")?,
        )?);
    }
    if op == "basis" {
        return encode(curve::basis(
            field(&v, "degree")?,
            &field::<Vec<f64>>(&v, "knots")?,
            field(&v, "controlCount")?,
            field(&v, "u")?,
            v["periodic"].as_bool().unwrap_or(false),
        )?);
    }
    if op == "curve_prepare_coons_weights" {
        let result=coons::checked_endpoint_weights(&field(&v,"curve")?,field(&v,"maxError")?)?;
        return Ok(json!({"curve":result.curve,"report":{"accepted":result.accepted,
            "errorUpper":result.error_upper,"budget":field::<f64>(&v,"maxError")?,
            "wholeCurve":true,"method":"outward-homogeneous-Bernstein-difference"}}));
    }
    if op == "surface_coons_patch" {
        return encode(coons::patch(&field::<Vec<curve::Curve>>(
            &v,
            "boundaries",
        )?)?);
    }
    if op == "curve_match_g1" {
        return continuity::curve_match::checked(
            &field(&v, "reference")?,
            &field(&v, "edited")?,
            &field::<String>(&v, "referenceEnd")?,
            &field::<String>(&v, "editedEnd")?,
            optional_field::<f64>(&v, "maxAngleDegrees")?.unwrap_or(1e-6),
        );
    }
    if op == "surface_prepare_seams" {
        return continuity::preparation::checked_with_conversion(
            &field(&v, "reference")?,
            &field(&v, "edited")?,
            &field::<String>(&v, "referenceBoundary")?,
            &field::<String>(&v, "editedBoundary")?,
            optional_field::<bool>(&v, "reverse")?.unwrap_or(false),
            field(&v, "maxError")?,
            optional_field::<bool>(&v, "openPeriodic")?.unwrap_or(false),
        );
    }
    if op == "surface_match_jets" {
        return continuity::match_surface_jets_checked(
            &field(&v, "reference")?,
            &field(&v, "edited")?,
            &field::<String>(&v, "referenceBoundary")?,
            &field::<String>(&v, "editedBoundary")?,
            field(&v, "order")?,
            field(&v, "scale")?,
            optional_field::<bool>(&v, "reverse")?.unwrap_or(false),
            optional_field::<f64>(&v, "maxError")?.unwrap_or(1e-6),
        );
    }
    if op == "surface_scaled_sweep" { return sweep_requests::dispatch(v, op); }
    if op == "surface_profile_sweep" { return sweep_requests::dispatch(v, op); }
    if op == "surface_framed_sweep" { return sweep_requests::dispatch(v, op); }
    if op == "surface_sweep" { return sweep_requests::dispatch(v, op); }
    if op == "loft_aligned" { return loft_requests::dispatch(v, op); }
    if op == "loft" { return loft_requests::dispatch(v, op); }
    if op.starts_with("curve_") || op == "extrude" || op == "revolve" {
        let c: curve::Curve = field(&v, "curve")?;
        return match op.as_str() {
            "curve_validate" => {
                c.validate()?;
                Ok(Value::Null)
            }
            "curve_evaluate" => encode(c.evaluate(field(&v, "u")?)?),
            "curve_evaluate_points" => encode(c.evaluate_points(&field::<Vec<f64>>(&v, "parameters")?)?),
            "curve_sample" => encode(c.sample_uniform(
                field(&v, "segments")?,
                optional_field::<bool>(&v, "includeEnd")?.unwrap_or(true),
            )?),
            "curve_insert" => encode(c.insert(field(&v, "u")?, field(&v, "count")?)?),
            "curve_trim" => encode(c.trim(field(&v, "a")?, field(&v, "b")?)?),
            "curve_split" => encode(c.split(field(&v, "u")?)?),
            "curve_reverse" => encode(c.reverse()?),
            "curve_elevate" => encode(c.elevate(field(&v, "degree")?)?),
            "curve_decompose" => encode(c.decompose()?),
            "curve_decomposition_audit" => {
                let r=crate::curve_decomposition_certificate::inspect(&c,field(&v,"span")?,&field(&v,"retained")?,field(&v,"maxProducts")?)?;
                Ok(json!({"errorUpper":r.error_upper,"products":r.products,"reason":r.reason,"method":"original-span-bernstein-decomposition","continuousBound":false}))
            },
            "curve_bounds" => encode(c.bounds()?),
            "extrude" => encode(surface::extrude(&c, field(&v, "vector")?)?),
            "revolve" => encode(surface::revolve(
                &c,
                field(&v, "origin")?,
                field(&v, "axis")?,
                field(&v, "angle")?,
            )?),
            _ => Err(input("Unknown curve operation")),
        };
    }
    let s: surface::Surface = field(&v, "surface")?;
    match op.as_str() {
        "surface_validate" => {
            s.validate()?;
            Ok(Value::Null)
        }
        "surface_evaluate" => encode(s.evaluate(field(&v, "u")?, field(&v, "v")?)?),
        "surface_insert" => encode(s.edit_axis(field(&v, "axis")?, |c| {
            c.insert(field(&v, "u")?, field(&v, "count")?)
        })?),
        "surface_elevate" => {
            encode(s.edit_axis(field(&v, "axis")?, |c| c.elevate(field(&v, "degree")?))?)
        }
        "surface_reverse" => encode(s.edit_axis(field(&v, "axis")?, |c| c.reverse())?),
        "surface_trim" => encode(s.trim(field(&v, "bounds")?)?),
        "surface_iso" => encode(s.iso(field(&v, "axis")?, field(&v, "u")?)?),
        "surface_bounds" => encode(s.bounds()?),
        _ => Err(input("Unknown NURBS operation")),
    }
}
