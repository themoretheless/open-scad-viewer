use crate::*;

pub(super) const OPS: &[&str] = &["make_step_assembly", "step_part_tolerances", "set_scaled_step_placement", "step_occurrence_bounds", "step_proximity_pairs", "step_common_material", "step_interferences", "step_material_volume", "step_material_centroid", "step_material_inertia", "step_material_area", "step_principal_inertia", "step_mass_properties"];

fn from_parts(v:&Value)->Result<cad_step::StepSharedAssembly> {
    let mut models:Vec<brep_core::Model>=field(v,"models")?;
    let tolerances:Vec<f64>=field(v,"definition_tolerances_mm")?;
    if !tolerances.is_empty() {
        if tolerances.len()!=models.len() {return Err(input("Supply one tolerance per definition"));}
        for (m,t) in models.iter_mut().zip(tolerances) {
            if !t.is_finite() || t<=0. {return Err(input("Expected a positive definition tolerance"));}
            m.tolerance_mm=t;
        }
    }
    let ids:Vec<usize>=field(v,"definition_indices")?;
    let matrices:Vec<[[f64;4];4]>=field(v,"placements")?;
    if ids.len()!=matrices.len() {return Err(input("Supply one placement per occurrence"));}
    let placements=ids.into_iter().zip(matrices).map(|(definition,transform)|cad_step::StepAssemblyOccurrence{definition,transform}).collect();
    Ok(cad_step::StepSharedAssembly::from_placed_solids(models,placements)?)
}

pub(super) fn apply(v:&Value)->Result<Value> {
    let op=v["op"].as_str().unwrap_or("");
    let mut a=if op=="make_step_assembly" {from_parts(v)?} else {
        cad_step::import_step_shared_assembly(v["step"].as_str().ok_or_else(||input("Expected a STEP assembly"))?)?
    };
    let tolerance=||field::<f64>(v,"relative_tolerance");
    let budget=||field::<usize>(v,"max_evaluations");
    let preview=field(v,"preview_index")?;
    let mut common=None;
    let report=match op {
        "make_step_assembly" => json!({}),
        "set_scaled_step_placement" => {
            let i=field(v,"index")?;
            a.set_occurrence_orthogonal_placement(i,field(v,"placement")?)?;
            json!({"placement":a.occurrences()[i].transform})
        }
        "step_part_tolerances"=>json!({"tolerances":a.definitions().iter().map(|m|m.tolerance_mm).collect::<Vec<_>>()}),
        "step_occurrence_bounds"=>json!({"bounds":a.occurrence_bounds()?}),
        "step_proximity_pairs"=>json!({"pairs":a.proximity_candidates(field(v,"distance_mm")?)?}),
        "step_common_material"=>{
            let [first,second]:[usize;2]=field(v,"pair")?;
            common=Some(a.intersect_occurrences(first,second)?);
            json!({"pair":[first,second]})
        }
        "step_interferences"=>{
            let pairs:Vec<_>=a.check_interferences(field(v,"max_pairs")?)?.into_iter().map(|(pair,r)|match r{
                Ok(hit)=>json!({"pair":pair,"interferes":hit}),
                Err(error)=>json!({"pair":pair,"error":error.to_string()}),
            }).collect();
            json!({"pairs":pairs})
        }
        "step_material_volume"=>json!({"volumeMm3":a.material_volume_mm3(tolerance()?,budget()?)?}),
        "step_material_centroid"=>json!({"centroidMm":a.material_centroid_mm(tolerance()?,budget()?)?}),
        "step_material_inertia"=>json!({"inertiaMm5":a.material_inertia_mm5(tolerance()?,budget()?)?}),
        "step_material_area"=>json!({"areaMm2":a.material_surface_area_mm2(tolerance()?,budget()?)?}),
        "step_principal_inertia"=>{
            let r=a.material_principal_inertia(tolerance()?,budget()?)?;json!({"momentsMm5":r.moments_mm5,"axes":r.axes})
        }
        "step_mass_properties"=>{
            let r=a.mass_properties_with_densities(&field::<Vec<f64>>(v,"densities")?,tolerance()?,budget()?)?;
            json!({"massKg":r.mass,"centroidMm":r.centroid,"inertiaKgMm2":r.inertia})
        }
        _=>return Err(input("Unknown shared assembly property")),
    };
    let (model,step)=match common {
        Some(model)=>(model,None),
        None=>(a.materialize(preview)?,Some(cad_step::export_step_shared_assembly(&a)?)),
    };
    let mut result=report;
    result["model"]=encode(model)?;
    result["step"]=json!(step);
    result["definitionCount"]=json!(a.definitions().len());
    result["occurrenceCount"]=json!(a.occurrences().len());
    Ok(result)
}
