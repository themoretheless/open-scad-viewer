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
