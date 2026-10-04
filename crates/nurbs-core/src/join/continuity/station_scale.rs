//! Numerical proposals for station-strip reparameterization. A proposal never
//! certifies a seam: exact strip identities and regularity remain mandatory.
use crate::{Result, check, surface::Surface};

fn derivative(s: &Surface, boundary: &str) -> Result<Option<[f64; 3]>> {
    s.validate()?;
    check(matches!(boundary, "vMin" | "vMax"), "Invalid station boundary")?;
    check(s.control_points.iter().flatten().all(|p| p.len() == 3), "Station scale requires XYZ surfaces")?;
    let n = s.control_points[0].len();
    let v0 = s.knots_v[s.degree_v];
    let v1 = s.knots_v[n];
    if !s.periodic_v && n == s.degree_v + 1 && s.knots_v.len() == 2*n
        && s.knots_v[..n].iter().all(|&v| v == v0)
        && s.knots_v[n..].iter().all(|&v| v == v1)
        && s.weights.iter().all(|row| row[0] > 0. && row.iter().all(|&w| w == row[0])) {
        let i = if boundary == "vMin" {0} else {n-2};
        return Ok(Some(std::array::from_fn(|k|
            (s.control_points[0][i+1][k]-s.control_points[0][i][k])*s.degree_v as f64)));
    }
    let u = (s.knots_u[s.degree_u]+s.knots_u[s.control_points.len()])/2.;
    let v = if boundary == "vMin" {v0} else {v1};
    Ok(s.evaluate(u,v)?.first_derivatives().map(|(_,dv)|dv.map(|x|x*(v1-v0))))
}

pub fn propose_station_normal_scale(a: &Surface, b: &Surface, ab: &str, bb: &str) -> Result<f64> {
    let (Some(da), Some(db)) = (derivative(a,ab)?, derivative(b,bb)?) else {return Ok(1.);};
    let mut k = 0;
    for i in 1..3 {if da[i].abs() > da[k].abs() {k=i;}}
    let ratio = (db[k]/da[k]).abs();
    Ok(if ratio.is_finite() && ratio > 0. {ratio} else {1.})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn strip(degree: usize, height: f64) -> Surface {
        Surface {degree_u:1, degree_v:degree, knots_u:vec![0.,0.,1.,1.],
            knots_v:[vec![7.;degree+1],vec![19.;degree+1]].concat(),
            control_points:(0..2).map(|u|(0..=degree).map(|v|vec![u as f64,0.,height*v as f64]).collect()).collect(),
            weights:vec![vec![1.;degree+1];2],periodic_u:false,periodic_v:false}
    }
    #[test]
    fn normalized_bezier_scale_uses_endpoint_poles_and_retains_source() {
        let a=strip(5,0.125);let b=strip(5,0.25);
        let saved=a.control_points.clone();
        assert_eq!(propose_station_normal_scale(&a,&b,"vMax","vMin").unwrap(),2.);
        assert_eq!(propose_station_normal_scale(&a,&a,"vMax","vMin").unwrap(),1.);
        assert_eq!(a.control_points,saved);
        assert!(propose_station_normal_scale(&a,&b,"uMin","vMin").is_err());
        let flat=strip(1,0.);
        assert_eq!(propose_station_normal_scale(&flat,&flat,"vMax","vMin").unwrap(),1.);
    }
    #[test]
    fn varying_weights_use_the_native_rational_derivative() {
        let mut a=strip(1,0.125);let mut b=strip(1,0.25);
        a.weights=vec![vec![1.,2.];2];b.weights=a.weights.clone();
        assert_eq!(propose_station_normal_scale(&a,&b,"vMin","vMin").unwrap(),2.);
    }
    #[cfg(feature="codec")]
    #[test]
    fn proposal_is_available_through_the_json_boundary() {
        let result=crate::transport::dispatch(value_codec::json!({
            "op":"surface_station_normal_scale", "reference":strip(5,0.125), "edited":strip(5,0.25),
            "referenceBoundary":"vMax", "editedBoundary":"vMin"
        })).unwrap();
        assert_eq!(result.as_f64(),Some(2.));
    }
}
