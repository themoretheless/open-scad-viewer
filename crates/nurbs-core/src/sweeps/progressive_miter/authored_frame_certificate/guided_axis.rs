//! Authored longitudinal axis with guide-defined transverse orientation.
//! The guide is a world-space point rail; path jets use normalized traversal.
use super::*;

pub fn certify_authored_guide(
    longitudinal: &Curve,
    guide: &Curve,
    twist: &Curve,
    traversal: [f64; 2],
    path: [[f64; 2]; 3],
    velocity: [[f64; 2]; 3],
    max_cells: usize,
) -> Result<Report> {
    check(max_cells <= 100000, "Authored guide frame budget exceeds100000 cells")?;
    super::super::validate_law(twist, false)?;
    let mut out = Report {
        status: Status::Unresolved, cells: 0, longitudinal: None, transverse: None,
        binormal: None, single_span: false, reason: Some("authored-axis-enclosure-unresolved"),
    };
    let axis = vector_certificate::certify_traversal(longitudinal, traversal, max_cells, false)?;
    out.cells += axis.cells;
    if axis.status != Status::Certified { return Ok(out); }
    out.reason = Some("guide-law-enclosure-unresolved");
    let rail = vector_certificate::certify_traversal(guide, traversal, max_cells - out.cells, false)?;
    out.cells += rail.cells;
    if rail.status != Status::Certified { return Ok(out); }
    out.reason = Some("authored-axis-nonzero-unproved");
    let Some(t) = normalize(jet(longitudinal, &axis)?)? else { return Ok(out); };
    let g = jet(guide, &rail)?;
    let offset = Jet {
        v: sub(g.v, decode(path)?)?, d: sub(g.d, decode(velocity)?)?, dd: g.dd,
    };
    out.reason = Some("authored-guide-transverse-direction-unproved");
    let Some(b) = normalize(cross_jet(t, offset)?)? else { return Ok(out); };
    let n = cross_jet(b, t)?;
    out.longitudinal = Some(encode(t));
    out.transverse = Some(encode(n));
    out.binormal = Some(encode(b));
    out.single_span = axis.single_span && rail.single_span;
    out.reason = None;
    out.status = Status::Certified;
    apply_twist(out, twist, traversal, max_cells)
}

