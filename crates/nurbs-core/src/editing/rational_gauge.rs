//! Exact common binary gauge changes, without coordinate or parameter edits.
use crate::{Result,check,curve::Curve,surface::Surface};
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum Refusal { WeightRange{index:usize}, InvalidOutput }
#[derive(Clone,Debug)]
pub struct Report<T> {
    /// None on refusal; no partially changed geometry is emitted.
    pub geometry:Option<T>,
    /// The exact common factor is2^exponent, not a rounded arbitrary divisor.
    pub exponent:i16,
    pub refusal:Option<Refusal>,
}
fn exponent_valid(exponent:i16)->Result<()>{
    // Every valid input/output weight lies in[1e-12,1e12]. A larger
    // integer exponent cannot map even one such weight into this interval.
    check((-79..=79).contains(&exponent),"Binary gauge exponent must be in[-79,79]")
}
fn weight(value:f64,exponent:i16)->Option<f64>{
    // Validated input weights are positive NORMAL binary64 values. Within
    // the requested exponent range, the shifted exponent remains normal.
    // Retain all52 fraction bits exactly, with no floating multiplication.
    let bits=value.to_bits();let e=((bits>>52)&0x7ff) as i32+exponent as i32;
    let result=f64::from_bits((bits&((1u64<<52)-1))|((e as u64)<<52));
    (1e-12..=1e12).contains(&result).then_some(result)
}
fn normalizing_exponent(max:f64)->i16{1023-((max.to_bits()>>52)&0x7ff) as i16}
/// Multiply every weight by exactly2^exponent. Same nominal rational law,
/// controls, knots, degree, domain and periodic storage; evaluator rounding
/// and downstream source-bound certificates are not preserved by this claim.
pub fn curve(source:&Curve,exponent:i16)->Result<Report<Curve>>{
    source.validate()?;exponent_valid(exponent)?;
    let mut result=source.clone();
    for (index,(out,&input)) in result.weights.iter_mut().zip(&source.weights).enumerate(){
        let Some(value)=weight(input,exponent) else{return Ok(Report{geometry:None,exponent,refusal:Some(Refusal::WeightRange{index})})};*out=value;
    }
    if result.validate().is_err(){return Ok(Report{geometry:None,exponent,refusal:Some(Refusal::InvalidOutput)})}
    Ok(Report{geometry:Some(result),exponent,refusal:None})
}
/// Apply ONE global factor to the complete tensor net, never one per row.
/// Refusal emits no partially scaled surface. Source limits stay unchanged.
pub fn surface(source:&Surface,exponent:i16)->Result<Report<Surface>>{
    source.validate()?;exponent_valid(exponent)?;
    let mut result=source.clone();let mut index=0;
    for (out,&input) in result.weights.iter_mut().flatten().zip(source.weights.iter().flatten()){
        let Some(value)=weight(input,exponent) else{return Ok(Report{geometry:None,exponent,refusal:Some(Refusal::WeightRange{index})})};*out=value;index+=1;
    }
    if result.validate().is_err(){return Ok(Report{geometry:None,exponent,refusal:Some(Refusal::InvalidOutput)})}
    Ok(Report{geometry:Some(result),exponent,refusal:None})
}
/// Choose a common power of two putting the largest weight in[1,2).
/// Existing absolute bounds and conditioning are revalidated, never relaxed.
pub fn normalize_curve(source:&Curve)->Result<Report<Curve>>{
    source.validate()?;let max=source.weights.iter().copied().fold(0.,f64::max);
    curve(source,normalizing_exponent(max))
}
pub fn normalize_surface(source:&Surface)->Result<Report<Surface>>{
    source.validate()?;let max=source.weights.iter().flatten().copied().fold(0.,f64::max);
    surface(source,normalizing_exponent(max))
}
