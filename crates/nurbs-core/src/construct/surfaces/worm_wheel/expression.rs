//! Shared exact analytic expression for interval values and derivative algebras.
use super::*;
pub(super) trait Scalar: Copy {
    fn c(x: I) -> Self;
    fn value(self) -> I;
    fn add(self, b: Self) -> Result<Self>;
    fn sub(self, b: Self) -> Result<Self>;
    fn mul(self, b: Self) -> Result<Self>;
    fn div(self, b: Self) -> Result<Self>;
    fn sin_cos(self) -> Result<(Self, Self)>;
    fn sqrt(self) -> Result<Self>;
    fn asin(self) -> Result<Self>;
    /// Intersects coefficient zero with an independently proved source bound.
    fn restrict_value(self, x: I) -> Result<Self>;
}
pub(super) struct Expression<S> {
    pub point: [S; 3],
    pub roll: S,
    pub phi: S,
    pub denominator: S,
}
struct Contact<S> {
    x: S,
    y: S,
    z: S,
    profile_z: S,
    h: S,
    ratio: S,
    denominator: S,
}
pub(super) struct Prepared {
    p: worm::ParameterBounds,
    flank: Flank,
    h: I,
    ratio: I,
    wheel_radius: I,
    slope: I,
    period_offset: I,
    shift: I,
}
/// Ephemeral original-source constants, rebuilt at each admission/replay.
pub(super) fn prepare(spec: Spec, flank: Flank) -> Result<Prepared> {
    let p = worm::derive_parameters(spec.worm)?;
    let sign = if spec.worm.hand == worm::Hand::Right {
        1.
    } else {
        -1.
    };
    let h = I::point(spec.worm.axial_module)
        .mul(I::point(spec.worm.starts as f64))?
        .div(I::point(2.))?
        .mul(I::point(sign))?;
    let ratio = I::point(-sign * spec.worm.starts as f64).div(I::point(spec.teeth as f64))?;
    let wheel_radius = I::point(spec.worm.axial_module)
        .mul(I::point(spec.teeth as f64))?
        .div(I::point(2.))?;
    let (sp, cp) = crate::involute::certified_trig(I::point(spec.worm.pressure_angle))?;
    let period_offset = iv(p.pitch)?.mul(I::point(spec.start as f64))?;
    Ok(Prepared {
        p,
        flank,
        h,
        ratio,
        wheel_radius,
        slope: sp.div(cp)?,
        period_offset,
        shift: I::point(spec.axial_shift),
    })
}
fn contact<S: Scalar>(source: &Prepared, u: S, beta: S) -> Result<Contact<S>> {
    let p = &source.p;
    let flank = source.flank;
    let slope = source.slope;
    let h = S::c(source.h);
    let ratio = S::c(source.ratio);
    let rp = S::c(iv(p.pitch_radius)?);
    let center = rp.add(S::c(source.wheel_radius))?;
    let (r0, r1, z0, z1, k) = match flank {
        Flank::Descending => (
            p.crest_radius,
            p.root_radius,
            p.profile_z[1],
            p.profile_z[2],
            neg(slope)?,
        ),
        Flank::Ascending => (
            p.root_radius,
            p.crest_radius,
            p.profile_z[3],
            p.profile_z[4],
            slope,
        ),
    };
    let lerp = |a: [f64; 2], b: [f64; 2]| -> Result<S> {
        S::c(iv(a)?).add(S::c(iv(b)?.sub(iv(a)?)?).mul(u)?)
    };
    let r = lerp(r0, r1)?;
    let profile_z = lerp(z0, z1)?
        .add(S::c(source.period_offset))?
        .add(S::c(source.shift))?;
    let (sn, cs) = beta.sin_cos()?;
    let denominator = S::c(k).mul(cs)?.sub(h.div(r)?.mul(sn)?)?;
    check(
        denominator.value().lo > 0. || denominator.value().hi < 0.,
        "Wheel chart contact denominator is not separated from zero",
    )?;
    let z = rp.sub(r.mul(cs)?)?.div(denominator)?;
    Ok(Contact {
        x: r.mul(cs)?.sub(center)?,
        y: r.mul(sn)?,
        z,
        profile_z,
        h,
        ratio,
        denominator,
    })
}
/// Rotation-invariant source equation; no wheel-frame trig is needed here.
pub(super) fn radial_squared<S: Scalar>(spec: Spec, flank: Flank, u: S, beta: S) -> Result<S> {
    radial_prepared(&prepare(spec, flank)?, u, beta)
}
pub(super) fn radial_prepared<S: Scalar>(source: &Prepared, u: S, beta: S) -> Result<S> {
    let c = contact(source, u, beta)?;
    c.x.mul(c.x)?.add(c.z.mul(c.z)?)
}
pub(super) fn evaluate<S: Scalar>(
    spec: Spec,
    flank: Flank,
    u: S,
    beta: S,
) -> Result<Expression<S>> {
    let c = contact(&prepare(spec, flank)?, u, beta)?;
    let theta = c.z.sub(c.profile_z)?.div(c.h)?;
    let phi = beta.sub(theta)?;
    let (sa, ca) = c.ratio.mul(phi)?.sin_cos()?;
    Ok(Expression {
        point: [
            ca.mul(c.x)?.sub(sa.mul(c.z)?)?,
            c.y,
            sa.mul(c.x)?.add(ca.mul(c.z)?)?,
        ],
        roll: theta,
        phi,
        denominator: c.denominator,
    })
}

