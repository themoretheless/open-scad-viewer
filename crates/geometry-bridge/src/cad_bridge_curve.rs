//! Quintic Hermite bridge with exact endpoint position, tangent and curvature.
use super::{Result, Value, encode, field, input};
use nurbs_core::curve::Curve;
pub fn bridge(v: Value) -> Result<Value> {
    let a: Curve = field(&v,"a")?;
    let b: Curve = field(&v,"b")?;
    a.validate()?; b.validate()?;
    if a.control_points[0].len() != b.control_points[0].len() { return Err(input("Curve dimensions differ.")); }
    let end_a: String = field(&v,"endA")?;
    let end_b: String = field(&v,"endB")?;
    if !["start","end"].contains(&end_a.as_str()) || !["start","end"].contains(&end_b.as_str()) { return Err(input("Invalid bridge endpoint.")); }
    let tension: f64 = field(&v,"tension")?;
    if !tension.is_finite() || !(0.01..=10.).contains(&tension) { return Err(input("Bridge tension must be 0.01 to 10.")); }
    let ea = a.evaluate(a.domain()[usize::from(end_a=="end")])?;
    let eb = b.evaluate(b.domain()[usize::from(end_b=="end")])?;
    let da = ea.d1.ok_or_else(|| input("Source tangent unavailable."))?;
    let db = eb.d1.ok_or_else(|| input("Target tangent unavailable."))?;
    let dda = ea.d2.ok_or_else(|| input("Source curvature unavailable."))?;
    let ddb = eb.d2.ok_or_else(|| input("Target curvature unavailable."))?;
    let norm = |p: &[f64]| p.iter().map(|x|x*x).sum::<f64>().sqrt();
    let gap: Vec<f64> = ea.point.iter().zip(&eb.point).map(|(a,b)|b-a).collect();
    let length = norm(&gap)*tension;
    if length < 1e-9 || norm(&da)<1e-9 || norm(&db)<1e-9 { return Err(input("Bridge needs distinct endpoints and nonzero tangents.")); }
    let sa = length/norm(&da)*if end_a=="end" {1.} else {-1.};
    let sb = length/norm(&db)*if end_b=="start" {1.} else {-1.};
    let mut p = vec![ea.point.clone();6];
    p[5] = eb.point.clone();
    for k in 0..p[0].len() {
        p[1][k] = p[0][k]+da[k]*sa/5.;
        p[2][k] = dda[k]*sa*sa/20.+2.*p[1][k]-p[0][k];
        p[4][k] = p[5][k]-db[k]*sb/5.;
        p[3][k] = ddb[k]*sb*sb/20.+2.*p[4][k]-p[5][k];
    }
    let curve = Curve {degree:5,knots:[vec![0.;6],vec![1.;6]].concat(),control_points:p,weights:vec![1.;6],periodic:false};
    curve.validate()?;
    encode(curve)
}

#[cfg(test)]
mod tests {
    use super::*;
    use value_codec::json;
    #[test]
    fn rational_g2_endpoints() {
        let make=|x:f64| json!({"degree":2,"knots":[0,0,0,1,1,1],"controlPoints":[[x,0,0],[x+1.,2,0],[x+3.,1,0]],"weights":[1,2,1]});
        for end_a in ["start","end"] {for end_b in ["start","end"] {
            let v=json!({"a":make(0.),"b":make(10.),"endA":end_a,"endB":end_b,"tension":0.7});
            let curve: Curve=value_codec::from_value(bridge(v).unwrap()).unwrap();
            for (source,end,t,sign) in [(make(0.),end_a,0.,if end_a=="end" {1.} else {-1.}),(make(10.),end_b,1.,if end_b=="start" {1.} else {-1.})] {
                let source:Curve=value_codec::from_value(source).unwrap();
                let e=source.evaluate(if end=="end" {1.} else {0.}).unwrap();
                let actual=curve.evaluate(t).unwrap();
                let d=e.d1.unwrap();let dd=e.d2.unwrap();let ad=actual.d1.unwrap();let add=actual.d2.unwrap();
                let norm=|v:&[f64]|v.iter().map(|x|x*x).sum::<f64>().sqrt();
                let scale=norm(&ad)/norm(&d);
                for i in 0..3 {
                    assert!((actual.point[i]-e.point[i]).abs()<1e-9);
                    assert!((ad[i]-d[i]*scale*sign).abs()<1e-8);
                    assert!((add[i]-dd[i]*scale*scale).abs()<1e-7);
                }
            }
        }}
    }
}
