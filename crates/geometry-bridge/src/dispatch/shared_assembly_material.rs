use crate::*;

pub(super) const OPS: &[&str] = &["step_cut_stock", "step_fuse_instances", "step_revision_delta", "step_split_instance", "step_stock_delta", "step_slice_layers", "step_clip_material", "step_free_space", "step_interference_regions", "step_interference_material", "step_interference_groups", "step_group_interferences"];

fn selected<T>(items:Vec<T>,piece:usize)->Result<T> {
    items.into_iter().nth(piece).ok_or_else(||input("Selected result index is out of range"))
}
fn delta(d:cad_step::StepMaterialDelta,piece:usize)->Result<brep_core::Model> {
    match piece {0=>Ok(d.added),1=>Ok(d.removed),_=>Err(input("Select added (0) or removed (1) material"))}
}

pub(super) fn apply(v:&Value)->Result<Value> {
    let a=cad_step::import_step_shared_assembly(v["step"].as_str().ok_or_else(||input("Expected a STEP assembly"))?)?;
    let op=v["op"].as_str().unwrap_or("");
    let selection=||field::<Vec<usize>>(v,"selection");
    let piece=||field::<usize>(v,"piece");
    let (model,mut report)=match op {
        "step_cut_stock"=>(a.cut_stock(field(v,"stock")?,&selection()?)?,json!({"cutters":selection()?})),
        "step_fuse_instances"=>(a.fuse_occurrences(&selection()?)?,json!({"selection":selection()?})),
        "step_revision_delta"|"step_stock_delta"=>{
            let d=if op=="step_stock_delta" {a.stock_layout_delta(field(v,"stock")?,&selection()?)?}
                else {a.material_delta(field(v,"before")?,field(v,"after")?)?};
            let index=piece()?;
            (delta(d,index)?,json!({"added":index==0}))
        }
        "step_split_instance"|"step_slice_layers"=>{
            let index=field(v,"index")?;
            let axis=field(v,"axis")?;
            let parts=if op=="step_split_instance" {a.split_at_coordinate(index,axis,field(v,"coordinate_mm")?)?.into()}
                else{a.coordinate_layers(index,axis,&field::<Vec<f64>>(v,"cuts_mm")?)?};
            let count=parts.len();let i=piece()?;
            (selected(parts,i)?,json!({"pieces":count,"selected":i}))
        }
        "step_clip_material"=>{
            let [lo,hi]=field(v,"bounds_mm")?;
            let hits=a.material_in_box(lo,hi,field(v,"max_candidates")?)?;
            let summary=hits.iter().map(|(i,result)|match result{
                Ok(_)=>json!({"occurrence":i}),Err(e)=>json!({"occurrence":i,"error":e.to_string()}),
            }).collect::<Vec<_>>();
            let (i,model)=selected(hits,piece()?)?;
            (model?,json!({"selectedOccurrence":i,"results":summary}))
        }
        "step_free_space"=>{
            let [lo,hi]=field(v,"bounds_mm")?;
            (a.free_space_in_box(lo,hi,field(v,"max_candidates")?)?,json!({"bounds":[lo,hi]}))
        }
        "step_interference_regions"=>{
            let hits=a.interference_regions(field(v,"max_pairs")?,field(v,"relative_tolerance")?,field(v,"max_evaluations")?)?;
            let summary=hits.iter().map(|(pair,result)|match result{
                Ok((_,r))=>json!({"pair":pair,"volumeMm3":r.signed_volume_mm3,"centroid":r.centroid}),
                Err(e)=>json!({"pair":pair,"error":e.to_string()}),
            }).collect::<Vec<_>>();
            let (pair,r)=selected(hits,piece()?)?;
            (r?.0,json!({"selectedPair":pair,"results":summary}))
        }
        "step_interference_material"=>(a.interference_material(field(v,"max_pairs")?)?,json!({})),
        "step_interference_groups"=>(a.materialize(field(v,"preview_index")?)?,json!({"groups":a.interference_groups(field(v,"max_pairs")?)?})),
        "step_group_interferences"=>{
            let hits=a.interferences_between(&field::<Vec<usize>>(v,"left_group")?,&field::<Vec<usize>>(v,"right_group")?,field(v,"max_pairs")?)?;
            let summary=hits.iter().map(|(pair,r)|match r{
                Ok(_)=>json!({"pair":pair}),Err(e)=>json!({"pair":pair,"error":e.to_string()}),
            }).collect::<Vec<_>>();
            let (pair,r)=selected(hits,piece()?)?;
            (r?,json!({"selectedPair":pair,"results":summary}))
        }
        _=>return Err(input("Unknown assembly material operation")),
    };
    report["bodyCount"]=json!(model.bodies.len());
    report["model"]=encode(model)?;
    Ok(report)
}
