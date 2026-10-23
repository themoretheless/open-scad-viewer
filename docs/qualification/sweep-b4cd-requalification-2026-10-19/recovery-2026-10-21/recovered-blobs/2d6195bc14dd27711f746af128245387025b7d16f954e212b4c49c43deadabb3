//! Original arc-length position chord bound, independent of the relative pose.
//! A continuous unit tangent and a complete regular curvature cover give a
//! Lipschitz tangent in arc length, even across parameter-speed changes.
use super::*;
use crate::numerics::interval_vec3::{cross, norm};

pub(super) struct Report {
    pub cells: usize,
    pub curvature_upper: Option<f64>,
}

pub(super) fn certify(path: &Curve, budget: usize) -> Result<Report> {
    check(budget <= 100000, "Arc position work exceeds100000 cells")?;
    let mut out = Report { cells: 0, curvature_upper: None };
    if budget == 0 { return Ok(out); }
    // Original position and positive tangent direction must agree at every
    // source knot. C2 parameter jets are unnecessary for geometric curvature.
    let tangent = if path.periodic {
        super::super::endpoint_jets::certify_knots(path, 1, budget as u64)?
    } else {
        super::super::endpoint_jets::certify_direction(path, 0, false, false, budget as u64)?
    };
    out.cells += tangent.exact_work as usize;
    if !tangent.certified || out.cells == budget { return Ok(out); }
    let (work, owner) = super::super::source_jets::SourceJets::new_order(path, 2, budget - out.cells)?;
    out.cells += work;
    let Some(owner) = owner else { return Ok(out); };
    let mut upper = 0_f64;
    // The dyadic cover includes every original source span and both sides of
    // knots. Failed or partial covers never publish a curvature bound.
    for i in 0..64 {
        let query = owner.query([i as f64 / 64., (i + 1) as f64 / 64.], budget - out.cells)?;
        out.cells += query.cells;
        let Some(jets) = query.jets else { return Ok(out); };
        let decode = |v: [[f64; 2]; 3]| -> Result<[I; 3]> {
            Ok([I::new(v[0][0], v[0][1])?, I::new(v[1][0], v[1][1])?, I::new(v[2][0], v[2][1])?])
        };
        let velocity = decode(jets[1])?;
        let acceleration = decode(jets[2])?;
        let speed = norm(velocity)?;
        if speed.lo <= 0. { return Ok(out); }
        let curvature = norm(cross(velocity, acceleration)?)?
            .div(speed.mul(speed)?.mul(speed)?)?;
        upper = upper.max(curvature.hi);
    }
    out.curvature_upper = Some(upper);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_circle_curvature_owns_complete_cover_and_work() {
        let path = crate::primitives::circle([3., -2., 7.], [0., 0., 1.], 10.).unwrap();
        let saved = path.clone();
        let report = certify(&path, 10000).unwrap();
        let upper = report.curvature_upper.unwrap();
        assert!(upper >= 0.1 && upper < 0.2, "{upper}");
        assert!(report.cells > 0 && report.cells <= 10000);
        assert!(certify(&path, 0).unwrap().curvature_upper.is_none());
        assert!(certify(&path, report.cells - 1).unwrap().curvature_upper.is_none());
        assert_eq!(path, saved);
    }
    #[test]
    fn quadratic_curvature_bound_is_independent_of_parameter_domain() {
        let source = crate::paths::bezier(
            vec![vec![0.,0.,0.],vec![0.5,0.,0.],vec![1.,1.,0.]], None,
        ).unwrap();
        let mut remapped = source.clone();
        for knot in &mut remapped.knots { *knot = 2. + 3. * *knot; }
        for path in [&source, &remapped] {
            let saved = path.clone();
            let proof = certify(path, 1024).unwrap();
            // P(u)=(u,u^2,0) has maximum geometric curvature 2.
            let upper = proof.curvature_upper.unwrap();
            assert!(upper >= 2. && upper < 2.1, "{upper}");
            assert!(proof.cells <= 1024);
            assert!(certify(path, proof.cells - 1).unwrap().curvature_upper.is_none());
            assert_eq!(*path, saved);
        }
    }
    #[test]
    fn original_corner_and_stationary_source_do_not_publish_curvature() {
        let corner = Curve { degree: 1, knots: vec![0.,0.,0.5,1.,1.],
            control_points: vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![1.,1.,0.]],
            weights: vec![1.;3], periodic: false };
        assert!(certify(&corner, 10000).unwrap().curvature_upper.is_none());
        let stationary = crate::paths::bezier(vec![vec![0.;3];3], None).unwrap();
        assert!(certify(&stationary, 10000).unwrap().curvature_upper.is_none());
    }
    #[test]
    fn unequal_parameter_speeds_keep_the_original_geometric_tangent_premise() {
        let source = Curve { degree: 1, knots: vec![0.,0.,0.25,1.,1.],
            control_points: vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![5.,0.,0.]],
            weights: vec![1.;3], periodic: false };
        let report = certify(&source, 10000).unwrap();
        assert!(report.curvature_upper.unwrap() < 1e-10);
    }
}
