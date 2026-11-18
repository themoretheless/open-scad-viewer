//! A complete singular contact proof for definite forms in shifted powers.
//! Exact rational Bernstein coefficients, never a Hessian/sample tolerance test.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{One, ToPrimitive, Zero};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IsolatedGraphContactSpec {
    /// Normalized original parameters, strictly inside the common projection rectangle.
    pub center: [f64; 2],
    pub powers: [usize; 2],
    /// Same-sign nonzero coefficients of the second-minus-first graph gap.
    pub coefficients: [f64; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactIsolatedGraphContactSpec {
    /// Normalized U and V of the first original surface.
    pub center: [Q; 2],
    pub powers: [usize; 2],
    pub coefficients: [Q; 2],
}

#[derive(Clone, Debug)]
pub struct QualifiedIsolatedGraphContact {
    source: FiberSources,
    spec: ExactIsolatedGraphContactSpec,
    mixed: Q,
    second_uv: UvMap,
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
#[derive(Clone, Copy, Debug)]
struct UvMap {
    axes: [usize; 2],
    reversed: [bool; 2],
}

impl UvMap {
    const IDENTITY: Self = Self {
        axes: [0, 1],
        reversed: [false; 2],
    };

    fn degrees(self, s: &Surface) -> [usize; 2] {
        self.axes.map(|k| [s.degree_u, s.degree_v][k])
    }

    fn point(self, s: &Surface, aligned: [usize; 2]) -> &[f64] {
        let mut original = [0; 2];
        let degrees = self.degrees(s);
        for k in 0..2 {
            original[self.axes[k]] = if self.reversed[k] {
                degrees[k] - aligned[k]
            } else {
                aligned[k]
            };
        }
        &s.control_points[original[0]][original[1]]
    }

    fn native_normalized(self, center: &[Q; 2]) -> [Q; 2] {
        let mut native = [Q::zero(), Q::zero()];
        for k in 0..2 {
            native[self.axes[k]] = if self.reversed[k] {
                Q::one() - &center[k]
            } else {
                center[k].clone()
            };
        }
        native
    }
}

struct CommonGap {
    coefficients: Vec<Vec<Q>>,
    second_uv: UvMap,
}

/// Compare heights in a common exact Bernstein basis without changing either
/// original carrier or its natural parameter domain.
fn common_gap(source: &FiberSources) -> crate::Result<CommonGap> {
    // An affine view exists only when the homogeneous coordinates prove
    // pointwise equality on the original parameter domain.
    let [a, b] = source.evaluation_surfaces();
    admit(a)?;
    admit(b)?;
    let exact_point =
        |p: &[f64]| std::array::from_fn::<_, 3, _>(|axis| Q::from_float(p[axis]).unwrap());
    let origin = exact_point(&a.control_points[0][0]);
    let u = exact_point(&a.control_points[a.degree_u][0]);
    let v = exact_point(&a.control_points[0][a.degree_v]);
    let u = std::array::from_fn::<_, 3, _>(|k| &u[k] - &origin[k]);
    let v = std::array::from_fn::<_, 3, _>(|k| &v[k] - &origin[k]);
    let normal = std::array::from_fn::<_, 3, _>(|k| {
        let i = (k + 1) % 3;
        let j = (k + 2) % 3;
        &u[i] * &v[j] - &u[j] * &v[i]
    });
    // Each net must be the same affine base plus displacements parallel to
    // a single graph direction. A nonzero determinant proves injectivity
    // of projection along that direction on the common parameter rectangle.
    let find_projection = |map: UvMap| {
        let mut direction: Option<([Q; 3], usize)> = None;
        for (surface, map) in [(a, UvMap::IDENTITY), (b, map)] {
            let degree = map.degrees(surface);
            for i in 0..=degree[0] {
                for j in 0..=degree[1] {
                    let u_clock = Q::new(i.into(), degree[0].into());
                    let v_clock = Q::new(j.into(), degree[1].into());
                    let point = exact_point(map.point(surface, [i, j]));
                    let residual = std::array::from_fn(|k| {
                        &point[k] - &origin[k] - &u[k] * &u_clock - &v[k] * &v_clock
                    });
                    if let Some((axis, height)) = &direction {
                        if (0..3)
                            .any(|k| &residual[k] * &axis[*height] != &residual[*height] * &axis[k])
                        {
                            return None;
                        }
                    } else if let Some(height) = (0..3).find(|&k| !residual[k].is_zero()) {
                        let determinant: Q = normal.iter().zip(&residual).map(|(n, p)| n * p).sum();
                        if determinant.is_zero() {
                            return None;
                        }
                        direction = Some((residual, height));
                    }
                }
            }
        }
        direction.map(|(_, height)| (map, height))
    };
    let (second_uv, height) = [[0, 1], [1, 0]]
        .into_iter()
        .flat_map(|axes| {
            (0..4).map(move |mask| UvMap {
                axes,
                reversed: [mask & 1 != 0, mask & 2 != 0],
            })
        })
        .find_map(find_projection)
        .ok_or_else(|| crate::input("Missing common affine contact projection"))?;
    let [bu, bv] = second_uv.degrees(b);
    let degree = [a.degree_u.max(bu), a.degree_v.max(bv)];
    let elevate = |s: &Surface, map: UvMap| -> crate::Result<Vec<Vec<Q>>> {
        let old = map.degrees(s);
        let net = (0..=old[0])
            .map(|i| {
                (0..=old[1])
                    .map(|j| q(map.point(s, [i, j])[height]))
                    .collect::<crate::Result<Vec<_>>>()
            })
            .collect::<crate::Result<Vec<_>>>()?;
        let weights = |k: usize, i: usize| {
            let extra = degree[k] - old[k];
            (i.saturating_sub(extra)..=i.min(old[k])).map(move |j| {
                (
                    j,
                    Q::new(
                        (choose(old[k], j) * choose(extra, i - j)).into(),
                        choose(degree[k], i).into(),
                    ),
                )
            })
        };
        Ok((0..=degree[0])
            .map(|i| {
                (0..=degree[1])
                    .map(|j| {
                        weights(0, i).fold(Q::zero(), |sum, (u, wu)| {
                            sum + weights(1, j)
                                .fold(Q::zero(), |sum, (v, wv)| sum + &net[u][v] * &wu * wv)
                        })
                    })
                    .collect()
            })
            .collect())
    };
    let a = elevate(a, UvMap::IDENTITY)?;
    let b = elevate(b, second_uv)?;
    let coefficients = a
        .into_iter()
        .zip(b)
        .map(|(a, b)| a.into_iter().zip(b).map(|(a, b)| b - a).collect())
        .collect();
    Ok(CommonGap {
        coefficients,
        second_uv,
    })
}

/// Prove the entire original surface intersection is precisely one isolated
/// contact. The shared affine planar projection is globally injective; the exact
/// height gap is c0*(u-a)^even + c1*(v-b)^even. Same-sign terms vanish together
/// only at (a,b). All first gap derivatives vanish there, so this is a singular
/// intersection, not a regular branch. No trimmed-face or Boolean role follows.
pub fn qualify_isolated_graph_contact(
    source: &FiberSources,
    spec: IsolatedGraphContactSpec,
) -> crate::Result<QualifiedIsolatedGraphContact> {
    qualify_exact_isolated_graph_contact(
        source,
        ExactIsolatedGraphContactSpec {
            center: [q(spec.center[0])?, q(spec.center[1])?],
            powers: spec.powers,
            coefficients: [q(spec.coefficients[0])?, q(spec.coefficients[1])?],
        },
    )
}

pub fn qualify_exact_isolated_graph_contact(
    source: &FiberSources,
    spec: ExactIsolatedGraphContactSpec,
) -> crate::Result<QualifiedIsolatedGraphContact> {
    qualify_gap(source, spec, Q::zero(), &common_gap(source)?)
}

fn qualify_gap(
    source: &FiberSources,
    spec: ExactIsolatedGraphContactSpec,
    mixed: Q,
    gap: &CommonGap,
) -> crate::Result<QualifiedIsolatedGraphContact> {
    let second_uv = gap.second_uv;
    let gap = &gap.coefficients;
    let degree = [gap.len() - 1, gap[0].len() - 1];
    crate::check(
        spec.center.iter().all(|x| Q::zero() < *x && *x < Q::one())
            && spec
                .powers
                .iter()
                .all(|&n| (2..=8).contains(&n) && n % 2 == 0)
            && spec.powers[0] <= degree[0]
            && spec.powers[1] <= degree[1]
            && spec.coefficients.iter().all(|x| !x.is_zero())
            && (spec.coefficients[0] > Q::zero()) == (spec.coefficients[1] > Q::zero()),
        "Need interior even powers with nonzero same-sign gap coefficients",
    )?;
    let bu = bernstein(degree[0], spec.powers[0], spec.center[0].clone());
    let bv = bernstein(degree[1], spec.powers[1], spec.center[1].clone());
    let cross_u = bernstein(degree[0], spec.powers[0] / 2, spec.center[0].clone());
    let cross_v = bernstein(degree[1], spec.powers[1] / 2, spec.center[1].clone());
    let c = &spec.coefficients;
    crate::check(
        mixed.pow(2) < Q::from_integer(4.into()) * &c[0] * &c[1],
        "Mixed even-power gap form is not definite",
    )?;
    for (i, row) in gap.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            crate::check(
                *value == &c[0] * &bu[i] + &c[1] * &bv[j] + &mixed * &cross_u[i] * &cross_v[j],
                "Original whole-support even-power gap identity failed",
            )?;
        }
    }
    Ok(QualifiedIsolatedGraphContact {
        source: source.clone(),
        spec,
        mixed,
        second_uv,
    })
}
impl QualifiedIsolatedGraphContact {
    pub fn source(&self) -> &FiberSources {
        &self.source
    }
    /// Binary64 view for existing callers. Use the exact specification for replay.
    pub fn specification(&self) -> IsolatedGraphContactSpec {
        let approximate = |q: &Q| q.to_f64().expect("BigRational converts to binary64");
        IsolatedGraphContactSpec {
            center: self.spec.center.each_ref().map(approximate),
            powers: self.spec.powers,
            coefficients: self.spec.coefficients.each_ref().map(approximate),
        }
    }