pub(super) fn root<S: Scalar>(spec: Spec, part: RootPart, t: S, beta: S) -> Result<Expression<S>> {
    match part {
        RootPart::CrestEdge(flank) => {
            let source = prepare(spec, flank)?;
            let crest = S::c(I::point(if flank == Flank::Descending { 0. } else { 1. }));
            let c = contact(&source, crest, beta)?;
            let z = t.mul(c.z)?;
            let theta = z.sub(c.profile_z)?.div(c.h)?;
            let phi = beta.sub(theta)?;
            let (sa, ca) = c.ratio.mul(phi)?.sin_cos()?;
            Ok(Expression {
                point: [
                    ca.mul(c.x)?.sub(sa.mul(z)?)?,
                    c.y,
                    sa.mul(c.x)?.add(ca.mul(z)?)?,
                ],
                roll: theta,
                phi,
                denominator: c.denominator,
            })
        }
        RootPart::CrestCylinder => {
            let s = prepare(spec, Flank::Descending)?;
            let z0 = S::c(iv(s.p.profile_z[4])?)
                .add(S::c(iv(s.p.crest_width)?).mul(t)?)?
                .add(S::c(s.period_offset))?
                .add(S::c(s.shift))?;
            let theta = S::c(I::point(0.)).sub(z0)?.div(S::c(s.h))?;
            let phi = beta.sub(theta)?;
            let (sa, ca) = S::c(s.ratio).mul(phi)?.sin_cos()?;
            let (sn, cs) = beta.sin_cos()?;
            let rc = S::c(iv(s.p.crest_radius)?);
            let center = S::c(iv(s.p.pitch_radius)?).add(S::c(s.wheel_radius))?;
            let x = rc.mul(cs)?.sub(center)?;
            Ok(Expression {
                point: [ca.mul(x)?, rc.mul(sn)?, sa.mul(x)?],
                roll: theta,
                phi,
                denominator: cs,
            })
        }
    }
}

pub(super) fn sqrt_positive(x: I) -> Result<I> {
    check(
        x.lo > 0.,
        "Derivative square root requires a strictly positive argument",
    )?;
    I::new(x.lo.sqrt().next_down().max(0.), x.hi.sqrt().next_up())
}
/// Principal inverse sine, bounded by monotonic endpoints and certified atan.
/// The complementary identity covers arguments near +/-1 without large rolls.
pub(super) fn asin_interval(x: I) -> Result<I> {
    check(
        x.lo > -1. && x.hi < 1.,
        "Inverse sine derivatives require arguments strictly inside (-1,1)",
    )?;
    let endpoint = |v: f64| -> Result<I> {
        if v == 0. {
            return Ok(I::point(0.));
        }
        let a = I::point(v.abs());
        let c = sqrt_positive(I::point(1.).sub(a.mul(a)?)?)?;
        let value = if v.abs() <= 0.5 {
            crate::involute::certified_atan_positive(a.div(c)?)?
        } else {
            crate::involute::certified_pi()?
                .div(I::point(2.))?
                .sub(crate::involute::certified_atan_positive(c.div(a)?)?)?
        };
        if v < 0. { neg(value) } else { Ok(value) }
    };
    I::new(endpoint(x.lo)?.lo, endpoint(x.hi)?.hi)
}

pub(super) fn axial_root<S: Scalar>(
    spec: Spec,
    part: RootPart,
    t: S,
    y: S,
) -> Result<Expression<S>> {
    let p = worm::derive_parameters(spec.worm)?;
    let beta = y.div(S::c(iv(p.crest_radius)?))?.asin()?;
    let mut result = root(spec, part, t, beta)?;
    // Exact-real source identity Rc*sin(asin(Y/Rc))=Y. Keep physical Y
    // explicit so end-plane and derivative evidence never depends on snapping.
    result.point[1] = y;
    Ok(result)
}

