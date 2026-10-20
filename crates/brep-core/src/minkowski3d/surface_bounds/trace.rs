//! Piecewise original-chart curve identities. Irrational knot preimages are
//! isolated for the proof only; no algebraic seam vertex is rounded or fitted.
use super::*;
#[derive(Clone)]
pub(in crate::minkowski3d) enum Trace {
    Bezier(Vec<H>),
    Surface { surface: Surface, pcurve: Vec<H> },
}
fn value(p: &[H], t: &Q) -> H {
    let mut p = p.to_vec();
    while p.len() > 1 {
        p = p
            .windows(2)
            .map(|p| std::array::from_fn(|k| (Q::one() - t) * &p[0][k] + t * &p[1][k]))
            .collect()
    }
    p.pop().unwrap()
}
fn power(p: &[Q]) -> Vec<Q> {
    let n = p.len() - 1;
    let mut out = vec![Q::zero(); n + 1];
    for i in 0..=n {
        for j in i..=n {
            out[j] += &p[i]
                * choose(n, i)
                * choose(n - i, j - i)
                * integer(if (j - i) % 2 == 0 { 1 } else { -1 });
        }
    }
    out
}
pub(in crate::minkowski3d) fn curve_in_interval(
    p: &[H],
    axis: usize,
    lo: &Q,
    hi: &Q,
    work: &mut Work,
) -> Result<bool> {
    for coefficients in [
        p.iter().map(|p| &p[axis] - lo * &p[3]).collect::<Vec<_>>(),
        p.iter().map(|p| hi * &p[3] - &p[axis]).collect::<Vec<_>>(),
    ] {
        if coefficients.iter().all(|q| q >= &Q::zero()) {
            continue;
        }
        work.charge((p.len() + 1).pow(3))?;
        if !crate::profile_certificate::exact_polynomial_nonnegative(&power(&coefficients))? {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(in crate::minkowski3d) fn validate_uv(s: &Surface, p: &[H], work: &mut Work) -> Result<()> {
    for (axis, k, d, n) in [
        (0, &s.knots_u, s.degree_u, s.control_points.len()),
        (1, &s.knots_v, s.degree_v, s.control_points[0].len()),
    ] {
        if !curve_in_interval(
            p,
            axis,
            &Q::from_float(k[d]).unwrap(),
            &Q::from_float(k[n]).unwrap(),
            work,
        )? {
            return Err(fail(
                "original trim leaves the active NURBS parameter domain",
            ));
        }
    }
    Ok(())
}
impl Trace {
    pub(in crate::minkowski3d) fn reverse(&mut self) {
        match self {
            Self::Bezier(p) | Self::Surface { pcurve: p, .. } => p.reverse(),
        }
    }
    pub(in crate::minkowski3d) fn endpoints(&self) -> Result<[P; 2]> {
        match self {
            Self::Bezier(p) => Ok([point(&p[0]), point(p.last().unwrap())]),
            Self::Surface { surface, pcurve } => {
                let at = |p: &H| super::at(surface, &[&p[0] / &p[3], &p[1] / &p[3]]);
                Ok([at(&pcurve[0])?, at(pcurve.last().unwrap())?])
            }
        }
    }
    fn projections(&self) -> Vec<Vec<Q>> {
        match self {
            Self::Bezier(_) => Vec::new(),
            Self::Surface { surface, pcurve } => [(0, &surface.knots_u), (1, &surface.knots_v)]
                .iter()
                .flat_map(|(axis, knots)| {
                    knots.iter().map(|k| {
                        let k = Q::from_float(*k).unwrap();
                        power(
                            &pcurve
                                .iter()
                                .map(|p| &p[*axis] - &k * &p[3])
                                .collect::<Vec<_>>(),
                        )
                    })
                })
                .collect(),
        }
    }
    fn chart(&self, t: &Q, work: &mut Work) -> Result<Vec<H>> {
        match self {
            Self::Bezier(p) => Ok(p.clone()),
            Self::Surface { surface, pcurve } => {
                let p = value(pcurve, t);
                lift_chart(surface, pcurve, Some(&[&p[0] / &p[3], &p[1] / &p[3]]), work)
            }
        }
    }
}
pub(in crate::minkowski3d) fn same_trace(a: &Trace, b: &Trace, work: &mut Work) -> Result<bool> {
    if a.endpoints()? != b.endpoints()? {
        return Ok(false);
    }
    if let (Trace::Bezier(a), Trace::Bezier(b)) = (a, b) {
        work.charge(a.len() * b.len() * 6)?;
        return Ok(same(a, b));
    }
    let mut projections = a.projections();
    projections.extend(b.projections());
    for t in crate::profile_certificate::critical_samples(&projections)? {
        let a = a.chart(&t, work)?;
        let b = b.chart(&t, work)?;
        work.charge(a.len() * b.len() * 6)?;
        if !same(&a, &b) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Positive B-spline weights and knot multiplicity <= degree imply C0. For
/// full-multiplicity breaks, compare the complete one-sided seam functions.
pub(in crate::minkowski3d) fn continuous(s: &Surface, work: &mut Work) -> Result<()> {
    for (axis, knots, degree, count) in [
        (0, &s.knots_u, s.degree_u, s.control_points.len()),
        (1, &s.knots_v, s.degree_v, s.control_points[0].len()),
    ] {
        let mut values = knots[degree..=count].iter().copied().collect::<Vec<_>>();
        values.dedup();
        for d in values.windows(3) {
            let k = d[1];
            if knots.iter().filter(|x| **x == k).count() <= degree {
                continue;
            }
            let (other, od, oc) = if axis == 0 {
                (&s.knots_v, s.degree_v, s.control_points[0].len())
            } else {
                (&s.knots_u, s.degree_u, s.control_points.len())
            };
            for pair in other[od..=oc].windows(2).filter(|p| p[0] < p[1]) {
                work.charge(1)?;
                let aa = [Q::from_float(d[0]).unwrap(), Q::from_float(k).unwrap()];
                let bb = [Q::from_float(k).unwrap(), Q::from_float(d[2]).unwrap()];
                let other = [
                    Q::from_float(pair[0]).unwrap(),
                    Q::from_float(pair[1]).unwrap(),
                ];
                let (a, b) = if axis == 0 {
                    (cell(s, aa, other.clone())?, cell(s, bb, other)?)
                } else {
                    (cell(s, other.clone(), aa)?, cell(s, other, bb)?)
                };
                let (a, b) = if axis == 0 {
                    (a.last().unwrap().clone(), b[0].clone())
                } else {
                    (
                        a.iter().map(|r| r.last().unwrap().clone()).collect(),
                        b.iter().map(|r| r[0].clone()).collect(),
                    )
                };
                if !same(&a, &b) {
                    return Err(fail(
                        "original NURBS chart is discontinuous at a full-multiplicity knot",
                    ));
                }
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_nondyadic_tangency_does_not_require_its_control_hull_inside_the_domain() {
        let q = |x: f64| Q::from_float(x).unwrap();
        // u=3t-9t²/4 touches u=1 at t=2/3, despite its middle control u=1.5.
        let pc = vec![
            [q(0.), q(0.), q(0.), q(1.)],
            [q(1.5), q(0.5), q(0.), q(1.)],
            [q(0.75), q(1.), q(0.), q(1.)],
        ];
        assert!(curve_in_interval(&pc, 0, &q(0.), &q(1.), &mut Work::default()).unwrap());
        assert!(!curve_in_interval(&pc, 0, &q(0.), &q(0.999), &mut Work::default()).unwrap());
    }
    #[test]
    fn nonlinear_trace_crosses_an_irrational_surface_knot_without_rounding_the_cut() {
        let mut s = crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap().faces[0]
            .surface
            .clone();
        // Use a literal xy plane to make the independently authored trace clear.
        s.control_points = vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ];
        s = nurbs_core::editing::surface_edit::insert(&s, nurbs_core::surface::Axis::U, 0.5, 1)
            .unwrap();
        let q = |x: f64| Q::from_float(x).unwrap();
        let pc = vec![
            [q(0.), q(0.), q(0.), q(1.)],
            [q(0.), q(0.5), q(0.), q(1.)],
            [q(1.), q(1.), q(0.), q(1.)],
        ];
        let a = Trace::Surface {
            surface: s.clone(),
            pcurve: pc.clone(),
        };
        let b = Trace::Bezier(pc);
        continuous(&s, &mut Work::default()).unwrap();
        assert!(same_trace(&a, &b, &mut Work::default()).unwrap());
        s.control_points[0][0][2] = 0.125;
        assert!(
            !same_trace(
                &Trace::Surface {
                    surface: s,
                    pcurve: if let Trace::Surface { pcurve, .. } = a {
                        pcurve
                    } else {
                        unreachable!()
                    }
                },
                &b,
                &mut Work::default()
            )
            .unwrap()
        );
    }
    #[test]
    fn full_multiplicity_surface_jump_is_not_a_continuous_homotopy() {
        let mut s = crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap().faces[0]
            .surface
            .clone();
        s.knots_u = vec![0., 0., 0.5, 0.5, 1., 1.];
        s.control_points = vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]],
            vec![vec![0.5, 0., 1.], vec![0.5, 1., 1.]],
            vec![vec![1., 0., 1.], vec![1., 1., 1.]],
        ];
        s.weights = vec![vec![1.; 2]; 4];
        assert!(continuous(&s, &mut Work::default()).is_err());
    }
}
