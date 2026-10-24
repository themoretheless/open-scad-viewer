use super::*;

impl Compiler {
    pub(super) fn operation(
        &mut self,
        name: &str,
        input: Option<String>,
        mut args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        let name = match name {
            "move" => "translate",
            "rect" => "rectangle",
            _ => name,
        };
        if name=="boundary_fill" {
            let at=args.iter().position(|(n,_)|n.as_deref()==Some("center")).ok_or_else(||crate::error("boundary_fill requires named center"))?;
            let center=raw(args.remove(at).1)?;
            if args.iter().any(|(n,_)|n.is_some()){return Err(crate::error("Boundary fill edges must be positional"));}
            let mut inputs=Vec::new();if let Some(id)=input{inputs.push(id);}
            for(_,value)in args{inputs.push(self.geometry(value)?);}
            if !(3..=32).contains(&inputs.len()){return Err(crate::error("Boundary fill needs 3..32 edges"));}
            return self.add(json!({"op":name,"inputs":inputs,"center":center}));
        }
        if name=="gordon_surface" {
            if input.is_some(){return Err(crate::error("Gordon families must be named"));}
            let mut node=json!({"op":name});
            for key in ["u_curves","v_curves"] {
                let at=args.iter().position(|(n,_)|n.as_deref()==Some(key)).ok_or_else(||crate::error(format!("gordon_surface requires named {key}")))?;
                let V::Array(values)=args.remove(at).1 else{return Err(crate::error("Gordon families must be curve arrays"));};
                let mut ids=Vec::new();for value in values{ids.push(self.geometry(value)?);}
                node[key]=json!(ids);
            }
            for key in ["parameters_u","parameters_v"] {
                let at=args.iter().position(|(n,_)|n.as_deref()==Some(key)).ok_or_else(||crate::error(format!("gordon_surface requires named {key}")))?;
                node[key]=raw(args.remove(at).1)?;
            }
            if !args.is_empty(){return Err(crate::error("Unexpected Gordon arguments"));}
            return self.add(node);
        }
        if ["loft_match_surface","brep_natural_loft","brep_capped_loft"].contains(&name) {
            let mut node=json!({"op":name});
            if name=="loft_match_surface" {
                let source=if let Some(id)=input {id} else {
                    if args.is_empty() || args[0].0.is_some(){return Err(crate::error("Loft match needs a positional surface"));}
                    self.geometry(args.remove(0).1)?
                };
                node["input"]=json!(source);
            } else if input.is_some(){return Err(crate::error("Capped loft uses named loop arrays"));}
            for (key,value) in args {
                let key=key.ok_or_else(||crate::error("Loft options must be named"))?;
                let depth=match (name,key.as_str()) {
                    ("brep_natural_loft","sections")=>Some(3),
                    ("brep_capped_loft","start"|"end"|"sides")=>Some(2),
                    ("brep_capped_loft","cap_surfaces")=>Some(1),
                    ("brep_capped_loft","cap_trims")=>Some(3),
                    ("loft_match_surface","sections"|"guides")=>Some(1),
                    ("loft_match_surface","start_reference"|"end_reference")=>Some(0),
                    _=>None,
                };
                node[&key]=if let Some(depth)=depth {self.loft_references(value,depth)?} else {raw(value)?};
            }
            return self.add(node);
        }
        if name=="loft_embedding_limits" {
            if input.is_some() {return Err(crate::error("Embedding limits cannot take a geometry receiver"));}
            let mut limits=json!({"exactWork":1000000,"trimPairs":10000,"trimCells":100000,
                "trimDomainCells":1000000,"spans":10000,"facePairs":10000,"faceCells":1000000,
                "faceDomainCells":1000000,"faceCellsPerPair":10000,"faceDomainCellsPerPair":10000});
            let mut seen=std::collections::BTreeSet::new();
            for (key,value) in args {
                let key=key.ok_or_else(||crate::error("Embedding limits must be named"))?;
                if limits.get(&key).is_none() || !seen.insert(key.clone()) {return Err(crate::error("Invalid or repeated embedding limit"));}
                limits[&key]=raw(value)?;
            }
            return Ok(V::Json(limits));
        }
        if name=="loft_parameter_map" {
            if input.is_some() { return Err(crate::error("Parameter maps cannot take a geometry receiver")); }
            let mut fields=json!({});
            for (key,value) in args {
                let key=key.ok_or_else(||crate::error("Parameter map options must be named"))?;
                let key=if key=="control_values" {"controlValues".to_string()} else {key};
                if !["domain","range","controlValues","weights","composition","pieces"].contains(&key.as_str()) || fields.get(&key).is_some() {
                    return Err(crate::error("Invalid or repeated parameter map option"));
                }
                fields[&key]=raw(value)?;
            }
            if fields.get("composition").is_some() || fields.get("pieces").is_some() {
                if fields.as_object().unwrap().len()!=1 { return Err(crate::error("Map composition/pieces cannot be mixed with scalar controls")); }
                if let Some(items)=fields.get("pieces") {
                    let mut pieces=Vec::new();
                    for item in items.as_array().ok_or_else(||crate::error("Map pieces must be an array"))? {
                        let values=item["pieces"].as_array().ok_or_else(||crate::error("Piece entries must be leaf parameter maps"))?;
                        pieces.extend(values.iter().cloned());
                    }
                    fields["pieces"]=json!(pieces);
                }
                return Ok(V::Json(fields));
            }
            if fields.as_object().unwrap().is_empty() {
                fields=json!({"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,1.],"weights":[1.,1.]});
            }
            for key in ["domain","range","controlValues","weights"] {
                if fields.get(key).is_none() { return Err(crate::error(format!("Parameter map requires {key}"))); }
            }
            return Ok(V::Json(json!({"pieces":[fields]})));
        }
        if name=="guided_loft_surface" || name=="auto_guided_loft_surface" || name=="control_tangent_loft_surface" || name=="natural_loft_surface" {
            let mut node=json!({"op":name});
            let at=args.iter().position(|(n,_)|n.as_deref()==Some("parameters")).ok_or_else(||crate::error("Loft requires named parameters"))?;
            node["parameters"]=raw(args.remove(at).1)?;
            if name=="guided_loft_surface" || name=="auto_guided_loft_surface" {
                let at=args.iter().position(|(n,_)|n.as_deref()==Some("guides")).ok_or_else(||crate::error("Guided loft requires named guides"))?;
                let V::Array(values)=args.remove(at).1 else{return Err(crate::error("Guides must be curve references"));};
                let mut ids=Vec::new();for value in values{ids.push(self.geometry(value)?);}
                node["guides"]=json!(ids);
                let field=if name=="auto_guided_loft_surface" {"budget"} else {"guide_parameters"};
                let at=args.iter().position(|(n,_)|n.as_deref()==Some(field)).ok_or_else(||crate::error(format!("Guided loft requires named {field}")))?;
                node[field]=raw(args.remove(at).1)?;
            }
            if name=="auto_guided_loft_surface" {
                if let Some(at)=args.iter().position(|(n,_)|n.as_deref()==Some("parameter_tolerance")) {
                    node["parameter_tolerance"]=raw(args.remove(at).1)?;
                }
            }
            for key in ["construction","error_budget","max_cells","max_map_evaluations","section_mappings"] {
                if let Some(at)=args.iter().position(|(n,_)|n.as_deref()==Some(key)) {
                    let value=raw(args.remove(at).1)?;
                    node[key]=if key=="construction" {
                        let mode=value["text"].as_str().ok_or_else(||crate::error("Loft construction must be a string"))?;
                        if !["cartesian","homogeneous"].contains(&mode){return Err(crate::error("Invalid loft construction"));}
                        json!(mode)
                    } else {value};
                }
            }
            for key in ["start_tangents","end_tangents"] {
                if let Some(at)=args.iter().position(|(n,_)|n.as_deref()==Some(key)) {node[key]=raw(args.remove(at).1)?;}
                else if name=="control_tangent_loft_surface" {return Err(crate::error(format!("Loft requires named {key}")));}
            }
            if node.get("start_tangents").is_some()!=node.get("end_tangents").is_some() {return Err(crate::error("Loft requires both tangent fields"));}
            if args.iter().any(|(n,_)|n.is_some()){return Err(crate::error("Loft sections must be positional"));}
            let mut inputs=Vec::new();if let Some(id)=input{inputs.push(id);}
            for(_,value)in args{inputs.push(self.geometry(value)?);}
            if !(2..=86).contains(&inputs.len()){return Err(crate::error("Loft requires 2..86 sections"));}
            node["inputs"]=json!(inputs);return self.add(node);
        }
        if name=="clamped_loft_surface" {
            let mut node=json!({"op":name});
            for key in ["parameters","start_tangent","end_tangent"] {
                let at=args.iter().position(|(n,_)|n.as_deref()==Some(key)).ok_or_else(||crate::error(format!("clamped_loft_surface requires named {key}")))?;
                node[key]=raw(args.remove(at).1)?;
            }
            if args.iter().any(|(n,_)|n.is_some()){return Err(crate::error("Clamped loft sections must be positional"));}
            let mut inputs=Vec::new();if let Some(id)=input{inputs.push(id);}
            for(_,value)in args{inputs.push(self.geometry(value)?);}
            if !(2..=11).contains(&inputs.len()){return Err(crate::error("Clamped loft requires 2..11 sections"));}
            node["inputs"]=json!(inputs);return self.add(node);
        }
        if name=="closed_loft_surface" {
            let at=args.iter().position(|(n,_)|n.as_deref()==Some("parameters")).ok_or_else(||crate::error("natural_loft_surface requires named parameters"))?;
            let parameters=raw(args.remove(at).1)?;
            if args.iter().any(|(n,_)|n.is_some()){return Err(crate::error("natural loft sections must be positional"));}
            let mut inputs=Vec::new();if let Some(id)=input{inputs.push(id);}
            for(_,value)in args{inputs.push(self.geometry(value)?);}
            if !(if name=="closed_loft_surface" {4} else {2}..=11).contains(&inputs.len()){return Err(crate::error("Cubic loft section count is outside its budget"));}
            return self.add(json!({"op":name,"inputs":inputs,"parameters":parameters}));
        }
        if ["brep_miter_sweep","brep_progressive_miter_sweep"].contains(&name) {
            if input.is_some(){return Err(crate::error("Miter sweep takes nested profile loops"));}
            let mut node=json!({"op":name});
            for key in ["points","normal"] {
                let at=args.iter().position(|(n,_)|n.as_deref()==Some(key)).ok_or_else(||crate::error(format!("Miter sweep requires {key}")))?;
                node[key]=raw(args.remove(at).1)?;
            }
            if name=="brep_progressive_miter_sweep" {
                if let Some(at)=args.iter().position(|(n,_)|n.as_deref()==Some("orientation_guide")) {
                    node["orientation_guide"]=json!(self.geometry(args.remove(at).1)?);
                }
                for key in ["axis_scale","center_law","frame_axis","frame_normal"] {
                    if let Some(at)=args.iter().position(|(n,_)|n.as_deref()==Some(key)) {
                        let V::Record(fields,_,_,_)=args.remove(at).1 else {return Err(crate::error(format!("{key} must be a vector-law record")));};
                        let mut law=J::Object(value_codec::Map::new());
                        for (component,value) in fields {law[&component]=raw(value)?;}
                        node[key]=law;
                    }
                }
                for key in ["scale","twist","max_deviation"] {
                    let at=args.iter().position(|(n,_)|n.as_deref()==Some(key)).ok_or_else(||crate::error(format!("Miter sweep requires {key}")))?;
                    let value=args.remove(at).1;
                    node[key]=if key=="scale" || key=="twist" {
                        let V::Record(fields,_,_,_)=value else {return Err(crate::error("Miter law must be a record"));};
                        let mut law=J::Object(value_codec::Map::new());
                        for (component,value) in fields {law[&component]=raw(value)?;} law
                    } else {raw(value)?};
                }
                if let Some(at)=args.iter().position(|(n,_)|n.as_deref()==Some("cap_correction_authored_frame")) {
                    let value=raw(args.remove(at).1)?;
                    node["cap_correction_authored_frame"]=match value {
                        J::Bool(b)=>J::Bool(b),
                        J::Number(n) if n.as_f64()==Some(0.) || n.as_f64()==Some(1.) => J::Bool(n.as_f64()==Some(1.)),
                        _=>return Err(crate::error("cap_correction_authored_frame expects true or false")),
                    };
                }
                for key in ["initial_steps","max_steps","retained_wall_max_injectivity_cells","circle_correction_tolerance","circle_correction_quantum","circle_correction_max_work"] {if let Some(at)=args.iter().position(|(n,_)|n.as_deref()==Some(key)){node[key]=raw(args.remove(at).1)?;}}
            }
            {
                for key in ["cap_correction_tolerance","cap_correction_quantum","cap_correction_max_work"] {
                    if let Some(at)=args.iter().position(|(n,_)|n.as_deref()==Some(key)) {node[key]=raw(args.remove(at).1)?;}
                }
            }
            if name=="brep_miter_sweep" {
                for key in ["max_deviation","circle_correction_tolerance","circle_correction_quantum","circle_correction_max_work"] {
                    if let Some(at)=args.iter().position(|(n,_)|n.as_deref()==Some(key)){node[key]=raw(args.remove(at).1)?;}
                }
                if node.get("max_deviation").is_none() && ["circle_correction_tolerance","circle_correction_quantum","circle_correction_max_work"].iter().any(|key|node.get(*key).is_some()) {
                    return Err(crate::error("Direct miter circle correction requires max_deviation"));
                }
            }
            if let Some(at)=args.iter().position(|(n,_)|n.as_deref()==Some("miter_limit")) {node["miter_limit"]=raw(args.remove(at).1)?;}
            if let Some(at)=args.iter().position(|(n,_)|n.as_deref()==Some("closed")) {
                node["closed"]=match raw(args.remove(at).1)? {
                    J::Bool(b)=>J::Bool(b),
                    J::Number(n)=>J::Bool(n.as_f64().is_some_and(|x|x!=0.)),
                    _=>return Err(crate::error("closed expects true or false")),
                };
            }
            if args.len()!=1 || args[0].0.is_some(){return Err(crate::error("Miter sweep needs one positional nested loop list"));}
            let V::Array(loops)=args.remove(0).1 else {return Err(crate::error("Miter sweep requires nested loops"));};
            if loops.is_empty() || loops.len()>16 {return Err(crate::error("Miter sweep needs 1..16 loops"));}
            let mut count=0;
            node["loops"]=J::Array(loops.into_iter().map(|wire| {
                let V::Array(curves)=wire else {return Err(crate::error("Miter sweep requires nested curves"));};
                count+=curves.len();if curves.is_empty() || count>64 {return Err(crate::error("Miter sweep needs nonempty loops and at most64 curves"));}
                curves.into_iter().map(|curve|self.geometry(curve).map(J::String)).collect::<crate::Result<Vec<_>>>().map(J::Array)
            }).collect::<crate::Result<Vec<_>>>()?);
            return self.add(node);
        }
        if name=="brep_progressive_sweep" || name=="progressive_sweep" || name=="profile_sweep" || name=="framed_sweep" || name=="scaled_sweep" || name=="two_guide_sweep" || name=="twist_sweep" {
            let mut node=json!({"op":name});
            if name=="brep_progressive_sweep" {
                for key in ["cap_correction_tolerance","cap_correction_quantum","cap_correction_max_work","circle_correction_tolerance","circle_correction_quantum","circle_correction_max_work"] {
                    if let Some(at)=args.iter().position(|(n,_)|n.as_deref()==Some(key)){node[key]=raw(args.remove(at).1)?;}
                }
            }
            if ["progressive_sweep","brep_progressive_sweep"].contains(&name) {
                if let Some(at)=args.iter().position(|(n,_)|n.as_deref()==Some("orientation_guide")) {
                    node["orientation_guide"]=json!(self.geometry(args.remove(at).1)?);
                }
            }
            let keys: &[&str]=if ["progressive_sweep","brep_progressive_sweep"].contains(&name) {&["scale","twist","normal","max_deviation"]} else if name=="profile_sweep" {&["scale","normal","sections","max_deviation"]} else if name=="twist_sweep" {&["origin","axis","start_degrees","sweep_degrees"]} else if name=="two_guide_sweep" {&["width","axis_y","axis_z"]} else if name=="scaled_sweep" {&["origin","scale"]} else {&["normal","sections","max_deviation"]};
            for &key in keys {
                let at=args.iter().position(|(n,_)|n.as_deref()==Some(key)).ok_or_else(||crate::error(format!("{name} requires named {key}")))?;
                let value=args.remove(at).1;
                node[key]=if (["scaled_sweep","profile_sweep","progressive_sweep","brep_progressive_sweep"].contains(&name) && key=="scale") || (["progressive_sweep","brep_progressive_sweep"].contains(&name) && key=="twist") {
                    let V::Record(fields,_,_,_)=value else {return Err(crate::error(format!("{name} {key} must be a record")));};
                    let mut law=J::Object(value_codec::Map::new());
                    for (component,value) in fields {law[&component]=raw(value)?;}
                    law
                } else {raw(value)?};
            }
            if ["progressive_sweep","brep_progressive_sweep"].contains(&name) {
                for key in ["axis_scale","center_law","frame_axis","frame_normal"] {
                    if let Some(at)=args.iter().position(|(n,_)|n.as_deref()==Some(key)) {
                        let V::Record(fields,_,_,_)=args.remove(at).1 else {return Err(crate::error(format!("{key} must be a vector-law record")));};
                        let mut law=J::Object(value_codec::Map::new());
                        for (component,value) in fields {law[&component]=raw(value)?;}
                        node[key]=law;
                    }
                }
                for key in ["orientation","spacing","initial_sections","max_sections","length_tolerance","length_max_cells","contact_parameter","contact_profile","rmf_transport_steps","error_max_cells","error_max_products"] {
                    if let Some(at)=args.iter().position(|(n,_)|n.as_deref()==Some(key)) {
                        let value=raw(args.remove(at).1)?;
                        node[key]=if ["orientation","spacing"].contains(&key) {
                            J::String(value.as_str().or_else(||value.get("text").and_then(J::as_str)).ok_or_else(||crate::error(format!("{key} must be a text choice")))?.to_owned())
                        } else {value};
                    }
                }
            }
            if args.iter().any(|(n,_)|n.is_some()){return Err(crate::error(format!("{name} profile/path are positional; unexpected named argument")));}
            if name=="brep_progressive_sweep" {
                if input.is_some() || args.len()!=2 {return Err(crate::error("brep_progressive_sweep requires nested loops followed by a path"));}
                let V::Array(loops)=args.remove(0).1 else {return Err(crate::error("Sweep body requires nested curve lists"));};
                if loops.is_empty() || loops.len()>16 {return Err(crate::error("Sweep body needs 1..16 loops"));}
                let mut count=0;
                node["loops"]=J::Array(loops.into_iter().map(|wire| {
                    let V::Array(curves)=wire else {return Err(crate::error("Sweep body requires nested curve lists"));};
                    count+=curves.len();if curves.is_empty() || count>64 {return Err(crate::error("Sweep body needs nonempty loops and at most64 curves"));}
                    curves.into_iter().map(|curve|self.geometry(curve).map(J::String)).collect::<crate::Result<Vec<_>>>().map(J::Array)
                }).collect::<crate::Result<Vec<_>>>()?);
                node["inputs"]=json!([self.geometry(args.remove(0).1)?]);
                return self.add(node);
            }
            let mut inputs=Vec::new();if let Some(id)=input{inputs.push(id);}
            for(_,value)in args{inputs.push(self.geometry(value)?);}
            if if name=="progressive_sweep" {!(2..=65).contains(&inputs.len())} else {inputs.len()!=if name=="two_guide_sweep" {3} else {2}} {return Err(crate::error(format!("{name} requires profile curves followed by its guide curve")));}
            node["inputs"]=json!(inputs);return self.add(node);
        }
        if [
            "surface_sweep",
            "surface_loft",
            "ruled_surface",
            "triangular_patch",
            "coons_patch",
            "curve_compose",
            "sdf_union",
            "sdf_intersection",
            "sdf_difference",
            "sdf_smooth_union",
            "mesh_union",
            "brep_union",
            "brep_subtract",
            "brep_intersection",
            "brep_xor",
            "mesh_intersection",
            "mesh_subtract",
            "union",
            "intersection",
            "subtract",
            "hull",
        ]
        .contains(&name)
        {
            let smooth = name == "sdf_smooth_union";
            let radius = if smooth {
                args.iter()
                    .position(|(n, _)| n.as_deref() == Some("radius"))
                    .map(|i| args.remove(i).1)
            } else {
                None
            };
            if args.iter().any(|(n, _)| n.is_some()) {
                return Err(crate::error(if name.starts_with("sdf_") {
                    "SDF operands are positional; smooth radius is named"
                } else {
                    "Boolean arguments are positional"
                }));
            }
            let mut inputs = Vec::new();
            if let Some(i) = input {
                inputs.push(i)
            }
            for (_, v) in args {
                inputs.push(self.geometry(v)?)
            }
            if name.starts_with("sdf_") && (inputs.len() != 2 || smooth && radius.is_none()) {
                return Err(crate::error(
                    "SDF operation requires two fields; smooth union also requires radius",
                ));
            }
            if name.starts_with("brep_") {
                if inputs.len() != 2 {
                    return Err(crate::error("B-rep Boolean requires exactly two bodies"));
                }
                return self.add(json!({"op":"brep_boolean","inputs":inputs,"operation":match name {"brep_union"=>"union","brep_intersection"=>"intersection","brep_xor"=>"xor",_=>"difference"}}));
            }
            if name.starts_with("mesh_") {
                if inputs.len() != 2 {
                    return Err(crate::error(
                        "Mesh Boolean operation requires exactly two meshes",
                    ));
                }
                return self.add(json!({"op":"mesh_boolean","inputs":inputs,"operation":match name{"mesh_union"=>"union","mesh_intersection"=>"intersection",_=>"difference"}}));
            }
            if inputs.is_empty() && ["union", "intersection", "subtract", "hull"].contains(&name) {
                return Err(crate::error("Boolean operation needs geometry"));
            }
            let mut node = if name == "subtract" {
                json!({"op":"difference","base":&inputs[0],"subtract":&inputs[1..]})
            } else {
                json!({"op":name,"inputs":inputs})
            };
            if let Some(v) = radius {
                node["radius"] = raw(v)?
            }
            return self.add(node);
        }
        if ["box", "translate"].contains(&name)
            && args.len() == 3
            && args.iter().all(|(name, _)| name.is_none())
        {
            let values = args
                .into_iter()
                .map(|(_, value)| scalar(value).map(V::Json))
                .collect::<R<Vec<_>>>()?;
            args = vec![(
                Some(if name == "box" { "size" } else { "vector" }.into()),
                V::Array(values),
            )];
        }
        if ["translate", "rotate", "scale"].contains(&name)
            && args
                .iter()
                .any(|(n, _)| matches!(n.as_deref(), Some("x" | "y" | "z")))
        {
            let mut names = Set::new();
            if args.iter().any(|(n, _)| {
                !matches!(n.as_deref(), Some("x" | "y" | "z")) || !names.insert(n.clone())
            }) {
                return Err(crate::error("Use unique x/y/z arguments or one vector"));
            }
            let vec = ["x", "y", "z"]
                .iter()
                .map(|axis| {
                    args.iter()
                        .find(|(n, _)| n.as_deref() == Some(axis))
                        .map_or(
                            V::Json(json!(if name == "scale" { 1 } else { 0 })),
                            |(_, v)| v.clone(),
                        )
                })
                .collect();
            args = vec![(Some("vector".into()), V::Array(vec))]
        }
        if name == "offset" && args.iter().any(|(n, _)| n.as_deref() == Some("delta")) {
            if input.is_none() || args.len() != 1 || args[0].0.as_deref() != Some("delta") {
                return Err(crate::error(
                    "offset(delta: distance) requires one argument and piped geometry",
                ));
            }
            return self.add(json!({"op":"offset","input":input,"distance":raw(args.remove(0).1)?,"mode":"delta"}));
        }
        let sig =
            signature(name).ok_or_else(|| crate::error(format!("Unknown operation {name}")))?;
        let mut node = json!({"op":name});
        for (i, (n, v)) in args.into_iter().enumerate() {
            let key = n
                .as_deref()
                .or_else(|| sig.get(i).copied())
                .ok_or_else(|| crate::error("Unknown or duplicate argument"))?;
            if !sig.contains(&key) || node.get(key).is_some() {
                return Err(crate::error("Unknown or duplicate argument"));
            }
            node[key] = if name=="ribbon_surface" && key=="width_law" {J::String(self.geometry(v)?)} else if name == "variable_pipe_surface" && key == "radius_law" { J::String(self.geometry(v)?) } else if name == "brep_extrude_curves" && key == "loops" {
                let V::Array(loops) = v else {
                    return Err(crate::error("Curve extrusion requires nested curve lists"));
                };
                if loops.len() > 64 {
                    return Err(crate::error("Curve extrusion accepts at most 64 loops"));
                }
                let mut count = 0;
                J::Array(loops.into_iter().map(|wire| {
                    let V::Array(curves) = wire else {
                        return Err(crate::error("Curve extrusion requires nested curve lists"));
                    };
                    count += curves.len();
                    if curves.is_empty() || count > 254 {
                        return Err(crate::error("Curve extrusion requires nonempty loops and at most 254 curve references"));
                    }
                    curves.into_iter().map(|curve| {
                        if matches!(curve, V::Array(_)) {
                            return Err(crate::error("Each loop item must be one curve"));
                        }
                        self.geometry(curve).map(J::String)
                    }).collect::<R<Vec<_>>>().map(J::Array)
                }).collect::<R<_>>()?)
            } else if ["herringbone", "internal", "left_handed", "lower", "closed", "periodic"].contains(&key) {
                // Text has no boolean literal: true/false lower to 1/0, and the
                // canonical schema wants a JSON boolean for these flags.
                match raw(v)? {
                    J::Bool(b) => J::Bool(b),
                    J::Number(n) => J::Bool(n.as_f64().is_some_and(|x| x != 0.)),
                    _ => return Err(crate::error(format!("{key} expects true or false"))),
                }
            } else {
                raw(v)?
            }
        }
        if ["helix_curve","ribbon_surface", "variable_pipe_surface", "pipe_surface","screw_surface","clothoid_curve", "spherical_spiral_curve", "toroidal_spiral_curve", "torus_knot_curve", "helicoid_patches", "circle_rectangle_transition", "ellipse_transition_surface", "circle_transition_surface", "helicoid_surface", "catenoid_patches", "catenoid_surface", "catenary_curve", "archimedean_spiral_curve", "epicycloid_curve", "hypocycloid_curve", "trochoid_curve", "cycloid_curve", "lissajous_curve", "logarithmic_spiral_curve", "involute_curve", "elliptic_helix_curve","conical_helix_curve","variable_pitch_helix_curve"].contains(&name) {
            for key in sig {
                if node.get(*key).is_none(){return Err(crate::error(format!("{name} requires {key}")));}
            }
        }
        let modifier = [
            "polygon_extrude",
            "polygon_sweep",
            "mesh_to_nurbs_brep",
            "mesh_to_sdf",
            "mesh_to_subdivision",
            "subdivision_tessellate",
            "mesh_to_nurbs",
            "mesh_fit_nurbs",
            "nurbs_patches_tessellate",
            "sdf_tessellate",
            "sdf_offset",
            "sdf_translate",
            "brep_tessellate",
            "brep_smooth_miter_stations",
            "brep_smooth_sweep_stations",
            "brep_extrude",
            "brep_revolve",
            "brep_chamfer",
            "brep_fillet",
            "ribbon_surface", "variable_pipe_surface", "pipe_surface", "screw_surface", "surface_extrude_patches", "surface_extrude",
            "surface_revolve",
            "tessellate",
            "thicken",
            "transform",
            "extrude",
            "revolve",
            "translate",
            "rotate",
            "scale",
            "offset",
            "mirror",
        ]
        .contains(&name);
        if modifier && input.is_none() {
            return Err(crate::error(format!("{name} requires piped geometry")));
        }
        if !modifier && input.is_some() {
            return Err(crate::error(format!(
                "{name} does not accept piped geometry"
            )));
        }
        if let Some(i) = input {
            let collection = self.collections.contains(&i);
            if collection && !["translate", "rotate", "scale", "mirror"].contains(&name) {
                return Err(crate::error(
                    "Transform each generated part inside the generator, or explicitly union the collection first",
                ));
            }
            node["input"] = json!(i);
            let result = self.add(node)?;
            if collection {
                let id = self.geometry(result.clone())?;
                self.collections.insert(id);
            }
            return Ok(result);
        }
        self.add(node)
    }
}
