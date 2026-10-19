//! Original arc-station path chord error, including inverse-length residuals.
use super::*;
pub(in crate::sweeps::progressive_sweep) struct GlobalChord {
    pub cells:usize,
    pub bound:Option<f64>,
    pub budget_stopped:bool,
}
/// One immutable original source/count/length proof for a complete profile
/// union. The MultiSweep owner binds identical path and station policy and
/// charges these cells ONCE before passing the opaque result to its profiles.
pub(in crate::sweeps::progressive_sweep) fn global(path:&Curve,count:usize,length:[f64;2],max:usize)->Result<GlobalChord>{
    path.validate()?;
    check(count>=2 && count<=16384 && max<=100000,"Invalid shared arc chord limits")?;
    let mut cells=0;let mut width=0_f64;
    for i in 0..count-1 {
        if cells==max {return Ok(GlobalChord {cells,bound:None,budget_stopped:true});}
        cells+=1;
        let lo=i as f64/(count-1)as f64;let hi=(i+1)as f64/(count-1)as f64;
        width=width.max(I::point(hi).sub(I::point(lo))?.hi);
    }
    if cells==max {return Ok(GlobalChord {cells,bound:None,budget_stopped:true});}
    let (work,bound)=upper(path,[0.,1.],[0.,width],length,0.,false,max-cells)?;
    cells+=work;
    Ok(GlobalChord {cells,budget_stopped:bound.is_none() && cells==max,bound})
}
pub(in crate::sweeps::progressive_sweep::authored_error) fn upper(
    path: &Curve,
    traversal: [f64; 2],
    station: [f64; 2],
    length: [f64; 2],
    residual: f64,
    geometric_c1: bool,
    max: usize,
) -> Result<(usize, Option<f64>)> {
    let source = vector_certificate::certify_traversal(path, traversal, max, false)?;
    if source.status != Status::Certified || !(source.single_span || geometric_c1) {
        return Ok((source.cells, None));
    }
    let mut spent = source.cells;
    let h = I::point(station[1]).sub(I::point(station[0]))?;
    // A proposal relative to the existing first-order chord bound. All
    // accepted leaves still carry complete outward original derivatives.
    let target = I::point(length[1]).mul(h)?.div(I::point(16.))?.hi;
    let mut pending = vec![(traversal, Some(source))];
    let mut error = 0_f64;
    while let Some((range, cached)) = pending.pop() {
        if spent == max {
            return Ok((spent, None));
        }
        spent += 1; // original derivative/refinement cell, including failure
        let source = if let Some(source) = cached {
            source
        } else {
            let source = vector_certificate::certify_traversal(path, range, max - spent, false)?;
            spent += source.cells;
            source
        };
        if source.status != Status::Certified {
            return Ok((spent, None));
        }
        let bound = bound(&source, station, length, residual).ok();
        if let Some(bound) = bound.filter(|b| {
            b.hi <= target
                // Refinement cannot drive a true nonzero curvature below the
                // target. A narrow outward curvature interval is already a
                // complete source proof, even when its lower bound is larger.
                || b.lo > 0. && b.hi <= b.lo * (17. / 16.)
                || range[1] - range[0] <= 2_f64.powi(-9)
        }) {
            error = error.max(bound.hi);
            continue;
        }
        // A bounded proposal must not chase an unattainable speed enclosure
        // at a stationary endpoint forever. No partial derivative cover is
        // returned; the owner may keep its independent coarse error bound.
        if range[1] - range[0] <= 2_f64.powi(-9) {
            return Ok((spent, None));
        }
        let mid = crate::distance_bounds::midpoint(range[0], range[1]);
        if !(range[0] < mid && mid < range[1]) {
            return Ok((spent, None));
        }
        pending.extend([([range[0], mid], None), ([mid, range[1]], None)]);
    }
    Ok((spent, Some(error)))
}
fn bound(
    source: &vector_certificate::Report,
    station: [f64; 2],
    length: [f64; 2],
    residual: f64,
) -> Result<I> {
    let decode = |b: [[f64; 2]; 3]| -> Result<[I; 3]> {
        Ok([
            I::new(b[0][0], b[0][1])?,
            I::new(b[1][0], b[1][1])?,
            I::new(b[2][0], b[2][1])?,
        ])
    };
    let v = decode(source.first.unwrap())?;
    let a = decode(source.second.unwrap())?;
    let speed = crate::numerics::interval_vec3::norm(v)?;
    check(
        speed.lo > 0.,
        "Arc chord needs a positive whole-source speed",
    )?;
    // For normalized arc length s, C_s=L*v/|v|, hence
    // |C_ss|=L^2*|v cross a|/|v|^3. The scalar identity removes the
    // repeated correlated tangent from (omega cross tangent)/speed.
    // Both ends enclose the curvature at EVERY point in this source cell.
    let cross = crate::numerics::interval_vec3::norm(crate::numerics::interval_vec3::cross(v, a)?)?;
    let l = I::new(length[0], length[1])?;
    let second = l.mul(l)?.mul(cross)?.div(speed.mul(speed)?.mul(speed)?)?;
    let h = I::point(station[1]).sub(I::point(station[0]))?;
    // The source transport owns exact positional/forward-tangent joins,
    // or this query is a single original span. Thus the piecewise second
    // derivative remainder is valid; bounded one-sided jets alone are not.
    second
        .mul(h)?
        .mul(h)?
        .div(I::point(8.))?
        .add(I::point(residual))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn curved_original_chord_converges_above_the_refinement_target() {
        // P(u)=(0,u^2,u): |P_u cross P_uu|=2 and |P_u|>=1.
        // Its normalized-arc second derivative has maximum 2*L^2 at u=0.
        // The h=1/2 remainder proxy L^2/16 exceeds the L/32 proposal;
        // refining a valid enclosure cannot make that proxy vanish.
        let length = [1.4789,1.4790];
        for domain in [[0.,1.],[-3.,5.]] {
            let path=Curve {degree:2,
                knots:vec![domain[0],domain[0],domain[0],domain[1],domain[1],domain[1]],
                control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],
                weights:vec![2.;3],periodic:false};
            let (used,upper)=upper(&path,[0.,1.],[0.,0.5],length,0.,false,10000).unwrap();
            let upper=upper.expect("A complete original derivative cover must fit the existing budget");
            assert!(used>0 && used<10000,"used={used}");
            assert!(upper >= length[0]*length[0]/16.);
            assert!(upper <= length[1]*length[1]/16.*(17./16.));
            for budget in [0,used-1] {
                let (spent,bound)=super::upper(&path,[0.,1.],[0.,0.5],length,0.,false,budget).unwrap();
                assert!(bound.is_none() && spent<=budget,"budget={budget}: {spent}/{bound:?}");
            }
        }
    }
    #[test]
    fn original_arc_chord_preserves_domain_units_and_inverse_residual() {
        let path = Curve {
            degree: 2,
            knots: vec![-3., -3., -3., 5., 5., 5.],
            control_points: vec![vec![0.; 3], vec![0., 0., 0.5], vec![0., 0., 2.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let (used, bound) =
            upper(&path, [0.1, 0.3], [0.1, 0.3], [2., 2.], 0.001, false, 1000).unwrap();
        assert!(used > 0);
        let bound = bound.unwrap();
        assert!(bound >= 0.001 && bound < 0.001000000001);
        let (spent, b) = upper(
            &path,
            [0.1, 0.3],
            [0.1, 0.3],
            [2., 2.],
            0.001,
            false,
            used - 1,
        )
        .unwrap();
        assert!(b.is_none() && spent <= used - 1);
        let (spent, b) = upper(&path, [0.1, 0.3], [0.1, 0.3], [2., 2.], 0.001, false, 0).unwrap();
        assert!(b.is_none());
        assert_eq!(spent, 0);
    }
    #[test]
    fn a_corner_with_bounded_one_sided_second_jets_has_no_quadratic_remainder_credit() {
        let path = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![0.; 3], vec![1., 0., 0.], vec![1., 1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        assert!(
            upper(&path, [0.49, 0.51], [0.49, 0.51], [2., 2.], 0., false, 1000)
                .unwrap()
                .1
                .is_none()
        );
    }
}
