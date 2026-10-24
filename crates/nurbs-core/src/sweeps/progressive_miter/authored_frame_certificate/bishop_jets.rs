//! Original Bishop jets from an OWNED original normal cover and angular ODE.
//! Reparameterization and twist stay in the declared station units. Both N and
//! B use the SAME angular field; cross-jet cancellation is not discarded.
use super::*;
#[derive(Clone, Copy)]
pub(crate) enum BishopCorrection {
    Parameter {
        angle: I,
        length: I,
    },
    /// Only with the SAME original path's normalized arc inverse owner.
    OriginalArc {
        angle: I,
    },
}
pub(crate) fn certify_bishop_jets(
    path: &Curve,
    normal: [[f64; 2]; 3],
    twist: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<Report> {
    certify_bishop_phase_jets(
        path,
        normal,
        twist,
        traversal,
        PathPhase::identity(traversal),
        max_cells,
        None,
    )
}
pub(crate) fn certify_arc_corrected_bishop_jets(
    path: &Curve,
    normal: [[f64; 2]; 3],
    twist: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
    angle: [f64; 2],
    length: [f64; 2],
) -> Result<Report> {
    certify_bishop_phase_jets(
        path,
        normal,
        twist,
        traversal,
        PathPhase::identity(traversal),
        max_cells,
        Some(BishopCorrection::Parameter {
            angle: I::new(angle[0], angle[1])?,
            length: I::new(length[0], length[1])?,
        }),
    )
}
/// Original corrected or raw N values on `phase.traversal` must belong to the
/// SAME path and correction owner. No sample/fitted reflection derivative.
pub(crate) fn certify_bishop_phase_jets(
    path: &Curve,
    normal: [[f64; 2]; 3],
    twist: &Curve,
    station: [f64; 2],
    phase: PathPhase,
    max_cells: usize,
    correction: Option<BishopCorrection>,
) -> Result<Report> {
    super::super::validate_law(twist, false)?;
    let p = vector_certificate::certify_third_traversal(path, phase.traversal, max_cells)?;
    let mut out = Report {
        status: Status::Unresolved,
        cells: p.base.cells,
        longitudinal: None,
        transverse: None,
        binormal: None,
        single_span: false,
        reason: Some("bishop-original-path-jets-unproved"),
    };
    if p.base.status != Status::Certified {
        return Ok(out);
    }
    let position = jet(path, &p.base)?;
    let [a, b] = path.domain();
    let width = I::point(b).sub(I::point(a))?;
    let jerk = mul(decode(p.third.unwrap())?, width.mul(width)?.mul(width)?)?;
    let v = position.d;
    let acc = position.dd;
    let Some(mut tangent) = normalize(Jet {
        v,
        d: [I::point(0.); 3],
        dd: [I::point(0.); 3],
    })?
    else {
        out.reason = Some("bishop-original-speed-unproved");
        return Ok(out);
    };
    let speed2 = square(v[0])?.add(square(v[1])?)?.add(square(v[2])?)?;
    if speed2.lo <= 0. {
        out.reason = Some("bishop-original-speed-unproved");
        return Ok(out);
    }
    let raw = div(cross(v, acc)?, speed2)?;
    let raw_d = sub(
        div(cross(v, jerk)?, speed2)?,
        mul(raw, dot(v, acc)?.mul(I::point(2.))?.div(speed2)?)?,
    )?;
    tangent.d = cross(raw, tangent.v)?;
    tangent.dd = add(cross(raw_d, tangent.v)?, cross(raw, tangent.d)?)?;
    let tangent = super::phase::compose(tangent, phase)?;
    let (omega, omega_d) = if phase.first.lo == 1.
        && phase.first.hi == 1.
        && phase.second.lo == 0.
        && phase.second.hi == 0.
    {
        (raw, raw_d)
    } else {
        (
            mul(raw, phase.first)?,
            add(
                mul(raw_d, phase.first.mul(phase.first)?)?,
                mul(raw, phase.second)?,
            )?,
        )
    };
    let (alpha_d, alpha_dd) = match correction {
        None => (I::point(0.), I::point(0.)),
        Some(BishopCorrection::OriginalArc { angle }) => (angle, I::point(0.)),
        Some(BishopCorrection::Parameter { angle, length }) => {
            if length.lo <= 0. {
                out.reason = Some("bishop-original-arc-length-unproved");
                return Ok(out);
            }
            let speed = I::new(
                speed2.lo.sqrt().next_down().max(0.),
                speed2.hi.sqrt().next_up(),
            )?;
            let first = angle.mul(speed)?.div(length)?;
            let second = angle.mul(dot(v, acc)?.div(speed)?)?.div(length)?;
            (
                first.mul(phase.first)?,
                second
                    .mul(phase.first.mul(phase.first)?)?
                    .add(first.mul(phase.second)?)?,
            )
        }
    };
    let theta =
        super::super::scalar_certificate::certify_traversal(twist, station, max_cells - out.cells)?;
    out.cells += theta.cells;
    if theta.status != Status::Certified {
        out.reason = Some("twist-law-enclosure-unresolved");
        return Ok(out);
    }
    let zero = twist.control_points.iter().all(|p| p[0] == 0.);
    let [a, b] = twist.domain();
    let width = I::point(b).sub(I::point(a))?;
    let beta_d = if zero {
        I::point(0.)
    } else {
        I::new(theta.first.unwrap()[0], theta.first.unwrap()[1])?.mul(width)?
    };
    let beta_dd = if zero {
        I::point(0.)
    } else {
        I::new(theta.second.unwrap()[0], theta.second.unwrap()[1])?.mul(width.mul(width)?)?
    };
    let n = decode(normal)?;
    let n = if zero {
        n
    } else {
        let trig = super::super::trigonometric_certificate::certify(theta.value.unwrap())?;
        add(
            mul(n, I::new(trig.cos[0], trig.cos[1])?)?,
            mul(cross(tangent.v, n)?, I::new(trig.sin[0], trig.sin[1])?)?,
        )?
    };
    let spin = alpha_d.add(beta_d)?;
    let spin_d = alpha_dd.add(beta_dd)?;
    let omega = add(omega, mul(tangent.v, spin)?)?;
    let omega_d = add(
        add(omega_d, mul(tangent.v, spin_d)?)?,
        mul(tangent.d, spin)?,
    )?;
    let rotating = |v: V| -> Result<Jet> {
        let d = cross(omega, v)?;
        Ok(Jet {
            v,
            d,
            dd: add(cross(omega_d, v)?, cross(omega, d)?)?,
        })
    };
    let normal = rotating(n)?;
    let binormal = rotating(cross(tangent.v, n)?)?;
    out.longitudinal = Some(encode(tangent));
    out.transverse = Some(encode(normal));
    out.binormal = Some(encode(binormal));
    out.status = Status::Certified;
    out.single_span = p.base.single_span && theta.single_span;
    out.reason = None;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_arc_correction_jets_include_both_nonuniform_clock_derivatives() {
        // Exact forward line P(t)=(0,0,t+t²), with original length 2.
        // A specified angular correction 1/2 has alpha=(t+t²)/4,
        // alpha'=(1+2t)/4 and alpha''=1/2. This is an OPEN analytic field;
        // no holonomy or closed-volume claim is made by this local calculus test.
        let path = Curve {
            degree: 2,
            knots: vec![-3., -3., -3., 5., 5., 5.],
            control_points: vec![vec![0., 0., 0.], vec![0., 0., 0.5], vec![0., 0., 2.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        // Use a separately authored zero twist; its original domain is unrelated.
        let twist = Curve {
            degree: 1,
            knots: vec![7., 7., 11., 11.],
            control_points: vec![vec![0.; 3]; 2],
            weights: vec![1.; 2],
            periodic: false,
        };
        for t in [0.125_f64, 0.5, 0.875] {
            let range = [t - 1e-4, t + 1e-4];
            let s = I::new(range[0], range[1]).unwrap();
            let alpha = s.add(s.mul(s).unwrap()).unwrap().div(I::point(4.)).unwrap();
            let tr = super::super::super::trigonometric_certificate::certify([alpha.lo, alpha.hi])
                .unwrap();
            let normal = [tr.cos, tr.sin, [0., 0.]];
            let r = certify_arc_corrected_bishop_jets(
                &path,
                normal,
                &twist,
                range,
                1000,
                [0.5, 0.5],
                [2., 2.],
            )
            .unwrap();
            assert_eq!(r.status, Status::Certified);
            let alpha = (t + t * t) / 4.;
            let d = (1. + 2. * t) / 4.;
            let dd = 0.5;
            let value = [alpha.cos(), alpha.sin(), 0.];
            let first = [-d * alpha.sin(), d * alpha.cos(), 0.];
            let second = [
                -dd * alpha.sin() - d * d * alpha.cos(),
                dd * alpha.cos() - d * d * alpha.sin(),
                0.,
            ];
            let n = r.transverse.as_ref().unwrap();
            for (box3, expected) in [n.value, n.first, n.second]
                .into_iter()
                .zip([value, first, second])
            {
                for k in 0..3 {
                    assert!(box3[k][0] <= expected[k] && expected[k] <= box3[k][1]);
                    assert!(box3[k][1] - box3[k][0] < 0.001);
                }
            }
            let short = certify_arc_corrected_bishop_jets(
                &path,
                normal,
                &twist,
                range,
                r.cells - 1,
                [0.5, 0.5],
                [2., 2.],
            )
            .unwrap();
            assert_eq!(short.status, Status::Unresolved);
            assert!(short.transverse.is_none());
        }
    }
}
