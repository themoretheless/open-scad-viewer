use super::*;

pub(super) fn dispatch(v: Value, op: String) -> Result<Value> {
if matches!(op.as_str(), "surface_auto_guided_loft" | "surface_auto_guided_loft_cartesian") {
    if op == "surface_auto_guided_loft" || op == "surface_auto_guided_loft_cartesian" {
        let curves = field::<Vec<curve::Curve>>(&v,"curves")?;
        let mappings = optional_field::<Vec<Option<Value>>>(&v,"section_mappings")?;
        let mapped = mappings.as_ref().map(|m| loft_reparameterization::prepare_with_budget(&curves,m,
            field(&v,"budget")?,optional_field(&v,"maxCells")?.unwrap_or(50_000),
            optional_field(&v,"maxMapEvaluations")?.unwrap_or(200_000))).transpose()?;
        let sections = mapped.as_ref().map_or(curves.as_slice(), |m| m.curves.as_slice());
        let parameters=field::<Vec<f64>>(&v,"parameters")?;
        let guides=field::<Vec<curve::Curve>>(&v,"guides")?;
        let budget=field::<f64>(&v,"budget")?;
        let parameter_tolerance=optional_field::<f64>(&v,"parameter_tolerance")?.unwrap_or(1e-8);
        let mut result=if op=="surface_auto_guided_loft_cartesian" {
            if let Some(controls)=control_tangents(&v)? {
                loft_alignment::interpolate_cartesian_with_control_tangents(sections,&parameters,&guides,&controls,
                    budget,parameter_tolerance,field(&v,"maxCells")?,field(&v,"maxMapEvaluations")?)?
            } else if let Some(targets)=optional_field::<[curve::Curve;2]>(&v,"boundary_tangents")? {
                loft_alignment::interpolate_cartesian_with_tangents(sections,&parameters,&guides,&targets,
                    budget,parameter_tolerance,field(&v,"maxCells")?,field(&v,"maxMapEvaluations")?)?
            } else {
                loft_alignment::interpolate_cartesian_with_parameter_tolerance(sections,&parameters,&guides,
                    budget,parameter_tolerance,field(&v,"maxCells")?,field(&v,"maxMapEvaluations")?)?
            }
        } else {
            loft_alignment::interpolate_with_parameter_tolerance(sections,&parameters,&guides,budget,parameter_tolerance)?
        };
        let original_section_certificates = if let Some(mappings) = mappings.as_ref() {
            let certificates = loft_reparameterization::certify_final_sections(&result.surface,
                &curves,&parameters,mappings,budget,
                optional_field(&v,"maxCells")?.unwrap_or(50_000),
                optional_field(&v,"maxMapEvaluations")?.unwrap_or(200_000))?;
            result.section_error_upper = certificates.iter().map(|c| c["errorUpper"].as_f64().unwrap()).collect();
            certificates
        } else {Vec::new()};
        let mut output = json!({"surface":result.surface,"guides":result.guides,"guide_parameters":result.guide_parameters,"guide_order":result.guide_order,"reversed":result.reversed,"section_error_upper":result.section_error_upper,"guide_error_upper":result.guide_error_upper});
        if op=="surface_auto_guided_loft_cartesian" {
            output["certificate"]=json!({"operation":"cartesian-auto-guided-loft","exact":false,
                "fittedToExactPromotion":false,"curves":result.curve_certificates,"tangents":result.tangent_certificates});
        }
        if !original_section_certificates.is_empty() {
            output["original_section_certificates"] = Value::Array(original_section_certificates);
        }
        if let Some(mapped) = mapped {
            output["sections"] = value_codec::to_value(mapped.curves).map_err(|e| input(e.to_string()))?;
            output["section_mapping_certificates"] = Value::Array(mapped.certificates);
        }
        return Ok(output);
    }
    }
if matches!(op.as_str(), "surface_guided_loft_cartesian") {
    if op == "surface_guided_loft_cartesian" {
        let originals=field::<Vec<curve::Curve>>(&v,"curves")?;
        let mappings=optional_field::<Vec<Option<Value>>>(&v,"section_mappings")?;
        let mapped=mappings.as_ref().map(|m|loft_reparameterization::prepare_with_budget(&originals,m,
            field(&v,"errorBudget")?,field(&v,"maxCells")?,field(&v,"maxMapEvaluations")?)).transpose()?;
        let sections=mapped.as_ref().map_or(originals.as_slice(),|m|m.curves.as_slice());
        let parameters=field::<Vec<f64>>(&v,"parameters")?;
        let guides=field::<Vec<curve::Curve>>(&v,"guides")?;
        let guide_parameters=field::<Vec<f64>>(&v,"guide_parameters")?;
        let tolerance=field(&v,"errorBudget")?;
        let max_cells=field(&v,"maxCells")?;
        let max_map_evaluations=field(&v,"maxMapEvaluations")?;
        let (surface,curves,tangents)=if let Some(controls)=control_tangents(&v)? {
            guided_loft::interpolate_cartesian_with_control_tangents(sections,&parameters,&guides,&guide_parameters,
                &controls,tolerance,max_cells,max_map_evaluations)?
        } else if let Some(targets)=optional_field::<[curve::Curve;2]>(&v,"boundary_tangents")? {
            guided_loft::interpolate_cartesian_with_tangents(sections,&parameters,&guides,&guide_parameters,
                &targets,tolerance,max_cells,max_map_evaluations)?
        } else {
            let (surface,curves)=guided_loft::interpolate_cartesian(sections,&parameters,&guides,&guide_parameters,
                tolerance,max_cells,max_map_evaluations)?;
            (surface,curves,Vec::new())
        };
        let original_sections=if let Some(mappings)=mappings.as_ref() {
            loft_reparameterization::certify_final_sections(&surface,&originals,&parameters,mappings,
                tolerance,max_cells,max_map_evaluations)?
        } else {Vec::new()};
        let mut output=json!({"surface":surface,"certificate":{"operation":"cartesian-guided-loft",
            "exact":false,"fittedToExactPromotion":false,"curves":curves,"tangents":tangents}});
        if let Some(mapped)=mapped {
            output["sections"]=encode(mapped.curves)?;
            output["section_mapping_certificates"]=Value::Array(mapped.certificates);
            output["original_section_certificates"]=Value::Array(original_sections);
        }
        return Ok(output);
    }
    }
if matches!(op.as_str(), "surface_natural_loft_checked") {
    if op == "surface_natural_loft_checked" {
        let curves=field::<Vec<curve::Curve>>(&v,"curves")?;
        let parameters=field::<Vec<f64>>(&v,"parameters")?;
        let mappings=field::<Vec<Option<Value>>>(&v,"section_mappings")?;
        let prepared=loft_reparameterization::prepare(&curves,&mappings)?;
        let surface=natural_loft::interpolate(&prepared.curves,&parameters)?;
        let certificates=loft_reparameterization::certify_final_sections(&surface,&curves,&parameters,&mappings,1e-6,50_000,200_000)?;
        return Ok(json!({"surface":surface,"sections":prepared.curves,
            "section_mapping_certificates":prepared.certificates,"original_section_certificates":certificates,
            "certificate":{"operation":"mapped-natural-loft","exact":false,"fittedToExactPromotion":false}}));
    }
    }
if matches!(op.as_str(), "surface_gordon_cartesian") {
    if op == "surface_gordon_cartesian" {
        let u=field::<Vec<curve::Curve>>(&v,"u_curves")?;
        let guides=field::<Vec<curve::Curve>>(&v,"v_curves")?;
        let pu=field::<Vec<f64>>(&v,"parameters_u")?;
        let pv=field::<Vec<f64>>(&v,"parameters_v")?;
        let tolerance=field::<f64>(&v,"errorBudget")?;
        let max_cells=field::<usize>(&v,"maxCells")?;
        let max_map_evaluations=field::<usize>(&v,"maxMapEvaluations")?;
        let (surface,curves,tangents)=if let Some(targets)=optional_field::<[curve::Curve;2]>(&v,"boundary_tangents")? {
            gordon::patch_cartesian_with_tangents(&u,&guides,&pu,&pv,&targets,tolerance,max_cells,max_map_evaluations)?
        } else {
            let (surface,curves)=gordon::patch_cartesian(&u,&guides,&pu,&pv,tolerance,max_cells,max_map_evaluations)?;
            (surface,curves,Vec::new())
        };
        return Ok(json!({"surface":surface,"certificate":{"operation":"cartesian-gordon",
            "exact":false,"fittedToExactPromotion":false,"curves":curves,"tangents":tangents}}));
    }
    }
if op == "surface_gordon" {return encode(gordon::patch(&field::<Vec<curve::Curve>>(&v,"u_curves")?,&field::<Vec<curve::Curve>>(&v,"v_curves")?,&field::<Vec<f64>>(&v,"parameters_u")?,&field::<Vec<f64>>(&v,"parameters_v")?)?);}
if op == "surface_closed_loft" { return encode(natural_loft::closed(&field::<Vec<curve::Curve>>(&v,"curves")?,&field::<Vec<f64>>(&v,"parameters")?)?); }
if op == "surface_loft_match_ends" {
        let mut references:[Option<surface::Surface>;2]=[None,None];
        let mut boundaries=[String::new(),String::new()];
        let mut orders=[1usize;2];let mut scales=[1.;2];let mut reversed=[false;2];
        for (i,key) in ["start","end"].iter().enumerate(){
            if let Some(value)=v.get(*key).filter(|x|!x.is_null()) {
                references[i]=Some(field(value,"reference")?);boundaries[i]=field(value,"boundary")?;
                orders[i]=field(value,"order")?;scales[i]=field(value,"scale")?;
                reversed[i]=optional_field(value,"reverse")?.unwrap_or(false);
            }
        }
        let ends=std::array::from_fn(|i|references[i].as_ref().map(|reference|loft_continuity::EndConstraint{reference,boundary:&boundaries[i],order:orders[i],scale:scales[i],reverse:reversed[i]}));
        let result=loft_continuity::match_ends(&field(&v,"surface")?,&field::<Vec<curve::Curve>>(&v,"curves")?,&field::<Vec<f64>>(&v,"parameters")?,&optional_field::<Vec<curve::Curve>>(&v,"guides")?.unwrap_or_default(),&optional_field::<Vec<f64>>(&v,"guide_parameters")?.unwrap_or_default(),ends,field(&v,"budget")?)?;
        return Ok(json!({"surface":result.surface,"seams":result.seams,"section_error_upper":result.section_error_upper,"guide_error_upper":result.guide_error_upper}));
    }
if op == "surface_guided_loft" {
        let start = optional_field::<Vec<[f64; 3]>>(&v, "start_tangents")?;
        let end = optional_field::<Vec<[f64; 3]>>(&v, "end_tangents")?;
        let tangents = match (start, end) {
            (None, None) => None,
            (Some(a), Some(b)) => Some([a,b]),
            _ => return Err(input("Guided loft requires both endpoint tangent fields")),
        };
        return encode(guided_loft::interpolate(
            &field::<Vec<curve::Curve>>(&v,"curves")?,
            &field::<Vec<f64>>(&v,"parameters")?,
            &field::<Vec<curve::Curve>>(&v,"guides")?,
            &field::<Vec<f64>>(&v,"guide_parameters")?, tangents,
        )?);
    }
if op == "surface_control_tangent_loft" {
        return encode(natural_loft::clamped_control_tangents(
            &field::<Vec<curve::Curve>>(&v, "curves")?,
            &field::<Vec<f64>>(&v, "parameters")?,
            &field::<Vec<[f64; 3]>>(&v, "start_tangents")?,
            &field::<Vec<[f64; 3]>>(&v, "end_tangents")?,
        )?);
    }
if op == "surface_clamped_loft" { return encode(natural_loft::clamped(&field::<Vec<curve::Curve>>(&v,"curves")?,&field::<Vec<f64>>(&v,"parameters")?,field(&v,"start_tangent")?,field(&v,"end_tangent")?)?); }
if op == "surface_natural_loft" {
        let curves = field::<Vec<curve::Curve>>(&v,"curves")?;
        let parameters = field::<Vec<f64>>(&v,"parameters")?;
        return encode(if let Some(mappings) = optional_field::<Vec<Option<Value>>>(&v,"section_mappings")? {
            loft_reparameterization::interpolate(&curves,&parameters,&mappings)?
        } else {natural_loft::interpolate(&curves,&parameters)?});
    }
if op == "loft_aligned" {
        return encode(surface::loft_aligned(&field::<Vec<curve::Curve>>(
            &v, "curves",
        )?)?);
    }
if op == "loft" {
        return encode(surface::loft(&field::<Vec<curve::Curve>>(&v, "curves")?)?);
    }
Err(input("Unknown NURBS operation"))
}
