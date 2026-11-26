use crate::*;
use nurbs_core::{curve::Curve, involute::LosslessSectorPattern};

pub(super) fn profile(v:&Value)->Result<Value> {
    let mut curves:Vec<Curve>=field(v,"curves")?;
    let width:usize=field(v,"pieces_per_sector")?;
    if curves.len()>256 || width==0 || width>=curves.len() {return Err(input("Expected source sectors and one complete replacement"));}
    let mut companions:Vec<Vec<Curve>>=field(v,"loops")?;
    let spatial=curves[0].control_points.first().is_some_and(|p|p.len()==3);
    for curve in curves.iter_mut().chain(companions.iter_mut().flatten()) {
        curve.validate()?;
        for pole in &mut curve.control_points {
            match pole.as_slice() {
                [_,_]=>{},
                [_,_,z] if *z==0.=>pole.truncate(2),
                _=>return Err(input("Profile edits require XY-plane curves")),
            }
        }
    }
    let (source,replacement)=curves.split_at(curves.len()-width);
    let index=field(v,"sector_index")?;
    let mut packed=LosslessSectorPattern::encode_bidirectional(source,width)?;
    let verified=brep_core::profile_certificate::replace_certified_profile_sector(
        &mut packed,index,replacement,&companions,field(v,"tolerance_mm")?)?;
    let sector=packed.sector(index)?;
    let piece:usize=field(v,"piece")?;
    let mut curve=sector.get(piece).ok_or_else(||input("Invalid profile piece"))?.clone();
    if spatial {for pole in &mut curve.control_points {pole.push(0.);}}
    Ok(json!({"curve":curve,"nestingDepth":verified.nesting_depth(),"spans":verified.spans(),"pieces":verified.pieces()}))
}

pub(super) fn gear(v:&Value,spec:&brep_core::GearSpec)->Result<Value> {
    use brep_core::gear::{profile_with_report,replace_verified_gear_sector,gear_from_verified_profile};
    let (loops,_)=profile_with_report(spec)?;
    let outline_index=usize::from(spec.internal);
    let outline=loops.get(outline_index).ok_or_else(||input("Missing tooth outline"))?;
    let companion=loops.get(1-outline_index).map(Vec::as_slice);
    let width=outline.len()/spec.teeth;
    let mut packed=LosslessSectorPattern::encode_bidirectional(outline,width)?;
    let index=field(v,"sector_index")?;
    let mut sector=packed.sector(index)?;
    let flank:usize=field(v,"flank_index")?;
    let pole:usize=field(v,"pole_index")?;
    let curve=sector.iter_mut().filter(|curve|curve.degree>2).nth(flank).ok_or_else(||input("Invalid flank index"))?;
    let xy=curve.control_points.get_mut(pole).ok_or_else(||input("Invalid flank control point"))?;
    let before=xy.clone();
    xy[0]+=field::<f64>(v,"offset_x_mm")?;
    xy[1]+=field::<f64>(v,"offset_y_mm")?;
    let after=xy.clone();
    let verified=replace_verified_gear_sector(&mut packed,index,&sector,spec.internal,companion,field(v,"tolerance_mm")?,Default::default())?;
    let model=gear_from_verified_profile(&verified,spec)?;
    let step=if v["op"].as_str()==Some("edited_gear_step") {Some(cad_step::export_step_v9(&model)?.0)} else {None};
    Ok(json!({"model":model,"step":step,"controlBefore":before,"controlAfter":after,
        "winding":verified.winding(),"radialRanges":verified.radial_ranges(),"teeth":spec.teeth}))
}
