use super::*;
use crate::analysis::surface_measure::{
    TrimmedAreaReport, trimmed_surface_area_in_window, trimmed_surface_area_inertia,
    trimmed_surface_centroid,
};
use crate::editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches;
use crate::trim_domain::TrimDomain;

pub(super) const OPS: &[&str] = &[
    "combine_surface_measures",
    "combine_surface_densities",
    "mass_weighted_centroid",
    "physical_principal_moments",
    "physical_moment_gaps",
    "find_principal_axis",
    "bound_axis_projector",
    "certify_principal_direction",
    "tensor_principal_bounds",
    "tensor_axis_bounds",
    "trim_set_axis_bounds",
    "periodic_trim_set_area",
    "periodic_trim_set_centroid",
    "trimmed_surface_area_in_window",
    "trimmed_surface_centroid",
    "trimmed_surface_area_inertia",
];

fn area(report: TrimmedAreaReport) -> Value {
    json!({"minimumSurfaceCells":report.minimum_surface_cells,"trimCells":report.trim_cells,
        "area":report.area.map(|a|json!({"boundsMm2":a.bounds,"valueMm2":a.value,"errorUpperMm2":a.error_upper,
            "withinTolerance":a.within_tolerance,"cells":a.cells,"stopReason":format!("{:?}",a.stop_reason)}))})
}
pub(super) fn dispatch(v: &Value, op: &str) -> Result<Value> {
    if matches!(op,"combine_surface_measures" | "combine_surface_densities") {
        return assembly(v,op);
    }
    if matches!(op,"find_principal_axis" | "bound_axis_projector") {
        use crate::analysis::surface_measure::{find_certified_principal_axis,bound_principal_axis_projector};
        let certificate = find_certified_principal_axis(field(v,"tensor_bounds")?,field(v,"moment_index")?,field(v,"moment_tolerance")?)?;
        let projector = if op == "bound_axis_projector" {
            certificate.map(bound_principal_axis_projector).transpose()?
        } else { None };
        return Ok(json!({"direction":certificate.map(|c|c.direction),"sineUpper":certificate.map(|c|c.sin_angle_upper),
            "projectorBounds":projector}));
    }
    if op == "mass_weighted_centroid" {
        use crate::analysis::surface_measure::{SurfaceAssemblyMassBounds,bound_mass_weighted_centroid};
        let masses: Vec<[f64;2]> = field(v,"mass_bounds")?;
        let centroids: Vec<[[f64;2];3]> = field(v,"centroid_bounds")?;
        check(!masses.is_empty() && masses.len()<=256 && masses.len()==centroids.len(),"Expected matching mass and centroid intervals for 1 to 256 pieces")?;
        let pieces=masses.into_iter().zip(centroids).map(|(mass_bounds,centroid_bounds)|SurfaceAssemblyMassBounds{
            mass_bounds,centroid_bounds,central_inertia_bounds:[[[0.;2];3];3],
        }).collect::<Vec<_>>();
        return Ok(json!({"centroidBounds":bound_mass_weighted_centroid(&pieces)?,"pieceCount":pieces.len()}));
    }
    if matches!(op,"physical_principal_moments" | "physical_moment_gaps" | "certify_principal_direction") {
        use crate::analysis::surface_measure::{bound_physical_principal_moments,bound_physical_principal_moment_gaps,certify_principal_axis_direction};
        let tensor=field(v,"tensor_bounds")?;
        let tolerance=field(v,"moment_tolerance")?;
        return match op {
            "physical_principal_moments" => Ok(json!({"principalBounds":bound_physical_principal_moments(tensor,tolerance)?})),
            "physical_moment_gaps" => Ok(json!({"gapBounds":bound_physical_principal_moment_gaps(tensor,tolerance)?})),
            _ => Ok(json!({"sineUpper":certify_principal_axis_direction(tensor,field(v,"moment_index")?,field(v,"axis_vector")?,tolerance)?})),
        };
    }
    let source: Surface = field(v, "surface")?;
    if matches!(op,"tensor_principal_bounds" | "tensor_axis_bounds") {
        use crate::analysis::surface_measure::{bound_principal_moments_from_tensor,bound_directional_inertia};
        let tensor = field(v,"tensor_bounds")?;
        return if op == "tensor_principal_bounds" {
            Ok(json!({"principalBounds":bound_principal_moments_from_tensor(tensor,field(v,"moment_tolerance")?)?}))
        } else {
            Ok(json!({"axisBounds":bound_directional_inertia(tensor,field(v,"axis_vector")?)?}))
        };
    }
    let uv_tolerance = field(v, "tolerance_uv")?;
    let mut used = 0;
    check(
        v.get("loops").is_some() != v.get("contours").is_some(),
        "Specify one of trim loops or periodic contours",
    )?;
    let (surface, trim) = if v.get("loops").is_some() {
        let loops: Vec<Vec<crate::curve::Curve>> = field(v, "loops")?;
        (source, TrimDomain::new(&loops, uv_tolerance)?)
    } else {
        let contours: Vec<Vec<[f64; 2]>> = field(v, "contours")?;
        let paths: Vec<_> = contours.iter().map(Vec::as_slice).collect();
        let result =
            build_exact_periodic_surface_trim_patches(&source, &paths, field(v, "max_work")?)?;
        used = result.work_used;
        let Some(patches) = result.patches else {
            return Ok(
                json!({"workUsed":used,"refusal":result.refusal.map(|r|format!("{r:?}")),"measure":null}),
            );
        };
        if op == "trim_set_axis_bounds" {
            let central = crate::analysis::surface_measure::periodic_trim_patch_set_central_inertia(
                &patches,field(v,"tolerance_mm2")?,uv_tolerance,field(v,"max_surface_cells")?,field(v,"max_trim_cells")?)?;
            return Ok(json!({"axisBounds":central.bound_axis_moment(field(v,"axis_vector")?)?,
                "centroidBoundsMm":central.centroid.centroid_bounds,"workUsed":used}));
        }
        if matches!(op, "periodic_trim_set_area" | "periodic_trim_set_centroid") {
            use crate::analysis::surface_measure::{periodic_trim_patch_set_area, periodic_trim_patch_set_centroid};
            let tolerance = field(v, "tolerance_mm2")?;
            let cells = field(v, "max_surface_cells")?;
            let trim_cells = field(v, "max_trim_cells")?;
            let (measure, centroid) = if op == "periodic_trim_set_area" {
                (periodic_trim_patch_set_area(&patches,tolerance,uv_tolerance,cells,trim_cells)?, None)
            } else {
                let r = periodic_trim_patch_set_centroid(&patches,tolerance,uv_tolerance,cells,trim_cells)?;
                (r.measure, r.centroid_bounds)
            };
            let components = measure.components.iter().map(|a|json!({"boundsMm2":a.bounds,
                "withinTolerance":a.within_tolerance,"cells":a.cells})).collect::<Vec<_>>();
            return Ok(json!({"measure":area(measure.total),"components":components,"centroidBoundsMm":centroid,
                "workUsed":used,"refusal":null}));
        }
        let index: usize = field(v, "outer_contour")?;
        let patch = patches
            .into_iter()
            .find(|p| p.trim.contours.first() == Some(&index))
            .ok_or_else(|| crate::input("Expected an outer contour index"))?;
        (
            patch.surface,
            TrimDomain::new(&patch.trim.loops, uv_tolerance)?,
        )
    };
    let tol = field(v, "tolerance_mm2")?;
    let cells = field(v, "max_surface_cells")?;
    let trim_cells = field(v, "max_trim_cells")?;
    let mut result = match op {
        "trimmed_surface_area_in_window" => {
            json!({"measure":area(trimmed_surface_area_in_window(&surface,&trim,field(v,"window")?,tol,cells,trim_cells)?)})
        }
        "trimmed_surface_centroid" => {
            let r = trimmed_surface_centroid(&surface, &trim, tol, cells, trim_cells)?;
            json!({"measure":area(r.measure),"centroidBoundsMm":r.centroid_bounds})
        }
        "trimmed_surface_area_inertia" => {
            let r = trimmed_surface_area_inertia(
                &surface,
                &trim,
                field(v, "origin")?,
                tol,
                cells,
                trim_cells,
            )?;
            json!({"measure":area(r.measure),"inertiaBoundsMm4":r.inertia_bounds})
        }
        _ => return Err(crate::input("Unknown trimmed surface integral")),
    };
    result["workUsed"] = json!(used);
    result["refusal"] = Value::Null;
    Ok(result)
}

