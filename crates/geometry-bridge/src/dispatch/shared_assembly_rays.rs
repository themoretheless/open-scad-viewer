use crate::*;

pub(super) const OPS: &[&str] = &["step_ray_crossings", "step_first_ray_crossing", "step_ray_candidates", "step_ray_intervals", "step_ray_length", "qualify_step_instance", "step_ray_coverage", "step_ray_multiplicity", "step_pair_clearance", "step_clearance_checks", "step_minimum_clearance", "step_polyhedral_sections"];

fn crossing(c:brep_core::ray_parity::Crossing)->Value {
    json!({"face":c.face,"uv":c.uv,"parameter":c.parameter})
}
pub(super) fn apply(v:&Value)->Result<Value> {
    let text=v["step"].as_str().ok_or_else(||input("Expected a STEP assembly"))?;
    let assembly=cad_step::import_step_shared_assembly(text)?;
    let op=v["op"].as_str().unwrap_or("");
    let index=field(v,"preview_index")?;
    let tolerance=||field::<f64>(v,"tolerance_uv");
    let cells=||field::<usize>(v,"max_cells");
    let domains=||field::<usize>(v,"max_domain_cells");
    let policy=|| -> Result<_> { Ok(crate::dispatch::brep::closed_ss::closed_solid_policy(tolerance()?)) };
    let limits=|| -> Result<_> {Ok(brep_core::solid_distance::Limits {
        validity:policy()?.geometry,pairs:100,contact_pairs:1000,cells:cells()?,domain_cells:domains()?,
    })};
    let origin=||field::<[f64;3]>(v,"ray_origin");
    let direction=||field::<[f64;3]>(v,"ray_direction");
    let length=||field::<f64>(v,"max_parameter");
    let mut r=match op {
        "step_ray_candidates"=>json!({"candidates":assembly.ray_candidates(origin()?,direction()?)?.into_iter()
            .map(|(occurrence,parameter)|json!({"occurrence":occurrence,"parameter":parameter})).collect::<Vec<_>>()}),
        "step_ray_crossings"=>{
            let hits=assembly.ray_crossings(origin()?,direction()?,tolerance()?,cells()?,domains()?)?
                .into_iter().map(|(i,r)| match r {
                    Ok(r)=>json!({"occurrence":i,"parity":r.parity,"crossings":r.crossings.into_iter().map(crossing).collect::<Vec<_>>(),
                        "unresolved":r.unresolved.into_iter().map(|u|json!({"face":u.face,"uv":u.uv,"reason":u.reason})).collect::<Vec<_>>() }),
                    Err(e)=>json!({"occurrence":i,"error":e.to_string()}),
                }).collect::<Vec<_>>();
            json!({"hits":hits})
        }
        "step_first_ray_crossing"=>{
            let hit=assembly.first_ray_crossing(origin()?,direction()?,tolerance()?,cells()?,domains()?)?;
            json!({"first":hit.map(|(i,c)|json!({"occurrence":i,"crossing":crossing(c)}))})
        }
        "step_ray_intervals"|"step_ray_length"|"qualify_step_instance"=>{
            let solid=assembly.qualify_occurrence(index,policy()?)?;
            match op {
                "step_ray_intervals"=>json!({"intervals":solid.ray_material_intervals(origin()?,direction()?,cells()?,domains()?)?}),
                "step_ray_length"=>json!({"lengthMm":solid.ray_material_length_mm(origin()?,direction()?,length()?,cells()?,domains()?)?}),
                _=>json!({"proven":solid.geometry().proven,"boundaryProven":solid.geometry().boundary.proven,"occurrence":index}),
            }
        }
        "step_ray_coverage"|"step_ray_multiplicity"=>{
            let r=if op=="step_ray_coverage" {
                assembly.ray_material_coverage(origin()?,direction()?,length()?,policy()?,cells()?,domains()?)?
            }else{
                assembly.ray_material_coverage_at_least(field(v,"minimum_occurrences")?,origin()?,direction()?,length()?,policy()?,cells()?,domains()?)?
            };
            json!({"guaranteed":r.guaranteed,"possible":r.possible})
        }
        "step_pair_clearance"=>{
            let r=assembly.occurrence_clearance(field(v,"pair")?,field(v,"tolerance_mm")?,tolerance()?,limits()?)?;
            json!({"distanceMm":r.distance_interval_mm,"materialOverlap":r.material_overlap,"converged":r.converged,"reason":r.reason})
        }
        "step_clearance_checks"=>{
            let pairs=assembly.check_clearances(field(v,"minimum_mm")?,field(v,"tolerance_mm")?,tolerance()?,limits()?,field(v,"max_pairs")?)?
                .into_iter().map(|(pair,r)|match r {
                    Ok(r)=>json!({"pair":pair,"status":format!("{:?}",r.status),"distanceMm":r.measurement.distance_interval_mm}),
                    Err(e)=>json!({"pair":pair,"error":e.to_string()}),
                }).collect::<Vec<_>>();
            json!({"pairs":pairs})
        }
        "step_minimum_clearance"=>{
            let r=assembly.minimum_clearance(field(v,"tolerance_mm")?,tolerance()?,limits()?,field(v,"max_pairs")?)?;
            json!({"minimum":r.map(|r|json!({"lowerMm":r.lower_bound_mm,"upperMm":r.upper_bound_mm,"pair":r.upper_bound_pair,
                "checkedPairs":r.checked_pairs,"converged":r.converged,"errors":r.pair_errors.into_iter().map(|(pair,e)|json!({"pair":pair,"error":e.to_string()})).collect::<Vec<_>>() }))})
        }
        "step_polyhedral_sections"=>{
            let sections=assembly.exact_polyhedral_sections(field(v,"plane_normal")?,field(v,"plane_offset_mm")?,policy()?,field(v,"max_work")?)?
                .into_iter().map(|(i,r)|match r {
                    Ok(r)=>json!({"occurrence":i,"areaSquaredMm4":r.area_squared_mm4.to_string(),"depths":r.depths,
                        "loops":r.loops.into_iter().map(|l|json!({"faces":l.faces,"pointsMm":l.points_mm.into_iter().map(|p|p.map(|q|q.to_string())).collect::<Vec<_>>() })).collect::<Vec<_>>() }),
                    Err(e)=>json!({"occurrence":i,"error":e.to_string()}),
                }).collect::<Vec<_>>();
            json!({"sections":sections})
        }
        _=>return Err(input("Unknown STEP ray or clearance operation")),
    };
    r["model"]=encode(assembly.materialize(index)?)?;
    Ok(r)
}
