//! Exact-rational affine cap proposal. No trace, regularity or Solid credit.
use crate::{LoftCap, Result, invalid};
use num_rational::BigRational as Q;
use num_traits::{ToPrimitive, Zero};
use nurbs_core::{interval_eval::Interval as I, surface::Surface};
type Pose = [[f64; 3]; 4];
type Matrix = [[Q; 3]; 3];
pub struct Report {
    pub cap: Option<LoftCap>,
    /// Exact inverse image of the proposed world cap, for a fresh filled map.
    pub reference: Option<Surface>,
    pub displacement_upper: Option<f64>,
    pub work: u64,
    pub reason: &'static str,
}
fn exact(q: &Q) -> Option<f64> {
    let f = q.to_f64()?;
    (f.is_finite() && Q::from_float(f).as_ref() == Some(q)).then_some(f)
}
fn linear(pose: Pose, p: &[f64], origin: [f64; 3]) -> [Q; 3] {
    std::array::from_fn(|k| {
        (0..3)
            .map(|j| {
                Q::from_float(pose[j][k]).unwrap()
                    * (Q::from_float(p[j]).unwrap() - Q::from_float(origin[j]).unwrap())
            })
            .sum()
    })
}
/// Exact complete affine sum, rather than a sequence of rounded products.
/// Every output coefficient must be exactly representable as binary64.
pub(crate) fn image(pose: Pose, p: &[f64], origin: [f64; 3]) -> Option<[f64; 3]> {
    if p.len() != 3
        || pose
            .iter()
            .flatten()
            .chain(p)
            .chain(&origin)
            .any(|v| !v.is_finite())
    {
        return None;
    }
    let q = linear(pose, p, origin);
    Some([
        exact(&(&q[0] + Q::from_float(pose[3][0])?))?,
        exact(&(&q[1] + Q::from_float(pose[3][1])?))?,
        exact(&(&q[2] + Q::from_float(pose[3][2])?))?,
    ])
}
pub(crate) fn inverse(pose: Pose) -> Option<Matrix> {
    if pose.iter().flatten().any(|v| !v.is_finite()) {
        return None;
    }
    let m: Matrix =
        std::array::from_fn(|r| std::array::from_fn(|c| Q::from_float(pose[c][r]).unwrap()));
    let cof: Matrix = std::array::from_fn(|r| {
        std::array::from_fn(|c| {
            let rr = (0..3).filter(|i| *i != r).collect::<Vec<_>>();
            let cc = (0..3).filter(|i| *i != c).collect::<Vec<_>>();
            let x = &m[rr[0]][cc[0]] * &m[rr[1]][cc[1]] - &m[rr[0]][cc[1]] * &m[rr[1]][cc[0]];
            if (r + c) % 2 == 0 { x } else { -x }
        })
    });
    let det: Q = (0..3).map(|c| &m[0][c] * &cof[0][c]).sum();
    if det.is_zero() {
        return None;
    }
    Some(std::array::from_fn(|r| {
        std::array::from_fn(|c| &cof[c][r] / &det)
    }))
}
fn step(out: &mut crate::rational_cap_translation::Report, max: u64) -> bool {
    if out.work == max {
        out.reason = "rational-cap-affine-work-limit";
        false
    } else {
        out.work += 1;
        true
    }
}
/// Common world-coordinate amplitude proposal, exact inverse reference and a
/// whole-carrier error bound. Material traces MUST be freshly proved afterward.
/// A singular matrix, unrepresentable reference or exhausted work returns no cap.
pub fn propose(
    cap: &LoftCap,
    pose: Pose,
    origin: [f64; 3],
    anchor: [f64; 3],
    tolerance: f64,
    max_work: u64,
) -> Result<Report> {
    cap.surface.validate()?;
    let s = &cap.surface;
    if max_work > 1000000
        || !tolerance.is_finite()
        || tolerance < 0.
        || pose
            .iter()
            .flatten()
            .chain(&origin)
            .chain(&anchor)
            .any(|v| !v.is_finite())
        || s.control_points.iter().flatten().any(|p| p.len() != 3)
        || s.control_points
            .len()
            .saturating_mul(s.control_points[0].len())
            > 4096
    {
        return Err(invalid("Invalid exact affine cap proposal"));
    }
    let mut ledger = crate::rational_cap_translation::Report {
        cap: None,
        displacement_upper: None,
        modified_axes: [false; 3],
        work: 0,
        reason: "rational-cap-affine-unproved",
    };
    let build=|ledger:&mut crate::rational_cap_translation::Report|->Result<Option<(LoftCap,Surface,f64)>> {
        for _ in 0..9 {if !step(ledger,max_work) {return Ok(None);}}
        let Some(inverse)=inverse(pose) else {ledger.reason="rational-cap-affine-singular";return Ok(None);};
        let mut world=Vec::new();
        for p in s.control_points.iter().flatten() {
            if !step(ledger,max_work) {return Ok(None);}
            world.push(linear(pose,p,origin));
        }
        let base=linear(pose,&anchor,origin);
        let mut candidate=cap.clone();
        for k in 0..3 {
            let values=world.iter().map(|v|v[k].clone()).collect::<Vec<_>>();
            let Some((values,modified))=crate::rational_cap_translation::axis_exact(&values,&base[k],&Q::from_float(pose[3][k]).unwrap(),ledger,max_work) else {return Ok(None);};
            for (p,v) in candidate.surface.control_points.iter_mut().flatten().zip(values) {p[k]=v;}
            ledger.modified_axes[k]=modified;
        }
        let mut bound:f64=0.;let mut reference=s.clone();
        for ((ideal,actual),reference) in world.iter().zip(candidate.surface.control_points.iter().flatten()).zip(reference.control_points.iter_mut().flatten()) {
            if !step(ledger,max_work) {return Ok(None);}
            let mut squared=I::point(0.);let mut different=false;
            for k in 0..3 {
                let delta=Q::from_float(actual[k]).unwrap()-(&ideal[k]+Q::from_float(pose[3][k]).unwrap());
                if delta.is_zero() {continue;}
                different=true;
                let Some(d)=delta.to_f64().filter(|d|d.is_finite()) else {ledger.reason="rational-cap-affine-error-unrepresentable";return Ok(None);};
                let radius=d.abs().next_up();squared=squared.add(I::point(radius).mul(I::point(radius))?)?;
            }
            if different {bound=bound.max(squared.hi.max(0.).sqrt().next_up());}
            for r in 0..3 {
                if !step(ledger,max_work) {return Ok(None);}
                let value=Q::from_float(origin[r]).unwrap()+(0..3).map(|k|&inverse[r][k]*(Q::from_float(actual[k]).unwrap()-Q::from_float(pose[3][k]).unwrap())).sum::<Q>();
                let Some(value)=exact(&value) else {ledger.reason="rational-cap-affine-reference-unrepresentable";return Ok(None);};reference[r]=value;
            }
        }
        ledger.displacement_upper=Some(bound);
        if !bound.is_finite() || bound>tolerance {ledger.reason="rational-cap-affine-error-budget-exceeded";return Ok(None);}
        Ok(Some((candidate,reference,bound)))
    };
    let value = build(&mut ledger)?;
    let mut out = Report {
        cap: None,
        reference: None,
        displacement_upper: ledger.displacement_upper,
        work: ledger.work,
        reason: ledger.reason,
    };
    if let Some((cap, reference, bound)) = value {
        out.cap = Some(cap);
        out.reference = Some(reference);
        out.displacement_upper = Some(bound);
        out.reason = "rational-cap-affine-proposal";
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_affine_inverse_and_full_sums_keep_cancellation_and_refuse_singular_maps() {
        // A triangular shear is neither diagonal nor a signed permutation.
        let p = [[1., 0., 0.], [2., 1., 0.], [0., 3., 1.], [0., 0., 2.]];
        let inv = inverse(p).unwrap();
        for r in 0..3 {
            for c in 0..3 {
                let v: Q = (0..3)
                    .map(|k| &inv[r][k] * Q::from_float(p[c][k]).unwrap())
                    .sum();
                assert_eq!(v, Q::from_integer(if r == c { 1.into() } else { 0.into() }));
            }
        }
        assert_eq!(image(p, &[1., 2., 3.], [0.; 3]), Some([5., 11., 5.]));
        let mut singular = p;
        singular[2] = singular[1];
        assert!(inverse(singular).is_none());
        // Both ordinary floating intermediate sums lose the unit; the full
        // exact affine sum is representable and must not be rejected.
        let q = [[1., 0., 0.], [1., 1., 0.], [-1., 0., 1.], [0.; 3]];
        let x = [2f64.powi(54), 1., 2f64.powi(54)];
        assert_eq!(image(q, &x, [0.; 3]), Some([1., 1., 2f64.powi(54)]));
        assert_ne!((x[0] + x[1]) - x[2], 1.);
    }
}
