use crate::*;
use brep_core::{analysis, closed_solid};

pub(super) fn measure(v: &Value, op: &str) -> Result<Value> {
    let source: brep_core::Model = field(v, "source")?;
    let policy = super::closed_ss::closed_solid_policy(field(v, "tolerance_uv")?);
    let solid = qualify(source, policy)?;
    let work = field(v, "max_work")?;
    let model = solid.model();
    Ok(match op {
        "project_polyhedral_body" => {
            let r = analysis::exact_polyhedral_projection(&solid, field(v, "direction")?, work)?;
            json!({"model":model,"dotBounds":r.dot_bounds.map(|x|x.to_string()),"witnessesMm":r.witnesses_mm.map(|p|p.map(|x|x.to_string())),"workUsed":r.exact_work})
        }
        "measure_polyhedral_frame" => {
            let r = analysis::exact_polyhedral_frame_bounds(
                &solid,
                field(v, "origin_mm")?,
                field(v, "frame_basis")?,
                work,
            )?;
            json!({"model":model,"frameBounds":r.bounds.map(|b|b.map(|x|x.to_string())),"witnessesMm":r.witnesses_mm.map(|b|b.map(|p|p.map(|x|x.to_string()))),"workUsed":r.exact_work})
        }
        "measure_polyhedral_axis_gap" => {
            let other = qualify(field(v, "target")?, policy)?;
            let r = analysis::exact_polyhedral_axis_separation(
                &solid,
                &other,
                field(v, "direction")?,
                work,
            )?;
            json!({"model":model,"signedGap":r.signed_gap.to_string(),"planeDistanceSquaredMm2":r.separating_plane_distance_squared_mm2.map(|x|x.to_string()),"witnessesMm":r.witnesses_mm.map(|p|p.map(|x|x.to_string())),"workUsed":r.exact_work})
        }
        "classify_polyhedral_plane" => {
            let r = analysis::exact_polyhedral_plane_relation(
                &solid,
                field(v, "plane_normal")?,
                field(v, "plane_offset_mm")?,
                work,
            )?;
            json!({"model":model,"relation":format!("{:?}",r.relation),"residualBounds":r.residual_bounds.map(|x|x.to_string()),"distanceSquaredMm2":r.distance_squared_mm2.to_string(),"witnessesMm":r.witnesses_mm.map(|p|p.map(|x|x.to_string())),"workUsed":r.exact_work})
        }
        "intersect_polyhedral_edges" => {
            let r = analysis::exact_polyhedral_edge_plane_intersection(
                &solid,
                field(v, "plane_normal")?,
                field(v, "plane_offset_mm")?,
                work,
            )?;
            let hits=r.hits.into_iter().map(|h|match h.contact {
                analysis::PolyhedralEdgePlaneContact::Point{fraction,point_mm}=>json!({"edge":h.edge,"span":h.span,"kind":"point","fraction":fraction.to_string(),"pointMm":point_mm.map(|x|x.to_string())}),
                analysis::PolyhedralEdgePlaneContact::Coplanar{endpoints_mm}=>json!({"edge":h.edge,"span":h.span,"kind":"coplanar","endpointsMm":endpoints_mm.map(|p|p.map(|x|x.to_string()))})
            }).collect::<Vec<_>>();
            json!({"model":model,"hits":hits,"workUsed":r.exact_work})
        }
        "measure_polyhedral_volume" => {
            let r = analysis::exact_polyhedral_volume(&solid, work)?;
            json!({"model":model,"volumeMm3":r.volume_mm3.to_string(),"workUsed":r.exact_work})
        }
        "measure_polyhedral_centroid" => {
            let r = analysis::exact_polyhedral_centroid(&solid, work)?;
            json!({"model":model,"volumeMm3":r.volume_mm3.to_string(),"centroidMm":r.centroid_mm.map(|q|q.to_string()),"workUsed":r.exact_work})
        }
        "measure_polyhedral_inertia" => {
            let r = analysis::exact_polyhedral_inertia(&solid, work)?;
            json!({"model":model,"volumeMm3":r.volume_mm3.to_string(),"centroidMm":r.centroid_mm.map(|q|q.to_string()),"inertiaMm5":r.inertia_mm5.map(|row|row.map(|q|q.to_string())),"workUsed":r.exact_work})
        }
        "measure_polyhedral_face_areas" => {
            let r = analysis::exact_polyhedral_area(&solid, work)?;
            json!({"model":model,"faceAreaSquaredMm4":r.face_area_squared_mm4.into_iter().map(|q|q.to_string()).collect::<Vec<_>>(),"workUsed":r.exact_work})
        }
        "bound_polyhedral_area" => {
            let r = analysis::bounded_polyhedral_area(&solid, field(v, "max_width_mm2")?, work)?;
            json!({"model":model,"areaBoundsMm2":r.bounds_mm2.map(|q|q.to_string()),"workUsed":r.exact_work})
        }
        "bound_polyhedral_edge_length" => {
            let r =
                analysis::bounded_polyhedral_edge_length(&solid, field(v, "max_width_mm")?, work)?;
            json!({"model":model,"edgeLengthBoundsMm":r.bounds_mm.map(|q|q.to_string()),"edgeCount":r.edge_count,"workUsed":r.exact_work})
        }
        _ => {
            return Err(nurbs_core::Error::new(
                "BREP_INVALID_INPUT",
                "Unknown polyhedral measurement",
            ));
        }
    })
}

