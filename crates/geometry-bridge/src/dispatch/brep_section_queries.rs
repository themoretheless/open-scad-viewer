use crate::*;
use brep_core::analysis as a;
use num_rational::BigRational as Q;
fn invalid() -> nurbs_core::Error {
    nurbs_core::Error::new(
        "BREP_INVALID_INPUT",
        "Expected finite 3D coordinates or rational numerator/denominator strings",
    )
}
pub(super) fn coordinate(v: &Value) -> Result<Q> {
    if let Some(x) = v.as_f64() {
        return Q::from_float(x).ok_or_else(invalid);
    }
    let text = v.as_str().filter(|s| s.len() <= 5000).ok_or_else(invalid)?;
    let (num, den) = text.split_once('/').unwrap_or((text, "1"));
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(num.strip_prefix('-').unwrap_or(num))
        || !digits(den)
        || !den.bytes().any(|b| b != b'0')
    {
        return Err(invalid());
    }
    text.parse().map_err(|_| invalid())
}
pub(super) fn point(v: &Value) -> Result<[Q; 3]> {
    let p = v.as_array().filter(|p| p.len() == 3).ok_or_else(invalid)?;
    Ok([coordinate(&p[0])?, coordinate(&p[1])?, coordinate(&p[2])?])
}
fn boundary(n: a::PolyhedralSectionNearestPoint) -> Value {
    json!({"kind":"boundary","pointMm":n.point_mm.map(|q|q.to_string()),"distanceSquaredMm2":n.distance_squared_mm2.to_string(),"ring":n.ring,"segment":n.segment,"face":n.face})
}
pub(super) fn measure(v: &Value, op: &str) -> Result<Value> {
    let solid = super::polyhedral_mass::qualify(
        field(v, "source")?,
        super::closed_ss::closed_solid_policy(field(v, "tolerance_uv")?),
    )?;
    let model = solid.model();
    let normal = field(v, "plane_normal")?;
    let offset = field(v, "plane_offset_mm")?;
    let work = field(v, "max_work")?;
    Ok(match op {
        "locate_section_point" => {
            let r = a::exact_polyhedral_section_point(
                &solid,
                normal,
                offset,
                &point(&v["query_point_mm"])?,
                work,
            )?;
            use a::PolyhedralSectionPointLocation as L;
            let location = match r.location {
                L::OffPlane => json!({"kind":"offPlane"}),
                L::Material => json!({"kind":"material"}),
                L::Outside => json!({"kind":"outside"}),
                L::Boundary {
                    ring,
                    segment,
                    face,
                } => json!({"kind":"boundary","ring":ring,"segment":segment,"face":face}),
            };
            json!({"model":model,"location":location,"workUsed":r.exact_work})
        }
        "measure_section_contour_distance" => {
            let r = a::exact_polyhedral_section_distance(
                &solid,
                normal,
                offset,
                &point(&v["query_point_mm"])?,
                work,
            )?;
            json!({"model":model,"nearest":r.nearest.map(boundary),"workUsed":r.exact_work})
        }
        "measure_section_material_distance" => {
            let r = a::exact_polyhedral_section_material_distance(
                &solid,
                normal,
                offset,
                &point(&v["query_point_mm"])?,
                work,
            )?;
            let nearest=r.nearest.map(|n|match n{a::PolyhedralSectionMaterialWitness::Boundary(b)=>boundary(b),a::PolyhedralSectionMaterialWitness::Interior{point_mm,distance_squared_mm2}=>json!({"kind":"interior","pointMm":point_mm.map(|q|q.to_string()),"distanceSquaredMm2":distance_squared_mm2.to_string()})});
            json!({"model":model,"nearest":nearest,"workUsed":r.exact_work})
        }
        _ => {
            let raw = v["query_path_mm"]
                .as_array()
                .filter(|p| (2..=4096).contains(&p.len()))
                .ok_or_else(invalid)?;
            let points = raw.iter().map(point).collect::<Result<Vec<_>>>()?;
            if op == "clip_section_segment" {
                let ends = points.try_into().map_err(|_| invalid())?;
                let r = a::exact_polyhedral_section_clip(&solid, normal, offset, &ends, work)?;
                json!({"model":model,"intervals":r.intervals.into_iter().map(|i|i.map(|q|q.to_string())).collect::<Vec<_>>(),"workUsed":r.exact_work})
            } else if op == "clip_section_polyline" {
                let r = a::exact_polyhedral_section_clip_polyline(
                    &solid, normal, offset, &points, work,
                )?;
                json!({"model":model,"spanIntervals":r.span_intervals.into_iter().map(|s|s.into_iter().map(|i|i.map(|q|q.to_string())).collect::<Vec<_>>()).collect::<Vec<_>>(),"workUsed":r.exact_work})
            } else {
                let r = a::bounded_polyhedral_section_path_length(
                    &solid,
                    normal,
                    offset,
                    &points,
                    field(v, "max_width_mm")?,
                    work,
                )?;
                json!({"model":model,"lengthBoundsMm":r.bounds_mm.map(|q|q.to_string()),"workUsed":r.exact_work})
            }
        }
    })
}
