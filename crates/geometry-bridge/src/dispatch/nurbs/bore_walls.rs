use crate::*;
use brep_core::body_analysis::{through_wall,through_wall_cylinder};
use brep_core::profile_certificate::original_restriction::plane::pullback::{stored_affine_loop,stored_affine_trim};

pub(super) const OPS:&[&str]=&["piecewise_axis_radius","bore_wall_faces","cap_edge_pcurve","transfer_cap_loop","replace_bore_wall","cut_bore_cylinder"];
pub(super) fn apply(v:&Value,op:&str)->Result<Value>{
    if op=="piecewise_axis_radius"{
        let axis=field(v,"axis_index")?;
        let bound=nurbs_core::radial_bounds::piecewise_axis_radius_bounds(&field(v,"surface")?,axis,
            field(v,"origin")?,field(v,"max_patches")?)?;
        return Ok(json!({"axisIndex":axis,"radiusBoundsMm":bound}));
    }
    if op=="cap_edge_pcurve" {
        let proof=stored_affine_trim(&field(v,"surface")?,&field(v,"edge")?,field(v,"tolerance_mm")?)?;
        proof.reverify()?;
        return Ok(json!({"pcurve":proof.pcurve()}));
    }
    let source:brep_core::Model=field(v,"model")?;
    if op=="transfer_cap_loop"{
        let z:f64=field(v,"source_height_mm")?;
        let mut caps=source.faces.iter().filter(|f|f.surface.control_points.iter().flatten().all(|p|p.len()==3 && p[2]==z));
        let cap=caps.next().ok_or_else(||input("Missing planar cap at the selected height"))?;
        if caps.next().is_some(){return Err(input("Expected one cap at the selected height"));}
        let wire=stored_affine_loop(&source,cap.outer,&field(v,"surface")?,field(v,"reverse")?,field(v,"tolerance_mm")?)?;
        return Ok(json!({"coedges":wire.coedges,"sourceLoop":cap.outer}));
    }
    let result=if op=="cut_bore_cylinder"{
        let cutter=brep_core::cylinder(field(v,"radius_mm")?,field(v,"height_mm")?)?;
        brep_core::boolean(&source,&cutter,"difference")?
    }else{
        let holed:Vec<_>=source.faces.iter().enumerate().filter_map(|(i,f)|(!f.holes.is_empty()).then_some(i)).collect();
        let ends:[usize;2]=field(v,"cap_indices")?;
        let cap=|i:usize|holed.get(i).copied().ok_or_else(||input("Selected holed cap does not exist"));
        let caps=[cap(ends[0])?,cap(ends[1])?];
        if op=="bore_wall_faces"{
            let holes:[usize;2]=field(v,"hole_indices")?;
            let faces=through_wall::between_holes(&source,field(v,"shell_index")?,[(caps[0],holes[0]),(caps[1],holes[1])])?;
            return Ok(json!({"wallFaces":faces,"capFaces":caps}));
        }
        through_wall_cylinder::replace(&source,caps,field(v,"radius_mm")?,field(v,"height_mm")?)?
    };
    let valid=result.validate()?;
    Ok(json!({"faceCount":result.faces.len(),"sourceFaceCount":source.faces.len(),"boundaryEdges":valid.boundary_edge_count,
        "bodyCount":result.bodies.len(),"model":result}))
}
