//! Correlated whole-domain normal-radius bound for the exact graph family.
//! Cancellation is performed in exact coefficients before interval enclosure.
use super::*;
use crate::distance_bounds::Interval as I;
fn derivative(a: &[Q], span: &Q, b: &mut Budget) -> Result<Vec<Q>> {
    if a.len() == 1 {
        return Ok(vec![Q::zero()]);
    }
    a.windows(2)
        .map(|w| b.keep((&w[1] - &w[0]) * Q::from_integer((a.len() - 1).into()) / span))
        .collect()
}
fn add(a: &[Q], z: &[Q], sign: i64, b: &mut Budget) -> Result<Vec<Q>> {
    let n = (a.len() - 1).max(z.len() - 1);
    let a = elevate(a, n, b)?;
    let z = elevate(z, n, b)?;
    a.iter()
        .zip(z)
        .map(|(a, z)| b.keep(a + z * Q::from_integer(sign.into())))
        .collect()
}
fn hull(a: &[Q], b: &mut Budget) -> Result<I> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for x in a {
        let x = curvature::enclose(x, b)?;
        lo = lo.min(x.lo);
        hi = hi.max(x.hi);
    }
    I::new(lo, hi)
}
fn square(x: I) -> Result<I> {
    I::new(
        if x.lo <= 0. && x.hi >= 0. {
            0.
        } else {
            (x.lo * x.lo).min(x.hi * x.hi).next_down().max(0.)
        },
        (x.lo * x.lo).max(x.hi * x.hi).next_up(),
    )
}
pub(super) struct GraphCurvatureFactors {
    pub contact_radius_squared: Option<I>,
    pub source_curvature_squared: I,
    pub d_ratio: I,
    pub e: I,
    pub phi_second: I,
    pub normal_slopes: [I; 2],
    pub radial_distance_error_upper: f64,
    pub radial_normal_chord_error_upper: Option<f64>,
}
impl Patch {
    pub(super) fn graph_curvature_factors(
        &self,
        domain: [[f64; 2]; 2],
        require_contact_radius: bool,
        b: &mut Budget,
    ) -> Result<Option<GraphCurvatureFactors>> {
        let s = &self.sources[0];
        let n = s.degree_u;
        let wd = self.width.degree;
        // Parent scalar binomial arithmetic is bounded to degree <=32 here.
        if 2 * (n - 1 + wd) + 2 * wd > 32 {
            return Ok(None);
        }
        let source_low = b.authored(s.knots_u[n])?;
        let source_high = b.authored(s.knots_u[s.control_points.len()])?;
        let span = b.keep(&source_high - &source_low)?;
        let d = self.width.domain();
        let start = b.authored(d[0])?;
        let finish = b.authored(d[1])?;
        let width_span = b.keep(&finish - &start)?;
        let lo = b.keep((&start - &source_low) / &span)?;
        let hi = b.keep((&finish - &source_low) / &span)?;
        let original_q = s
            .control_points
            .iter()
            .map(|r| b.authored(r[0][2]))
            .collect::<Result<Vec<_>>>()?;
        let original_h = s
            .control_points
            .iter()
            .map(|r| {
                let q = b.authored(r[0][2])?;
                let z = b.authored(r[1][2])?;
                b.keep(q - z)
            })
            .collect::<Result<Vec<_>>>()?;
        let q = restrict(&original_q, &lo, &hi, b)?;
        let h = restrict(&original_h, &lo, &hi, b)?;
        let qp = restrict(&derivative(&original_q, &span, b)?, &lo, &hi, b)?;
        let hp = restrict(&derivative(&original_h, &span, b)?, &lo, &hi, b)?;
        let w = self
            .width
            .control_points
            .iter()
            .map(|p| b.authored(p[0]))
            .collect::<Result<Vec<_>>>()?;
        let wp = derivative(&w, &width_span, b)?;
        // Fresh complete tensor identity. A stale profile or changed interior
        // coefficient can never inherit this specialized graph certificate.
        let degree = self.coefficients.len() - 1;
        let m = self.profile.len() - 1;
        if self.coefficients.iter().any(|r| r.len() != m + 1) {
            return Ok(None);
        }
        let qe = elevate(&q, degree, b)?;
        let we = elevate(&w, degree, b)?;
        let hw = product(&h, &w, b)?;
        let hwe = elevate(&hw, degree, b)?;
        for (i, row) in self.coefficients.iter().enumerate() {
            let x = b.keep(&start + &width_span * ratio(i as u64, degree as u64))?;
            for (j, p) in row.iter().enumerate() {
                let y = b.keep(&we[i] * (ratio(2 * j as u64, m as u64) - Q::one()))?;
                let z = b.keep(&qe[i] - &hwe[i] * &self.profile[j])?;
                if p != &[x.clone(), y, z] {
                    return Ok(None);
                }
            }
        }
        let hpw = product(&hp, &w, b)?;
        let hwp = product(&h, &wp, b)?;
        let a0 = add(&qp, &hpw, -1, b)?;
        let h2 = product(&h, &h, b)?;
        let w2 = product(&w, &w, b)?;
        let a2 = product(&a0, &a0, b)?;
        let d0 = add(&add(&[Q::one()], &h2, 1, b)?, &a2, 1, b)?;
        let numerator = product(&w2, &d0, b)?;
        let denominator = elevate(&h2, numerator.len() - 1, b)?;
        if numerator.iter().any(|x| !x.is_positive()) {
            return Ok(None);
        }
        let source_ratios = denominator
            .iter()
            .zip(&numerator)
            .map(|(a, z)| b.keep(a / z))
            .collect::<Result<Vec<_>>>()?;
        let source_curvature_squared = hull(&source_ratios, b)?;
        let source_curvature_squared = I::new(
            source_curvature_squared.lo.max(0.),
            source_curvature_squared.hi,
        )?;
        let contact = if denominator.iter().all(|x| x.is_positive()) {
            let ratios = numerator
                .iter()
                .zip(&denominator)
                .map(|(a, z)| b.keep(a / z))
                .collect::<Result<Vec<_>>>()?;
            Some(hull(&ratios, b)?)
        } else if require_contact_radius {
            return Ok(None);
        } else {
            None
        };
        let urange = [b.authored(domain[0][0])?, b.authored(domain[0][1])?];
        // Positive denominator Bernstein coefficients make the rational curve
        // a convex combination of these coefficient ratios. Use its full
        // patch-U range; this remains conservative for any requested subset.
        let u_hull = |a: &[Q], b: &mut Budget| hull(&restrict(a, &urange[0], &urange[1], b)?, b);
        let h = u_hull(&h, b)?;
        let hpw = u_hull(&hpw, b)?;
        let hwp = u_hull(&hwp, b)?;
        let w = u_hull(&w, b)?;

        let a0 = u_hull(&a0, b)?;
        let pr = [b.authored(domain[1][0])?, b.authored(domain[1][1])?];
        let pv = derivative(&self.profile, &Q::one(), b)?;
        let pvv = derivative(&pv, &Q::one(), b)?;
        let half_pv = pv
            .iter()
            .map(|x| b.keep(x * ratio(1, 2)))
            .collect::<Result<Vec<_>>>()?;
        let transverse_residual = add(&[-Q::one(), Q::one()], &half_pv, -1, b)?;
        let transverse_residual = hull(&restrict(&transverse_residual, &pr[0], &pr[1], b)?, b)?;
        let phi = hull(&restrict(&self.profile, &pr[0], &pr[1], b)?, b)?;
        let pv = hull(&restrict(&pv, &pr[0], &pr[1], b)?, b)?;
        let pvv = hull(&restrict(&pvv, &pr[0], &pr[1], b)?, b)?;
        let x = I::new(domain[1][0], domain[1][1])?
            .mul(I::point(2.))?
            .sub(I::point(1.))?;
        let h2 = square(h)?;
        let pv2 = square(pv)?;
        let delta_a = hpw
            .mul(I::point(1.).sub(phi)?)?
            .add(hwp.mul(x.mul(pv)?.mul(I::point(0.5))?.sub(phi)?)?)?;
        let delta_d = delta_a
            .mul(a0.mul(I::point(2.))?.add(delta_a)?)?
            .add(h2.mul(pv2.mul(I::point(0.25))?.sub(I::point(1.))?)?)?;
        let base_d = I::point(1.).add(h2)?.add(square(a0)?)?;
        let base_d = I::new(base_d.lo.max(1.), base_d.hi)?;
        let d_ratio = I::point(1.).add(delta_d.div(base_d)?)?;
        let e = I::point(1.).add(h2.mul(pv2)?.mul(I::point(0.25))?)?;
        // Source-normal intersection center has homogeneous numerator
        // (u*h+a0*w, 0, q*h-h*h*w-w), denominator h. The following bound
        // eliminates h from the radial residual, so the flat limit stays finite.
        let residual = square(x)?
            .add(I::point(1.))?
            .sub(phi.mul(I::point(2.))?)?
            .add(h2.mul(square(I::point(1.).sub(phi)?)?)?)?;
        let radial = w.mul(h)?.mul(residual)?;
        let radial_distance_error_upper = radial.lo.abs().max(radial.hi.abs());
        check(
            radial_distance_error_upper.is_finite(),
            "Source radial-distance bound overflow",
        )?;
        let radial_z = I::point(1.).add(h2.mul(I::point(1.).sub(phi)?)?)?;
        let radial_normal_chord_error_upper = if radial_z.lo > 0. {
            let dx = delta_a.add(a0.mul(radial_z.sub(I::point(1.))?)?.div(radial_z)?)?;
            let dy = h
                .mul(
                    transverse_residual
                        .sub(pv.mul(I::point(0.5))?.mul(radial_z.sub(I::point(1.))?)?)?,
                )?
                .div(radial_z)?;
            let squared = square(dx)?.add(square(dy)?)?;
            let value = (squared.hi.max(0.).sqrt().next_up() * 2.).next_up();
            if value.is_finite() { Some(value) } else { None }
        } else {
            None
        };
        Ok(Some(GraphCurvatureFactors {
            contact_radius_squared: contact,
            source_curvature_squared,
            d_ratio,
            e,
            phi_second: pvv,
            normal_slopes: [
                I::point(0.).sub(a0.add(delta_a)?)?,
                h.mul(pv)?.mul(I::point(0.5))?,
            ],
            radial_distance_error_upper,
            radial_normal_chord_error_upper,
        }))
    }
    pub(super) fn correlated_normal_radius_within(
        &self,
        domain: [[f64; 2]; 2],
        low: I,
        high: I,
        b: &mut Budget,
    ) -> Result<bool> {
        let Some(f) = self.graph_curvature_factors(domain, true, b)? else {
            return Ok(false);
        };
        let Some(contact) = f.contact_radius_squared else {
            return Ok(false);
        };
        let curvature = square(f.phi_second)?;
        if curvature.lo <= 0. {
            return Ok(false);
        }
        let normal_squared = contact
            .mul(f.d_ratio)?
            .mul(square(f.e)?)?
            .mul(I::point(16.))?
            .div(curvature)?;
        Ok(square(high)?.sub(normal_squared)?.lo >= 0.
            && normal_squared.sub(square(low)?)?.lo >= 0.)
    }
}
