use crate::*;
use cad_step::{export_named_step_assembly,import_named_step_assembly,import_step_shared_assembly,NamedStepAssembly};

pub(super) const OPS:&[&str]=&["name_step_parts","read_step_names","step_parts_list","compress_step_definition","select_step_parts",
    "name_step_variant","append_named_parts","consolidate_named_parts","compare_parts_lists"];

fn write(assembly:&NamedStepAssembly)->Result<String> {
    let labels:Vec<_>=assembly.definition_names.iter().map(String::as_str).collect();
    Ok(export_named_step_assembly(&assembly.assembly,&assembly.name,&labels)?)
}

pub(super) fn apply(v:&Value)->Result<Value>{
    let text=v["step"].as_str().ok_or_else(||input("Expected a STEP assembly"))?;
    let op=v["op"].as_str().unwrap_or("");
    let mut assembly=if op=="name_step_parts"{
        NamedStepAssembly{assembly:import_step_shared_assembly(text)?,name:field(v,"product_name")?,definition_names:field(v,"part_names")?}
    }else{import_named_step_assembly(text)?};
    let mut report=json!({});
    let output=match op{
        "name_step_parts"=>write(&assembly)?,
        "read_step_names"=>text.to_owned(),
        "step_parts_list"=>{
            let rows:Vec<_>=assembly.surface_usage()?.iter().map(|row| json!({
                "name":row.part.name,
                "definition":row.part.definition,
                "quantity":row.part.quantity,
                "storedFaces":row.stored_faces,
                "placedFaces":row.placed_faces,
                "storedPoles":row.stored_poles,
                "placedPoles":row.placed_poles,
            })).collect();
            report["surfaceUsage"]=json!(rows);
            text.to_owned()
        }
        "compress_step_definition"=>{
            let definition=field(v,"definition")?;
            let faces:Vec<usize>=field(v,"faces")?;
            let max_work=field(v,"max_work")?;
            let accepted=assembly.assembly.compress_definition_faces(definition,&faces,max_work)?;
            report["definition"]=json!(definition);
            report["accepted"]=json!(accepted);
            if accepted { write(&assembly)? } else { text.to_owned() }
        }
        "select_step_parts"=>assembly.export_selected_parts(&field::<Vec<usize>>(v,"definition_indices")?,&field::<String>(v,"product_name")?)?,
        "name_step_variant"=>{
            let id=assembly.replace_named_variant(&field::<Vec<usize>>(v,"selection")?,field(v,"model")?,field(v,"product_name")?)?;
            report["variantDefinition"]=json!(id);
            write(&assembly)?
        }
        "append_named_parts"=>{
            let other=import_named_step_assembly(v["otherStep"].as_str().ok_or_else(||input("Expected second STEP assembly"))?)?;
            let range=assembly.append_named_assembly(other,field(v,"placement")?)?;
            report["insertedRange"]=json!([range.start,range.end]);
            write(&assembly)?
        }
        "consolidate_named_parts"=>{
            report["removedDefinitions"]=json!(assembly.consolidate_named_parts()?);
            write(&assembly)?
        }
        "compare_parts_lists"=>{
            let other=import_named_step_assembly(v["otherStep"].as_str().ok_or_else(||input("Expected revised STEP assembly"))?)?;
            let changes:Vec<_>=assembly.parts_list_changes(&other)?.into_iter().map(|r|
                json!({"name":r.name,"before":r.before,"after":r.after,"beforeDefinition":r.before_definition,"afterDefinition":r.after_definition})).collect();
            report["changes"]=json!(changes);
            text.to_owned()
        }
        _=>return Err(input("Unknown parts list operation")),
    };
    let restored=import_named_step_assembly(&output)?;
    let rows:Vec<_>=restored.parts_list()?.iter().map(|r|json!({"name":r.name,"definition":r.definition,"quantity":r.quantity})).collect();
    report["parts"]=json!(rows);
    report["assemblyName"]=json!(restored.name);
    report["definitionNames"]=json!(restored.definition_names);
    report["occurrenceCount"]=json!(restored.assembly.occurrences().len());
    report["model"]=json!(restored.assembly.materialize(field(v,"preview_index")?)?);
    report["step"]=json!(output);
    Ok(report)
}
