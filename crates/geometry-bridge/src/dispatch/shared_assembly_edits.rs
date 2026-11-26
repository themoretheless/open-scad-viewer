use crate::*;

pub(super) const OPS: &[&str] = &["move_step_group","copy_step_group","replace_step_variant","read_shared_step", "replace_step_part", "replace_step_instance", "place_step_instance", "remove_step_instance", "add_step_instance", "insert_step_solid", "append_step_assembly", "deduplicate_step_parts", "extract_step_instances"];

pub(super) fn apply(v:&Value)->Result<Value> {
    let text=v["step"].as_str().ok_or_else(||input("Expected a STEP assembly"))?;
    let mut assembly=cad_step::import_step_shared_assembly(text)?;
    match v["op"].as_str().unwrap_or("") {
        "move_step_group" => assembly.move_occurrences(&field::<Vec<usize>>(v,"selection")?,field(v,"placement")?)?,
        "copy_step_group" => {assembly.copy_occurrences(&field::<Vec<usize>>(v,"selection")?,field(v,"placement")?)?;}
        "replace_step_variant" => {assembly.replace_variant(&field::<Vec<usize>>(v,"selection")?,field(v,"model")?)?;}
        "read_shared_step" => {}
        "replace_step_part" => assembly.replace_definition(field(v,"definition")?,field(v,"model")?)?,
        "replace_step_instance" => assembly.replace_occurrence_geometry(field(v,"index")?,field(v,"model")?)?,
        "place_step_instance" => assembly.set_occurrence_placement(field(v,"index")?,field(v,"placement")?)?,
        "remove_step_instance" => assembly.remove_occurrence(field(v,"index")?)?,
        "add_step_instance" => {assembly.add_occurrence(field(v,"definition")?,field(v,"placement")?)?;}
        "insert_step_solid" => {assembly.insert_solid(field(v,"model")?,field(v,"placement")?)?;}
        "append_step_assembly" => {
            let other=v["otherStep"].as_str().ok_or_else(||input("Expected another STEP assembly"))?;
            assembly.append_assembly(cad_step::import_step_shared_assembly(other)?,field(v,"placement")?)?;
        }
        "deduplicate_step_parts" => {assembly.deduplicate_definitions();}
        "extract_step_instances" => {assembly=assembly.extract_occurrences(&field::<Vec<usize>>(v,"selection")?)?;}
        _ => return Err(input("Unknown STEP assembly operation")),
    }
    let preview=field(v,"preview_index")?;
    let model=assembly.materialize(preview)?;
    let occurrences:Vec<_>=assembly.occurrences().iter().map(|o|json!({"definition":o.definition,"placement":o.transform})).collect();
    Ok(json!({"model":model,"step":cad_step::export_step_shared_assembly(&assembly)?,
        "definitionCount":assembly.definitions().len(),"occurrenceCount":occurrences.len(),"occurrences":occurrences}))
}
