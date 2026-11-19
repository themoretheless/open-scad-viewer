use super::*;
use crate::editing::exact_periodic_surface::*;
use num_rational::BigRational;

pub(super) const OPS: &[&str] = &[
    "classify_exact_periodic_surface_path_point",
    "classify_exact_periodic_surface_path_points",
    "classify_exact_periodic_surface_region_points",
    "certify_exact_periodic_surface_path_simplicity",
    "check_exact_periodic_surface_path_contact",
    "relate_exact_periodic_surface_paths",
    "exact_periodic_surface_contour_forest",
    "exact_periodic_surface_region_area",
    "exact_periodic_surface_region_component_centroids",
    "exact_periodic_surface_region_component_inertia",
    "evaluate_exact_wrapped_surface_point",
    "evaluate_exact_wrapped_surface_points",
    "lift_exact_periodic_surface_path",
    "exact_periodic_surface_path_winding",
    "exact_periodic_surface_path_signed_area",
    "exact_periodic_surface_path_centroid",
    "exact_periodic_surface_path_inertia",
];

fn ratio(x: BigRational) -> Value {
    json!({"numerator":x.numer().to_string(),"denominator":x.denom().to_string()})
}

fn centroid(p: PeriodicPathCentroidReport) -> Value {
    json!({"signedArea":p.signed_area.map(ratio),"centroid":p.centroid.map(|v|v.map(ratio)),
        "workUsed":p.work_used,"refusedCoordinate":p.refused_coordinate,"refusal":p.refusal.map(|r|format!("{r:?}"))})
}

pub(super) fn dispatch(request: &Value, op: &str) -> Result<Value> {
    let surface = field(request, "surface")?;
    let work = field(request, "max_work")?;
    if op == "evaluate_exact_wrapped_surface_point" {
        let p = evaluate_exact_wrapped_surface_point(&surface, field(request, "uv")?, work)?;
        return Ok(
            json!({"nativeUv":p.native_uv,"point":p.point,"workUsed":p.work_used,"refusal":p.refusal.map(|r|format!("{r:?}"))}),
        );
    }
    if REGION_OPS.contains(&op) {
        return region(request, op, &surface, work);
    }
    let path: Vec<[f64; 2]> = field(request, "parameters")?;
    Ok(match op {
        "evaluate_exact_wrapped_surface_points" => {
            let p = evaluate_exact_wrapped_surface_points(&surface, &path, work)?;
            json!({"samples":p.samples,"workUsed":p.work_used,"refusedSample":p.refused_sample,"refusal":p.refusal.map(|r|format!("{r:?}"))})
        }
        "lift_exact_periodic_surface_path" => {
            let p = lift_exact_periodic_surface_path(&surface, &path, work)?;
            json!({"parameters":p.parameters,"workUsed":p.work_used,"refusedCoordinate":p.refused_coordinate,"refusal":p.refusal.map(|r|format!("{r:?}"))})
        }
        "exact_periodic_surface_path_winding" => {
            let p = exact_periodic_surface_path_winding(&surface, &path, work)?;
            json!({"winding":p.winding,"workUsed":p.work_used,"refusedCoordinate":p.refused_coordinate,"refusal":p.refusal.map(|r|format!("{r:?}"))})
        }
        "exact_periodic_surface_path_signed_area" => {
            let p = exact_periodic_surface_path_signed_area(&surface, &path, work)?;
            json!({"signedArea":p.signed_area.map(ratio),"workUsed":p.work_used,"refusedCoordinate":p.refused_coordinate,"refusal":p.refusal.map(|r|format!("{r:?}"))})
        }
        "exact_periodic_surface_path_centroid" => {
            centroid(exact_periodic_surface_path_centroid(&surface, &path, work)?)
        }
        "exact_periodic_surface_path_inertia" => {
            let p = exact_periodic_surface_path_inertia(&surface, &path, work)?;
            let mut report = centroid(p.geometry);
            report["centralMoments"] = json!(p.central_moments.map(|v| v.map(ratio)));
            report
        }
        _ => unreachable!(),
    })
}

const REGION_OPS: &[&str] = &[
    "classify_exact_periodic_surface_path_point",
    "classify_exact_periodic_surface_path_points",
    "classify_exact_periodic_surface_region_points",
    "certify_exact_periodic_surface_path_simplicity",
    "check_exact_periodic_surface_path_contact",
    "relate_exact_periodic_surface_paths",
    "exact_periodic_surface_contour_forest",
    "exact_periodic_surface_region_area",
    "exact_periodic_surface_region_component_centroids",
    "exact_periodic_surface_region_component_inertia",
];