/// A complete closed angular cover tightens the scalar range used for admission.
/// Each child still uses the original certified Taylor/derivative bound.
pub(super) fn contact_trig_range(x: I) -> Result<(I, I)> {
    if x.lo == x.hi {
        return crate::involute::certified_trig(x);
    }
    let mut sin = None::<I>;
    let mut cos = None::<I>;
    let mut left = x.lo;
    for i in 1..=16 {
        let right = if i == 16 {
            x.hi
        } else {
            x.lo + (x.hi - x.lo) * (i as f64 / 16.)
        };
        check(left <= right, "Contact angle cover lost parameter ordering")?;
        let (s, c) = crate::involute::certified_trig(I::new(left, right)?)?;
        sin = Some(match sin {
            None => s,
            Some(a) => I::new(a.lo.min(s.lo), a.hi.max(s.hi))?,
        });
        cos = Some(match cos {
            None => c,
            Some(a) => I::new(a.lo.min(c.lo), a.hi.max(c.hi))?,
        });
        left = right;
    }
    Ok((sin.unwrap(), cos.unwrap()))
}

fn profile_radius<S: Scalar>(source: &Prepared, u: S) -> Result<S> {
    let (a, b) = if source.flank == Flank::Descending {
        (source.p.crest_radius, source.p.root_radius)
    } else {
        (source.p.root_radius, source.p.crest_radius)
    };
    S::c(iv(a)?).add(S::c(iv(b)?.sub(iv(a)?)?).mul(u)?)
}
/// Rotation-invariant physical-Y source: r*cos(beta)=sqrt(r^2-Y^2).
/// This is the same original contact equation on the principal near-side branch.
pub(super) fn axial_radial<S: Scalar>(source: &Prepared, u: S, y: S) -> Result<S> {
    let r = profile_radius(source, u)?;
    let projected = r.mul(r)?.sub(y.mul(y)?)?.sqrt()?;
    let k = if source.flank == Flank::Descending {
        neg(source.slope)?
    } else {
        source.slope
    };
    let denominator = S::c(k)
        .mul(projected.div(r)?)?
        .sub(S::c(source.h).mul(y)?.div(r.mul(r)?)?)?;
    check(
        denominator.value().lo > 0. || denominator.value().hi < 0.,
        "Physical axial flank contact denominator is not separated from zero",
    )?;
    let z = S::c(iv(source.p.pitch_radius)?)
        .sub(projected)?
        .div(denominator)?;
    let center = S::c(iv(source.p.pitch_radius)?).add(S::c(source.wheel_radius))?;
    let q = projected.sub(center)?;
    q.mul(q)?.add(z.mul(z)?)
}
pub(super) fn axial_flank<S: Scalar>(
    spec: Spec,
    flank: Flank,
    u: S,
    y: S,
) -> Result<Expression<S>> {
    let source = prepare(spec, flank)?;
    let beta = y.div(profile_radius(&source, u)?)?.asin()?;
    let mut result = evaluate(spec, flank, u, beta)?;
    result.point[1] = y;
    Ok(result)
}

pub(super) fn tip_blend<S: Scalar>(
    radius: f64,
    t: S,
    y: S,
    a: [S; 3],
    b: [S; 3],
) -> Result<[S; 3]> {
    let x = a[0].add(b[0].sub(a[0])?.mul(t)?)?;
    let z = a[2].add(b[2].sub(a[2])?.mul(t)?)?;
    // Original stock-root ownership proves |A|=|B|=R. This exact
    // identity removes derivatives of those constant norms; it is NOT a
    // normalization claim about arbitrary rounded/saved endpoint poles.
    let r2 = I::point(radius).mul(I::point(radius))?;
    let dot = a[0].mul(b[0])?.add(a[2].mul(b[2])?)?;
    let fraction = S::c(I::point(2.)).mul(t)?.mul(S::c(I::point(1.)).sub(t)?)?;
    // Original admission proves A.B>0 and 0<=t<=1. Convexity
    // gives R^2/2 <= |(1-t)A+tB|^2 <= R^2 on that source.
    let norm = S::c(r2)
        .add(fraction.mul(dot.sub(S::c(r2))?)?)?
        .restrict_value(I::new(r2.div(I::point(2.))?.lo, r2.hi)?)?
        .sqrt()?;
    let scale = S::c(I::point(radius)).div(norm)?;
    Ok([scale.mul(x)?, y, scale.mul(z)?])
}

#[cfg(test)]
mod inverse_value_tests {
    use super::*;
    #[test]
    fn principal_asin_bounds_cover_both_reductions_and_near_endpoint_arguments() {
        for x in [
            -1f64.next_down(),
            -0.9,
            -0.5000000000000001,
            -0.5,
            -0.2,
            0.,
            0.2,
            0.5,
            0.5000000000000001,
            0.9,
            1f64.next_down(),
        ] {
            let bound = asin_interval(I::point(x)).unwrap();
            let expected = x.asin();
            assert!(
                bound.lo <= expected && expected <= bound.hi,
                "{x}: {bound:?}, {expected}"
            );
        }
        let bound = asin_interval(I::new(-0.9, 0.8).unwrap()).unwrap();
        assert!(bound.lo <= (-0.9f64).asin() && 0.8f64.asin() <= bound.hi);
        for x in [-1., 1., 1.1] {
            assert!(asin_interval(I::point(x)).is_err());
        }
    }
}
