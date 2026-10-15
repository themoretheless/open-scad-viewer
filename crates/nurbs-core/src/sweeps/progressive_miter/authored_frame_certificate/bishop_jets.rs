//! Original Bishop frame jets from an owned normal-value cover and the ODE.
//! The supplied N box must enclose the original orthonormal Bishop solution
//! on this SAME path interval. Only the native transport owner calls this.
use super::*;

pub(crate) fn certify_bishop_jets(
    path: &Curve,
    normal: [[f64; 2]; 3],
    twist: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<Report> {
    super::super::validate_law(twist, false)?;
    let p = vector_certificate::certify_third_traversal(path, traversal, max_cells)?;
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
    let velocity = Jet {
        v: position.d,
        d: position.dd,
        dd: jerk,
    };
    let Some(mut t) = normalize(Jet {
        v: velocity.v,
        d: [I::point(0.); 3],
        dd: [I::point(0.); 3],
    })?
    else {
        out.reason = Some("bishop-original-speed-unproved");
        return Ok(out);
    };
    let speed2 = square(velocity.v[0])?
        .add(square(velocity.v[1])?)?
        .add(square(velocity.v[2])?)?;
    if speed2.lo <= 0. {
        out.reason = Some("bishop-original-speed-unproved");
        return Ok(out);
    }
    let omega = div(cross(velocity.v, velocity.d)?, speed2)?;
    let q_rate = dot(velocity.v, velocity.d)?
        .mul(I::point(2.))?
        .div(speed2)?;
    let omega_d = sub(
        div(cross(velocity.v, velocity.dd)?, speed2)?,
        mul(omega, q_rate)?,
    )?;
    // T' = omega x T preserves cancellation of tangential acceleration.
    // Differentiating that identity also encloses T'' without subtracting
    // two large independently rounded longitudinal terms.
    t.d = cross(omega, t.v)?;
    t.dd = add(cross(omega_d, t.v)?, cross(omega, t.d)?)?;
    let n = decode(normal)?;
    // N' = omega x N; N'' = omega' x N + omega x N'. No
    // differentiation of a sampled reflection, fitted normal, or value box.
    let nd = cross(omega, n)?;
    let n = Jet {
        v: n,
        d: nd,
        dd: add(cross(omega_d, n)?, cross(omega, nd)?)?,
    };
    let b = cross_jet(t, n)?;
    out.status = Status::Certified;
    out.longitudinal = Some(encode(t));
    out.transverse = Some(encode(n));
    out.binormal = Some(encode(b));
    out.single_span = p.base.single_span;
    out.reason = None;
    apply_twist(out, twist, traversal, max_cells)
}