/// Point restrictions use values only, avoiding zero-width derivative division.
pub fn certify_authored_guide_values(
    longitudinal: &Curve,
    guide: &Curve,
    twist: &Curve,
    traversal: [f64; 2],
    path: [[f64; 2]; 3],
    max_cells: usize,
) -> Result<ValuesReport> {
    check(max_cells <= 100000, "Authored guide frame budget exceeds100000 cells")?;
    super::super::validate_law(twist, false)?;
    let mut out = ValuesReport {
        status: Status::Unresolved, cells: 0, longitudinal: None, transverse: None, binormal: None,
    };
    let axis = vector_certificate::certify_values_traversal(longitudinal, traversal, max_cells, false)?;
    out.cells += axis.cells;
    let Some(axis) = axis.value else { return Ok(out); };
    let rail = vector_certificate::certify_values_traversal(guide, traversal, max_cells - out.cells, false)?;
    out.cells += rail.cells;
    let Some(rail) = rail.value else { return Ok(out); };
    let charge = (twist.degree..twist.control_points.len())
        .filter(|&i| twist.knots[i] < twist.knots[i + 1]).count();
    if charge > max_cells - out.cells { return Ok(out); }
    let theta = super::super::scalar_certificate::value_traversal(twist, traversal, charge)?;
    out.cells += charge;
    let Some(theta) = theta else { return Ok(out); };
    let constant = |v| Jet { v, d: [I::point(0.); 3], dd: [I::point(0.); 3] };
    let Some(t) = normalize(constant(decode(axis)?))? else { return Ok(out); };
    let offset = sub(decode(rail)?, decode(path)?)?;
    let Some(b) = normalize(constant(cross(t.v, offset)?))? else { return Ok(out); };
    let n = cross(b.v, t.v)?;
    let tr = super::super::trigonometric_certificate::certify(theta)?;
    let rotated = add(mul(n, I::new(tr.cos[0], tr.cos[1])?)?, mul(b.v, I::new(tr.sin[0], tr.sin[1])?)?)?;
    out.longitudinal = Some(t.v.map(|x| [x.lo, x.hi]));
    out.transverse = Some(rotated.map(|x| [x.lo, x.hi]));
    out.binormal = Some(cross(t.v, rotated)?.map(|x| [x.lo, x.hi]));
    out.status = Status::Certified;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn law(points: Vec<Vec<f64>>, domain: [f64; 2]) -> Curve {
        Curve { degree: 1, knots: vec![domain[0],domain[0],domain[1],domain[1]],
            weights: vec![1.;points.len()], control_points: points, periodic: false }
    }
    fn fixtures() -> (Curve,Curve,Curve) {
        (law(vec![vec![0.,0.,1.],vec![0.,1.,1.]],[2.,5.]),
         law(vec![vec![1.,0.,0.],vec![1.,0.,10.]],[7.,11.]),
         law(vec![vec![0.,0.,0.],vec![0.,0.,0.]],[-9.,-3.]))
    }
    #[test]
    fn moving_authored_axis_guide_encloses_original_normalized_jets() {
        let (axis,guide,twist)=fixtures();
        let before=(axis.clone(),guide.clone(),twist.clone());
        for i in 0..50 {
            let lo=i as f64/50.;let hi=(i+1) as f64/50.;
            let r=certify_authored_guide(&axis,&guide,&twist,[lo,hi],
                [[0.,0.],[0.,0.],[10.*lo,10.*hi]],[[0.,0.],[0.,0.],[10.,10.]],7).unwrap();
            assert_eq!(r.status,Status::Certified);assert_eq!(r.cells,7);
            for f in [lo,(lo+hi)/2.,hi] {
                let h=(1.+f*f).sqrt();
                let t=[0.,f/h,1./h];let td=[0.,1./h.powi(3),-f/h.powi(3)];
                let tdd=[0.,-3.*f/h.powi(5),(2.*f*f-1.)/h.powi(5)];
                for (a,v,d,dd) in [
                    (r.longitudinal.as_ref().unwrap(),t,td,tdd),
                    (r.transverse.as_ref().unwrap(),[1.,0.,0.],[0.;3],[0.;3]),
                    (r.binormal.as_ref().unwrap(),[0.,t[2],-t[1]],[0.,td[2],-td[1]],[0.,tdd[2],-tdd[1]])] {
                    for k in 0..3 {for (range,x) in [(a.value[k],v[k]),(a.first[k],d[k]),(a.second[k],dd[k])] {
                        assert!(range[0]<=x && x<=range[1],"{range:?} misses {x}");
                    }}
                }
            }
        }
        assert_eq!(axis.control_points,before.0.control_points);
        assert_eq!(guide.knots,before.1.knots);assert_eq!(twist.weights,before.2.weights);
    }
    #[test]
    fn authored_guide_points_share_budget_and_refuse_parallel_rails() {
        let (axis,guide,twist)=fixtures();
        for f in [0.,0.5,1.] {
            let path=[[0.,0.],[0.,0.],[10.*f,10.*f]];
            let r=certify_authored_guide_values(&axis,&guide,&twist,[f,f],path,7).unwrap();
            assert_eq!(r.status,Status::Certified);assert_eq!(r.cells,7);
            for budget in [0,2,3,5,6] {
                let r=certify_authored_guide_values(&axis,&guide,&twist,[f,f],path,budget).unwrap();
                assert_eq!(r.status,Status::Unresolved);assert!(r.cells<=budget);
                assert!(r.longitudinal.is_none() && r.transverse.is_none() && r.binormal.is_none());
            }
        }
        for budget in [0,2,3,5,6] {
            let r=certify_authored_guide(&axis,&guide,&twist,[0.,0.02],
                [[0.,0.],[0.,0.],[0.,0.2]],[[0.,0.],[0.,0.],[10.,10.]],budget).unwrap();
            assert_eq!(r.status,Status::Unresolved);assert!(r.cells<=budget);
            assert!(r.longitudinal.is_none() && r.transverse.is_none() && r.binormal.is_none());
        }
        let parallel=law(vec![vec![0.,0.,1.],vec![0.,1.,11.]],[7.,11.]);
        let r=certify_authored_guide_values(&axis,&parallel,&twist,[0.,0.],[[0.,0.];3],7).unwrap();
        assert_eq!(r.status,Status::Unresolved);
    }
}
