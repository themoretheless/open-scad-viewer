use crate::*;
use cad_step::{export_named_styled_step_assembly,import_named_step_assembly,import_step_product_paths,
    import_step_occurrence_colours,StepOccurrenceColour};

pub(super) fn apply(v:&Value)->Result<Value>{
    let source=v["step"].as_str().ok_or_else(||input("Missing input STEP assembly"))?;
    let op=v["op"].as_str().unwrap_or("");
    let step=if op=="read_occurrence_styles"{source.to_owned()}else{
        let scene=import_named_step_assembly(source)?;
        let groups:Vec<String>=field(v,"group_names")?;
        let groups:Vec<_>=groups.iter().map(String::as_str).collect();
        let placement_groups:Vec<i32>=field(v,"parent_indices")?;
        let parents=placement_groups.into_iter().map(|p|match p {
            -1 => Ok(None),
            0.. => Ok(Some(p as usize)),
            _ => Err(input("Expected parent group index or -1 for root")),
        }).collect::<Result<Vec<_>>>()?;
        let names:Vec<String>=if op=="style_step_occurrences"{field(v,"style_names")?}else{Vec::new()};
        let mut colours=std::iter::repeat_with(||None).take(scene.assembly.occurrences().len()).collect::<Vec<_>>();
        if op=="style_step_occurrences"{
            let rgbs:Vec<[f64;3]>=field(v,"style_colours")?;
            let selected:Vec<usize>=field(v,"selection")?;
            if selected.len()!=rgbs.len()||rgbs.len()!=names.len(){return Err(input("Expected one named colour for each selected occurrence"));}
            for ((i,name),rgb) in selected.into_iter().zip(&names).zip(rgbs){
                let slot=colours.get_mut(i).ok_or_else(||input("Occurrence index outside assembly"))?;
                if slot.is_some(){return Err(input("Occurrence specified twice"));}
                *slot=Some(StepOccurrenceColour{name,rgb});
            }
        }
        export_named_styled_step_assembly(&scene,&groups,&parents,&colours)?
    };
    let (scene,paths)=import_step_product_paths(&step)?;
    let colours=import_step_occurrence_colours(&step)?;
    let locations:Vec<_>=scene.assembly.occurrences().iter().map(|p|[p.transform[0][3],p.transform[1][3],p.transform[2][3]]).collect();
    let model=scene.assembly.materialize(field(v,"preview_index")?)?;
    Ok(json!({"model":model,"paths":paths,"colours":colours,"origins":locations,
        "definitionCount":scene.assembly.definitions().len(),"step":step}))
}

pub(super) const BRANCH_OPS: &[&str] = &[
    "export_step_branch",
    "replace_step_branch_leaf",
    "move_step_branch",
    "step_branch_parts",
    "step_branch_bounds",
    "step_branch_mass",
];

pub(super) fn branch(v: &Value) -> Result<Value> {
    let source = v["step"].as_str().ok_or_else(|| input("Missing input STEP assembly"))?;
    let path: Vec<String> = field(v, "product_path")?;
    let path: Vec<_> = path.iter().map(String::as_str).collect();
    let mut report = json!({});
    let updated = match v["op"].as_str().unwrap_or("") {
        "export_step_branch" => Some(cad_step::export_step_product_subassembly(source, &path)?),
        "replace_step_branch_leaf" => Some(cad_step::replace_step_product_at_path(
            source, &path, field(v, "model")?, &field::<String>(v, "product_name")?,
        )?),
        "move_step_branch" => Some(cad_step::move_step_product_branch(source, &path, field(v, "placement")?)?),
        "step_branch_parts" => {
            report["parts"] = json!(cad_step::step_branch_parts_list(source, &path)?
                .into_iter().map(|row| json!({
                    "definition": row.definition, "name": row.name, "inside": row.inside, "outside": row.outside,
                })).collect::<Vec<_>>());
            None
        }
        "step_branch_bounds" => {
            report["boundsMm"] = json!(cad_step::step_branch_bounds(source, &path)?);
            None
        }
        "step_branch_mass" => {
            let mut densities = field::<Vec<f64>>(v, "densities")?.into_iter();
            let properties = cad_step::step_branch_mass_properties(
                source, &path, field(v, "relative_tolerance")?, field(v, "max_evaluations")?,
                |_, _| densities.next().ok_or_else(|| input("Provide one density per selected STEP occurrence")),
            )?;
            if densities.next().is_some() {
                return Err(input("Provide one density per selected STEP occurrence"));
            }
            report["massKg"] = json!(properties.mass);
            report["centroidMm"] = json!(properties.centroid);
            report["inertiaKgMm2"] = json!(properties.inertia);
            None
        }
        _ => return Err(input("Unknown STEP branch operation")),
    };
    let step = updated.as_deref().unwrap_or(source);
    let (scene, paths) = import_step_product_paths(step)?;
    report["assemblyName"] = json!(scene.name);
    report["definitionCount"] = json!(scene.assembly.definitions().len());
    report["paths"] = json!(paths);
    report["colours"] = json!(import_step_occurrence_colours(step)?);
    report["placements"] = json!(scene.assembly.occurrences().iter().map(|part| part.transform).collect::<Vec<_>>());
    report["model"] = json!(scene.assembly.materialize(field(v, "preview_index")?)?);
    report["step"] = json!(step);
    Ok(report)
}
