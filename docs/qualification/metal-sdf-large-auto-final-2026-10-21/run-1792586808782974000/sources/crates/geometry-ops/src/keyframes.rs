//! Bounded keyframe sampling shared by assembly and orbit-camera previews.
use crate::{Result, fail};
pub fn sample(times: &[f64], values: &[Vec<f64>], time: f64, angles: &[usize]) -> Result<Vec<f64>> {
    if times.is_empty() || times.len() > 64 || times.len() != values.len() || !time.is_finite() {
        return Err(fail("Invalid keyframe timeline"));
    }
    let width = values[0].len();
    if width == 0 || width > 322 || values.iter().any(|v| v.len()!=width || v.iter().any(|x| !x.is_finite() || x.abs()>1e6))
        || times.iter().any(|x| !x.is_finite() || *x<0. || *x>120.)
        || times.windows(2).any(|p| p[0]>=p[1]) || angles.iter().any(|i| *i>=width) {
        return Err(fail("Invalid keyframe values"));
    }
    if time <= times[0] { return Ok(values[0].clone()); }
    if time >= times[times.len()-1] { return Ok(values[values.len()-1].clone()); }
    let upper=times.partition_point(|t| *t<time);
    let t=(time-times[upper-1])/(times[upper]-times[upper-1]);
    Ok(values[upper-1].iter().zip(&values[upper]).enumerate().map(|(i,(a,b))| {
        let delta=if angles.contains(&i) {(b-a+std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)-std::f64::consts::PI} else {b-a};
        a+delta*t
    }).collect())
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn samples_translation_and_shortest_rotation() {
        let v=sample(&[0.,2.],&[vec![0.,350f64.to_radians()],vec![10.,10f64.to_radians()]],1.,&[1]).unwrap();
        assert!((v[0]-5.).abs()<1e-12);assert!((v[1]-std::f64::consts::TAU).abs()<1e-12);
        assert_eq!(sample(&[0.,2.],&[vec![1.],vec![2.]],3.,&[]).unwrap(),vec![2.]);
    }
    #[test] fn refuses_nonmonotone_and_nonfinite_tracks() {
        assert!(sample(&[1.,1.],&[vec![0.],vec![1.]],0.,&[]).is_err());
        assert!(sample(&[0.],&[vec![f64::NAN]],0.,&[]).is_err());
    }
}