fn location(l: PeriodicPathLocation) -> &'static str {
    match l {
        PeriodicPathLocation::Outside => "outside",
        PeriodicPathLocation::Inside => "inside",
        PeriodicPathLocation::Boundary => "boundary",
    }
}
fn locations(v: Vec<PeriodicPathLocation>) -> Vec<&'static str> {
    v.into_iter().map(location).collect()
}
fn contact(c: PeriodicPathContact) -> Value {
    json!({"edges":c.edges,"turns":c.turns})
}
fn relation(r: PeriodicPathRelation) -> Value {
    match r {
        PeriodicPathRelation::FirstContainsSecond => json!({"kind":"firstContainsSecond"}),
        PeriodicPathRelation::SecondContainsFirst => json!({"kind":"secondContainsFirst"}),
        PeriodicPathRelation::Separate => json!({"kind":"separate"}),
        PeriodicPathRelation::BoundaryContact(c) => {
            json!({"kind":"boundaryContact","contact":contact(c)})
        }
        PeriodicPathRelation::NonSimple {
            contour,
            contact: c,
        } => json!({"kind":"nonSimple","contour":contour,"contact":contact(c)}),
    }
}
fn component(c: PeriodicRegionComponentCentroid) -> Value {
    json!({"outerContour":c.outer_contour,"area":ratio(c.area),"centroid":c.centroid.map(ratio)})
}
fn region(request: &Value, op: &str, surface: &Surface, work: u64) -> Result<Value> {
    Ok(match op {
        "classify_exact_periodic_surface_path_point" => {
            let path: Vec<[f64; 2]> = field(request, "uv_points")?;
            let p = classify_exact_periodic_surface_path_point(
                surface,
                &path,
                field(request, "query_uv")?,
                work,
            )?;
            json!({"workUsed":p.work_used,"refusal":p.refusal.map(|r|format!("{r:?}")),"location":p.location.map(location)})
        }
        "classify_exact_periodic_surface_path_points" => {
            let path: Vec<[f64; 2]> = field(request, "uv_points")?;
            let queries: Vec<[f64; 2]> = field(request, "query_points")?;
            let p = classify_exact_periodic_surface_path_points(surface, &path, &queries, work)?;
            json!({"workUsed":p.work_used,"refusal":p.refusal.map(|r|format!("{r:?}")),"locations":p.locations.map(locations),"refusedQuery":p.refused_query})
        }
        "classify_exact_periodic_surface_region_points" => {
            let data: Vec<Vec<[f64; 2]>> = field(request, "contours")?;
            let contours: Vec<_> = data.iter().map(Vec::as_slice).collect();
            let queries: Vec<[f64; 2]> = field(request, "query_points")?;
            let p =
                classify_exact_periodic_surface_region_points(surface, &contours, &queries, work)?;
            json!({"workUsed":p.work_used,"refusal":p.refusal.map(|r|format!("{r:?}")),"locations":p.locations.map(locations),"refusedQuery":p.refused_query,"refusedLoop":p.refused_loop})
        }
        "certify_exact_periodic_surface_path_simplicity" => {
            let path: Vec<[f64; 2]> = field(request, "uv_points")?;
            let p = certify_exact_periodic_surface_path_simplicity(surface, &path, work)?;
            json!({"workUsed":p.work_used,"refusal":p.refusal.map(|r|format!("{r:?}")),"simple":p.simple,"contact":p.contact.map(contact)})
        }
        "check_exact_periodic_surface_path_contact" => {
            let first: Vec<[f64; 2]> = field(request, "first_path")?;
            let second: Vec<[f64; 2]> = field(request, "second_path")?;
            let p = check_exact_periodic_surface_path_contact(surface, &first, &second, work)?;
            json!({"workUsed":p.work_used,"refusal":p.refusal.map(|r|format!("{r:?}")),"disjoint":p.disjoint,"contact":p.contact.map(contact)})
        }
        "relate_exact_periodic_surface_paths" => {
            let first: Vec<[f64; 2]> = field(request, "first_path")?;
            let second: Vec<[f64; 2]> = field(request, "second_path")?;
            let p = relate_exact_periodic_surface_paths(surface, &first, &second, work)?;
            json!({"workUsed":p.work_used,"refusal":p.refusal.map(|r|format!("{r:?}")),"relation":p.relation.map(relation)})
        }
        "exact_periodic_surface_contour_forest" => {
            let data: Vec<Vec<[f64; 2]>> = field(request, "contours")?;
            let contours: Vec<_> = data.iter().map(Vec::as_slice).collect();
            let p = exact_periodic_surface_contour_forest(surface, &contours, work)?;
            json!({"workUsed":p.work_used,"refusal":p.refusal.map(|r|format!("{r:?}")),"nodes":p.nodes.map(|v|v.into_iter().map(|n|json!({"parent":n.parent,"depth":n.depth})).collect::<Vec<_>>())})
        }
        "exact_periodic_surface_region_area" => {
            let data: Vec<Vec<[f64; 2]>> = field(request, "contours")?;
            let contours: Vec<_> = data.iter().map(Vec::as_slice).collect();
            let p = exact_periodic_surface_region_area(surface, &contours, work)?;
            json!({"workUsed":p.work_used,"refusal":p.refusal.map(|r|format!("{r:?}")),"area":p.area.map(ratio)})
        }
        "exact_periodic_surface_region_component_centroids" => {
            let data: Vec<Vec<[f64; 2]>> = field(request, "contours")?;
            let contours: Vec<_> = data.iter().map(Vec::as_slice).collect();
            let p = exact_periodic_surface_region_component_centroids(surface, &contours, work)?;
            json!({"workUsed":p.work_used,"refusal":p.refusal.map(|r|format!("{r:?}")),"components":p.components.map(|v|v.into_iter().map(component).collect::<Vec<_>>())})
        }
        "exact_periodic_surface_region_component_inertia" => {
            let data: Vec<Vec<[f64; 2]>> = field(request, "contours")?;
            let contours: Vec<_> = data.iter().map(Vec::as_slice).collect();
            let p = exact_periodic_surface_region_component_inertia(surface, &contours, work)?;
            json!({"workUsed":p.work_used,"refusal":p.refusal.map(|r|format!("{r:?}")),"components":p.components.map(|v|v.into_iter().map(|c|{let mut row=component(c.geometry);row["centralMoments"]=json!(c.central_moments.map(ratio));row}).collect::<Vec<_>>())})
        }
        _ => unreachable!(),
    })
}
