//! A complete singular contact proof for separable even-power graph gaps.
//! Exact rational Bernstein coefficients, never a Hessian/sample tolerance test.
use super::*;
use num_rational::BigRational as Q;
use num_traits::Zero;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IsolatedGraphContactSpec {
    /// Normalized original parameters, strictly inside the common XY rectangle.
    pub center: [f64; 2],
    pub powers: [usize; 2],
    /// Nonzero coefficients with the same sign in z_second - z_first.
    pub coefficients: [f64; 2],
}
#[derive(Clone, Debug)]
pub struct QualifiedIsolatedGraphContact {
    source: FiberSources,
    spec: IsolatedGraphContactSpec,
}
fn q(x: f64) -> crate::Result<Q> {
    Q::from_float(x).ok_or_else(|| crate::input("Nonfinite exact contact coefficient"))
}
fn choose(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    (0..k).fold(1, |a, j| a * (n - j) / (j + 1))
}
fn bernstein(degree: usize, power: usize, center: Q) -> Vec<Q> {
    (0..=degree)
        .map(|i| {
            (0..=i.min(power)).fold(Q::zero(), |sum, k| {
                sum + Q::from_integer(choose(power, k).into())
                    * (-&center).pow((power - k) as i32)
                    * Q::new(choose(i, k).into(), choose(degree, k).into())
            })
        })
        .collect()
}
fn admit(s: &Surface) -> crate::Result<()> {
    s.validate()?;
    crate::check(
        s.control_points.iter().flatten().all(|p| p.len() == 3),
        "Isolated contact requires spatial carriers",
    )?;
    crate::check(
        !s.periodic_u
            && !s.periodic_v
            && (1..=8).contains(&s.degree_u)
            && (1..=8).contains(&s.degree_v)
            && s.control_points.len() == s.degree_u + 1
            && s.control_points[0].len() == s.degree_v + 1,
        "Contact requires one degree1..8 polynomial Bezier graph",
    )?;
    for (d, k) in [(s.degree_u, &s.knots_u), (s.degree_v, &s.knots_v)] {
        crate::check(
            k.len() == 2 * (d + 1)
                && k[..=d].iter().all(|v| *v == k[d])
                && k[d + 1..].iter().all(|v| *v == k[d + 1]),
            "Contact requires full clamped original spans",
        )?;
    }
    let w = s.weights[0][0];
    crate::check(
        w > 0. && s.weights.iter().flatten().all(|&v| v == w),
        "Contact polynomial graph needs one positive constant weight",
    )
}
/// Prove the entire original surface intersection is precisely one isolated
/// contact. The shared affine XY projection is globally injective; the exact
/// height gap is c0*(u-a)^even + c1*(v-b)^even. Same-sign terms vanish together
/// only at (a,b). All first gap derivatives vanish there, so this is a singular
/// intersection, not a regular branch. No trimmed-face or Boolean role follows.
pub fn qualify_isolated_graph_contact(
    source: &FiberSources,
    spec: IsolatedGraphContactSpec,
) -> crate::Result<QualifiedIsolatedGraphContact> {
    let a = source.first();
    let b = source.second();
    admit(a)?;
    admit(b)?;
    crate::check(
        a.degree_u == b.degree_u && a.degree_v == b.degree_v,
        "Contact graph degrees must agree",
    )?;
    crate::check(
        spec.center
            .iter()
            .all(|x| x.is_finite() && 0. < *x && *x < 1.)
            && spec
                .powers
                .iter()
                .all(|&n| (2..=8).contains(&n) && n % 2 == 0)
            && spec.powers[0] <= a.degree_u
            && spec.powers[1] <= a.degree_v
            && spec.coefficients.iter().all(|x| x.is_finite() && *x != 0.)
            && spec.coefficients[0].is_sign_positive() == spec.coefficients[1].is_sign_positive(),
        "Need interior even powers with nonzero same-sign gap coefficients",
    )?;
    let origin = [q(a.control_points[0][0][0])?, q(a.control_points[0][0][1])?];
    let delta = [
        q(a.control_points[a.degree_u][0][0])? - &origin[0],
        q(a.control_points[0][a.degree_v][1])? - &origin[1],
    ];
    crate::check(
        delta.iter().all(|v| !v.is_zero()),
        "Degenerate contact XY projection",
    )?;
    let bu = bernstein(a.degree_u, spec.powers[0], q(spec.center[0])?);
    let bv = bernstein(a.degree_v, spec.powers[1], q(spec.center[1])?);
    let c = [q(spec.coefficients[0])?, q(spec.coefficients[1])?];
    for i in 0..=a.degree_u {
        for j in 0..=a.degree_v {
            let xy = [
                &origin[0] + &delta[0] * Q::new(i.into(), a.degree_u.into()),
                &origin[1] + &delta[1] * Q::new(j.into(), a.degree_v.into()),
            ];
            for s in [a, b] {
                for k in 0..2 {
                    crate::check(
                        q(s.control_points[i][j][k])? == xy[k],
                        "Original contact supports lack the identical affine XY frame",
                    )?;
                }
            }
            crate::check(
                q(b.control_points[i][j][2])? - q(a.control_points[i][j][2])?
                    == &c[0] * &bu[i] + &c[1] * &bv[j],
                "Original whole-support even-power gap identity failed",
            )?;
        }
    }
    Ok(QualifiedIsolatedGraphContact {
        source: source.clone(),
        spec,
    })
}
impl QualifiedIsolatedGraphContact {
    pub fn source(&self) -> &FiberSources {
        &self.source
    }
    pub fn specification(&self) -> IsolatedGraphContactSpec {
        self.spec
    }
    pub fn covers_entire_surfaces(&self) -> bool {
        true
    }
    pub fn reverify(&self) -> crate::Result<()> {
        qualify_isolated_graph_contact(&self.source, self.spec).map(|_| ())
    }
}

mod discovery;
pub use discovery::discover_isolated_graph_contact;
