use crate::*;
use cad_step::step_interchange_v3::{StepSemanticMetadata,StepProductMetadata,StepPresentationStyle,StepColour,export_step_v9_with_metadata_shared_points,import_step_v9_with_metadata};

pub(super) fn export(v:&Value)->Result<Value>{
    let model:brep_core::Model=field(v,"model")?;
    if model.bodies.len()!=1 {return Err(input("Compact product exchange requires a single body"));}
    let decorated=v["op"].as_str()==Some("compact_product_step");
    let step=if decorated {
        let face:usize=field(v,"face_index")?;
        if face>=model.faces.len(){return Err(input("Selected STEP face is outside the model"));}
        let rgb:[f64;3]=field(v,"colour_rgb")?;
        let name:String=field(v,"product_name")?;
        let material:String=field(v,"material_name")?;
        let layer:String=field(v,"layer_name")?;
        let mut meta=StepSemanticMetadata::empty(&model);
        meta.xde.products.insert(1,StepProductMetadata{entity:1,id:name.clone(),name,material:Some(material),..Default::default()});
        meta.xde.body_products[0]=Some(1);
        meta.presentation.body_products[0]=Some(1);
        meta.presentation.face_styles[face]=Some(StepPresentationStyle{
            layers:vec![layer],colour:Some(StepColour::new(rgb[0],rgb[1],rgb[2])?),..Default::default()});
        export_step_v9_with_metadata_shared_points(&model,&meta)?.0
    }else{cad_step::export_step_v9_shared_points(&model)?.0};
    let (restored,_,_,meta)=import_step_v9_with_metadata(&step)?;
    let validated=restored.validate()?;
    let mut report=json!({"stepBytes":step.len(),"bodies":restored.bodies.len(),"faces":restored.faces.len(),
        "boundaryEdges":validated.boundary_edge_count,"sharedPoints":step.matches("CARTESIAN_POINT(").count()});
    if decorated{
        let id=meta.xde.body_products.first().copied().flatten().ok_or_else(||input("Missing restored STEP product"))?;
        let product=meta.xde.products.get(&id).ok_or_else(||input("Missing restored STEP product metadata"))?;
        let face=field(v,"face_index")?;
        let style=meta.presentation.face_style(face,0).ok_or_else(||input("Missing restored STEP face style"))?;
        report["productName"]=json!(product.name);
        report["materialName"]=json!(product.material);
        report["layers"]=json!(style.layers);
        if let Some(c)=&style.colour{report["colourRgb"]=json!([c.red,c.green,c.blue]);}
    }
    report["step"]=json!(step);
    Ok(report)
}

pub(super) fn compress_brep(v: &Value) -> Result<Value> {
    let model: brep_core::Model = field(v, "model")?;
    let max_work = field(v, "max_work")?;
    let compressed = if v["op"].as_str() == Some("compress_brep_faces") {
        model.compress_face_surfaces(&field::<Vec<usize>>(v, "faces")?, max_work)?
    } else {
        model.compress_edge_with_trims(field(v, "edge_index")?, max_work)?
    };
    let accepted = compressed.is_some();
    Ok(json!({"model":compressed.unwrap_or(model),"accepted":accepted}))
}