    /// Center, exponents, and diagonal coefficients of the exact gap.
    pub fn exact_specification(&self) -> &ExactIsolatedGraphContactSpec {
        &self.spec
    }

    /// Coefficient of the product of the two half-power terms.
    pub fn mixed_coefficient(&self) -> &Q {
        &self.mixed
    }

    /// Original knot coordinates ordered as first U,V and second U,V.
    pub fn native_parameters(&self) -> [Q; 4] {
        native_parameters(&self.source, &self.spec.center, self.second_uv)
    }
    pub fn covers_entire_surfaces(&self) -> bool {
        true
    }
    pub fn reverify(&self) -> crate::Result<()> {
        qualify_gap(
            &self.source,
            self.spec.clone(),
            self.mixed.clone(),
            &common_gap(&self.source)?,
        )
        .map(|_| ())
    }
}

mod discovery;
pub use discovery::discover_isolated_graph_contact;

fn native_parameters(source: &FiberSources, center: &[Q; 2], second_uv: UvMap) -> [Q; 4] {
    let parameters = |s: &Surface, center: &[Q; 2]| {
        let axes = [
            (&s.knots_u, s.degree_u, s.control_points.len()),
            (&s.knots_v, s.degree_v, s.control_points[0].len()),
        ];
        std::array::from_fn::<_, 2, _>(|k| {
            let (knots, degree, count) = axes[k];
            let lo = Q::from_float(knots[degree]).unwrap();
            let hi = Q::from_float(knots[count]).unwrap();
            &lo + (hi - &lo) * &center[k]
        })
    };
    let [a, b] = parameters(source.first(), center);
    let [c, d] = parameters(source.second(), &second_uv.native_normalized(center));
    [a, b, c, d]
}

mod convex;
pub use convex::{QualifiedConvexGraphContact, discover_convex_graph_contact, qualify_convex_graph_contact};
