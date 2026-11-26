use super::*;
use crate::curve::Curve;
use crate::involute::{CircularProfilePattern, CircularSectorPattern, LosslessSectorPattern};

pub(super) const OPS: &[&str] = &["edit_profile_sector","edit_closed_sector","edit_tangent_sector","repeat_profile_segment","compress_rotated_segments",
    "compress_profile_sectors","discover_profile_sectors","restore_lossless_sector","restore_bidirectional_sector"];

pub(super) fn apply(v:&Value, op:&str) -> Result<Value> {
    let curves: Vec<Curve> = field(v,"curves")?;
    check((1..=256).contains(&curves.len()), "Expected one to 256 ordered profile segments")?;
    let index:usize=field(v,"sector_index")?;
    let n=curves.len();
    if matches!(op,"edit_profile_sector"|"edit_closed_sector"|"edit_tangent_sector") {
        let width:usize=field(v,"pieces_per_sector")?;
        check(width>0 && width<n,"Expected source sectors followed by one replacement sector")?;
        let (source,replacement)=curves.split_at(n-width);
        let mut packed=LosslessSectorPattern::encode_bidirectional(source,width)?;
        match op {
            "edit_closed_sector"=>packed.replace_sector_with_closed_joins(index,replacement)?,
            "edit_tangent_sector"=>packed.replace_sector_preserving_g1(index,replacement)?,
            _=>packed.replace_sector(index,replacement)?,
        }
        let read:usize=field(v,"read_sector")?;
        let piece:usize=field(v,"piece")?;
        let sector=packed.sector(read)?;
        let curve=sector.get(piece).ok_or_else(||input("Invalid sector piece"))?;
        return Ok(json!({"curve":curve,"editedSector":index,"readSector":read,"sectorCount":packed.sector_count(),
            "piecesPerSector":width,"controlPoints":curve.control_points,"weights":curve.weights}));
    }
    if op=="repeat_profile_segment" {
        check(n==1,"Rotation requires one prototype segment")?;
        let count=field(v,"copies")?;
        let (curve,error)=CircularProfilePattern::new(curves.into_iter().next().ok_or_else(||input("Missing prototype"))?,count)?.instance(index)?;
        return Ok(json!({"curve":curve,"sectorCount":count,"rotationErrorMm":error}));
    }
    if op=="compress_rotated_segments" {
        let (pattern,error)=CircularProfilePattern::compress(&curves,field(v,"tolerance_mm")?)?
            .ok_or_else(||input("Rotational copies exceed tolerance or have incompatible layouts"))?;
        let (curve,rotation)=pattern.instance(index)?;
        return Ok(json!({"curve":curve,"sectorCount":n,"sourceErrorMm":error,"rotationErrorMm":rotation}));
    }
    let piece:usize=field(v,"piece")?;
    if matches!(op,"compress_profile_sectors"|"discover_profile_sectors") {
        let tolerance=field(v,"tolerance_mm")?;
        let pattern=if op=="discover_profile_sectors" {CircularSectorPattern::discover(&curves,tolerance)?}
            else {CircularSectorPattern::compress(&curves,field(v,"pieces_per_sector")?,tolerance)?};
        let (pattern,error)=pattern.ok_or_else(||input("Profile has no compatible rotational sectors within tolerance"))?;
        let sector=pattern.sector(index)?;
        let curve=sector.get(piece).ok_or_else(||input("Invalid sector piece"))?;
        return Ok(json!({"curve":curve,"sectorCount":pattern.sector_count(),"piecesPerSector":sector.len(),"sourceErrorMm":error}));
    }
    let width=field(v,"pieces_per_sector")?;
    let packed=if op=="restore_lossless_sector" {LosslessSectorPattern::encode(&curves,width)?}
        else {LosslessSectorPattern::encode_bidirectional(&curves,width)?};
    let sector=packed.sector(index)?;
    let curve=sector.get(piece).ok_or_else(||input("Invalid sector piece"))?;
    Ok(json!({"curve":curve,"sectorCount":packed.sector_count(),"piecesPerSector":width,
        "residualBytes":packed.residual_bytes(),"controlPoints":curve.control_points,"weights":curve.weights}))
}