pub(super) fn qualify(
    source: brep_core::Model,
    policy: closed_solid::AdmissionPolicy,
) -> Result<closed_solid::QualifiedClosedSolid> {
    closed_solid::qualify(source, policy)?
        .into_solid()
        .ok_or_else(|| {
            nurbs_core::Error::new(
                "BREP_CLOSED_SOLID_UNPROVED",
                "Polyhedral measurement requires a qualified material boundary",
            )
        })
}

pub(super) fn measure_section(v: &Value, op: &str) -> Result<Value> {
    let solid = qualify(
        field(v, "source")?,
        super::closed_ss::closed_solid_policy(field(v, "tolerance_uv")?),
    )?;
    let normal = field(v, "plane_normal")?;
    let offset = field(v, "plane_offset_mm")?;
    let work = field(v, "max_work")?;
    let model = solid.model();
    let loops_json = |rings: Vec<analysis::PolyhedralSectionLoop>| {
        rings.into_iter().map(|ring|json!({"faces":ring.faces,"pointsMm":ring.points_mm.into_iter().map(|p|p.map(|x|x.to_string())).collect::<Vec<_>>()})).collect::<Vec<_>>()
    };
    Ok(match op {
        "section_polyhedral_faces" => {
            let r = analysis::exact_polyhedral_transverse_section(&solid, normal, offset, work)?;
            let faces=r.faces.into_iter().map(|f|json!({"face":f.face,"segmentsMm":f.segments_mm.into_iter().map(|seg|seg.map(|p|p.map(|x|x.to_string()))).collect::<Vec<_>>()})).collect::<Vec<_>>();
            json!({"model":model,"faces":faces,"workUsed":r.exact_work})
        }
        "assemble_polyhedral_section_loops" => {
            let r = analysis::exact_polyhedral_section_loops(&solid, normal, offset, work)?;
            json!({"model":model,"loops":loops_json(r.loops),"workUsed":r.exact_work})
        }
        "measure_polyhedral_section_area" => {
            let r = analysis::exact_polyhedral_section_area(&solid, normal, offset, work)?;
            json!({"model":model,"areaSquaredMm4":r.area_squared_mm4.to_string(),"depths":r.depths,"loops":loops_json(r.loops),"workUsed":r.exact_work})
        }
        "measure_polyhedral_section_centroid" => {
            let r = analysis::exact_polyhedral_section_centroid(&solid, normal, offset, work)?;
            json!({"model":model,"centroidMm":r.centroid_mm.map(|p|p.map(|x|x.to_string())),"areaSquaredMm4":r.section.area_squared_mm4.to_string(),"depths":r.section.depths,"loops":loops_json(r.section.loops),"workUsed":r.section.exact_work})
        }
        "bound_polyhedral_section_perimeter" => {
            let r = analysis::bounded_polyhedral_section_perimeter(
                &solid,
                normal,
                offset,
                field(v, "max_width_mm")?,
                work,
            )?;
            json!({"model":model,"perimeterBoundsMm":r.bounds_mm.map(|x|x.to_string()),"loops":loops_json(r.loops),"workUsed":r.exact_work})
        }
        _ => {
            return Err(nurbs_core::Error::new(
                "BREP_INVALID_INPUT",
                "Unknown section measurement",
            ));
        }
    })
}
