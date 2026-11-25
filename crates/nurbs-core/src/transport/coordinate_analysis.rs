use super::*;
use crate::{bounds, conditioning, coordinate_frame, curve::Curve, surface::Surface};

enum Geometry { Curve(Curve), Surface(Surface) }
impl Geometry {
    fn decode(v:&Value)->Result<Self>{
        match (v.get("curve"),v.get("surface")) {
            (Some(_),None)=>Ok(Self::Curve(field(v,"curve")?)),
            (None,Some(_))=>Ok(Self::Surface(field(v,"surface")?)),
            _=>Err(input("Expected one curve or one surface")),
        }
    }
    fn bounds(&self)->Result<bounds::Bounds>{
        match self {
            Self::Curve(c)=>{c.validate()?; c.bounds()}
            Self::Surface(s)=>{s.validate()?; s.bounds()}
        }
    }
}

pub(super) fn apply(v:&Value,op:&str)->Result<Value>{
    if op=="geometry_bounds_union" {
        let items: Vec<Value>=field(v,"items")?;
        check((1..=2048).contains(&items.len()),"Bounds require 1..2048 geometries")?;
        let boxes=items.iter().map(|item|Geometry::decode(item)?.bounds()).collect::<Result<Vec<_>>>()?;
        return encode(bounds::union(&boxes)?);
    }
    let source=Geometry::decode(v)?;
    if op=="geometry_conditioning" {
        let r=match source {
            Geometry::Curve(c)=>conditioning::curve(&c)?,
            Geometry::Surface(s)=>conditioning::surface(&s)?,
        };
        return Ok(json!({"weightRatio":r.weight_ratio.is_finite().then_some(r.weight_ratio),
            "weightRatioOverflow":!r.weight_ratio.is_finite(),"minRelativeKnotSpan":r.min_relative_knot_span,
            "coordinateOffsetRatio":r.coordinate_offset_ratio.filter(|x|x.is_finite()),
            "coordinateOffsetRatioOverflow":r.coordinate_offset_ratio.is_some_and(|x|!x.is_finite()),"zeroExtent":r.zero_extent}));
    }
    let frame=coordinate_frame::Frame{origin:field(v,"origin")?,scale:field(v,"frame_scale")?};
    let world=op=="coordinate_frame_world";
    match source {
        Geometry::Curve(c)=>encode(coordinate_frame::curve(&c,&frame,world)?),
        Geometry::Surface(s)=>encode(coordinate_frame::surface(&s,&frame,world)?),
    }
}