fn assembly(v: &Value, op: &str) -> Result<Value> {
    use crate::analysis::surface_measure::{SurfaceMassPropertiesBounds, trimmed_surface_central_inertia,
        combine_surface_mass_properties, combine_surface_mass_properties_with_density};
    let surfaces: Vec<Surface> = field(v,"surfaces")?;
    check((1..=32).contains(&surfaces.len()),"Expected 1 to 32 assembly surfaces")?;
    let densities: Option<Vec<[f64;2]>> = if op == "combine_surface_densities" {
        let values: Vec<[f64;2]> = field(v,"densities")?;
        check(values.len()==surfaces.len() && values.iter().all(|[lo,hi]| lo.is_finite() && hi.is_finite() && *lo>0. && lo<=hi),
            "Expected one positive ordered density interval per surface")?;
        Some(values)
    } else { None };
    let tol = field(v,"tolerance_mm2")?;
    let uv = field(v,"tolerance_uv")?;
    let cells = field(v,"max_surface_cells")?;
    let trim_cells = field(v,"max_trim_cells")?;
    let mut pieces = Vec::with_capacity(surfaces.len());
    for (index,surface) in surfaces.iter().enumerate() {
        surface.validate()?;
        let u = [surface.knots_u[surface.degree_u],surface.knots_u[surface.control_points.len()]];
        let v = [surface.knots_v[surface.degree_v],surface.knots_v[surface.control_points[0].len()]];
        let border = crate::curve::Curve::from_polyline(vec![vec![u[0],v[0]],vec![u[1],v[0]],
            vec![u[1],v[1]],vec![u[0],v[1]],vec![u[0],v[0]]])?;
        let trim = TrimDomain::new(&[vec![border]],uv)?;
        let r = trimmed_surface_central_inertia(surface,&trim,tol,cells,trim_cells)?;
        let (Some(area),Some(centroid_bounds),Some(central_inertia_bounds)) =
            (r.centroid.measure.area,r.centroid.centroid_bounds,r.central_inertia_bounds) else {
                return Err(crate::input(format!("Mass properties unavailable for surface {index}")));
            };
        pieces.push(SurfaceMassPropertiesBounds{area_bounds:area.bounds,centroid_bounds,central_inertia_bounds});
    }
    if let Some(densities) = densities {
        let weighted = pieces.into_iter().zip(densities).collect::<Vec<_>>();
        let r = combine_surface_mass_properties_with_density(&weighted)?;
        Ok(json!({"pieceCount":weighted.len(),"massBounds":r.mass_bounds,"centroidBoundsMm":r.centroid_bounds,
            "centralInertiaBounds":r.central_inertia_bounds}))
    } else {
        let r = combine_surface_mass_properties(&pieces)?;
        Ok(json!({"pieceCount":pieces.len(),"areaBoundsMm2":r.area_bounds,"centroidBoundsMm":r.centroid_bounds,
            "centralInertiaBounds":r.central_inertia_bounds}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_uv_loop_bounds_area_on_open_support() {
        let line = crate::curve::Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let surface = crate::surface::extrude(&line, [0., 1., 0.]).unwrap();
        let border = crate::curve::Curve {
            degree: 1,
            knots: vec![0., 0., 1., 2., 3., 4., 4.],
            control_points: vec![
                vec![0., 0.],
                vec![1., 0.],
                vec![1., 1.],
                vec![0., 1.],
                vec![0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: false,
        };
        let request = json!({"surface":surface,"loops":[[border]],"window":[[0.25,0.75],[0.25,0.75]],"tolerance_uv":1e-6,"tolerance_mm2":1e-4,"max_surface_cells":1000,"max_trim_cells":10000});
        let report = dispatch(&request, "trimmed_surface_area_in_window").unwrap();
        let area = &report["measure"]["area"];
        assert_eq!(area["withinTolerance"], true);
        let bounds = &area["boundsMm2"];
        assert!(bounds[0].as_f64().unwrap() <= 0.25 && bounds[1].as_f64().unwrap() >= 0.25);
    }
}
