use crate::*;
use nurbs_core::involute::{self as gear,SpurMesh};

pub(super) fn measure(v:&Value)->Result<Value> {
    let op=v["op"].as_str().unwrap_or("");
    let first=&v["gear"];
    let module=field(first,"module")?;
    let teeth=field(first,"teeth")?;
    let number=|spec:&Value,key:&str,default:f64|spec.get(key).and_then(Value::as_f64).unwrap_or(default);
    let check_spur=|spec:&Value|->Result<()>{
        if number(spec,"helix",0.)!=0. || spec["herringbone"].as_bool()==Some(true) {
            return Err(input("Expected a straight spur gear"));
        }
        Ok(())
    };
    check_spur(first)?;
    if first["internal"].as_bool()==Some(true) {return Err(input("The first gear must be external"));}
    let pressure=number(first,"pressure",20.);
    let clearance=number(first,"clearance",0.25);
    let backlash=number(first,"backlash",0.);
    if op=="spur_tooth_chord" {
        let d=gear::spur_tooth_chord_bounds(module,teeth,pressure,clearance,backlash,field(v,"measuring_radius_mm")?)?;
        return Ok(json!({"chordMm":d.chord_length,"depthMm":d.depth_from_tip}));
    }
    if op=="spur_tooth_thinning" {
        return Ok(json!({"thinningMm":gear::spur_tooth_thinning_bounds(module,teeth,pressure,clearance,field(v,"measuring_radius_mm")?,field(v,"measured_chord_mm")?)?}));
    }
    if op=="spinner_layout_counts" {
        return Ok(json!({"counts":gear::planetary_layout_counts(module,field(v,"sun_teeth")?,teeth,field(v,"minimum_gap_mm")?)?}));
    }
    let mate=&v["mate"];
    check_spur(mate)?;
    if field::<f64>(mate,"module")?!=module || number(mate,"pressure",20.)!=pressure {
        return Err(input("The two spur gears require matching modules and pressure angles"));
    }
    let pair=[teeth,field(mate,"teeth")?];
    let internal=mate["internal"].as_bool().unwrap_or(false);
    if (matches!(op,"internal_spur_ratio"|"internal_spur_backlash") && !internal)
        || (matches!(op,"external_spur_ratio"|"external_spur_backlash"|"planet_tip_separation") && internal) {
        return Err(input("Selected operation and mating gear type disagree"));
    }
    let mesh=if internal {SpurMesh::Internal} else {SpurMesh::External};
    let thin=[backlash,number(mate,"backlash",0.)];
    let centre=||field::<f64>(v,"centre_mm");
    Ok(match op {
        "external_spur_ratio"=>json!({"contactRatio":gear::external_spur_contact_ratio_bounds(module,pair,pressure)?}),
        "internal_spur_ratio"=>json!({"contactRatio":gear::internal_spur_contact_ratio_bounds(module,pair[0],pair[1],pressure)?}),
        "mounted_spur_ratio"=>json!({"contactRatio":gear::mounted_spur_contact_ratio_bounds(module,pair,pressure,mesh,centre()?)?}),
        "spur_tip_clearance"=>json!({"clearancesMm":gear::spur_tip_root_clearance_bounds(module,pair,[clearance,number(mate,"clearance",0.25)],mesh,centre()?)?}),
        "external_spur_backlash"=>json!({"backlashMm":gear::mounted_spur_backlash_bounds(module,pair,pressure,thin,centre()?)?}),
        "internal_spur_backlash"=>json!({"backlashMm":gear::mounted_internal_spur_backlash_bounds(module,pair,pressure,thin,centre()?)?}),
        "spur_backlash_range"=>json!({"backlashMm":gear::spur_backlash_over_mounting_range(module,pair,pressure,thin,mesh,field(v,"centre_range_mm")?)?}),
        "spur_overlap_range"=>json!({"contactRatio":gear::spur_contact_ratio_over_mounting_range(module,pair,pressure,mesh,field(v,"centre_range_mm")?)?}),
        "spur_mounting_window"=>json!({"centresMm":gear::mounting_window_for_contact_ratio(module,pair,pressure,mesh,field(v,"minimum_ratio")?)?}),
        "planet_tip_separation"=>json!({"gapMm":gear::planet_tip_gap_bounds(module,pair,field(v,"orbit_ranges_mm")?,field(v,"separation_degrees")?)?}),
        _=>return Err(input("Unknown spur gear inspection")),
    })
}
