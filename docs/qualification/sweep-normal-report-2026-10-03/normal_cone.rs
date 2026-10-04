//! Certified normal-cone enclosures, silhouette rejection, interval curvature
//! enclosures, global offset-smoothness certificates and draft-angle
//! enclosures (checklist 505-506, 511-512, 618-621).
//!
//! Method: the unit normal field of a regular NURBS patch is the normalized
//! cross product `S_u x S_v` of two rational vector fields. Naive interval
//! evaluation of the rational quotient explodes through variable dependency
//! (the denominator interval `W`, `W_u`, ... correlates with the numerator),
//! so per uv box we use a *mean-value / bootstrap* scheme that stays rigorous
//! while keeping widths `O(box size)` with small constants:
//!
//! 1. The jet at the box center comes from outward-rounded interval
//!    evaluation on a degenerate point box: rigorous and roundoff-tight.
//! 2. Homogeneous sums `X^(a,b) = sum_ij d^a N_i d^b M_j * Pw_ij` over the
//!    box are interval-evaluated once; they are plain polynomials in the
//!    basis, so their enclosures are moderate.
//! 3. From `X = S * W` and Leibniz, *scalar magnitude bounds* over the box
//!    are bootstrapped analytically, e.g.
//!    `|S_uu| <= (|X_uu| + 2|S_u||W_u| + |S||W_uu|) / W_lo` with
//!    `W_lo = min weight interval > 0`. No interval quotient is ever taken,
//!    which removes the dependency blow-up.
//! 4. Box enclosures of the derivative vectors are Taylor/mean-value forms
//!    `value(center) + sum_axes |D_axis| * h_axis` with the scalar bounds of
//!    step 3 as certified remainders. Every bound is inflated by outward
//!    rounding plus a relative `1e-9` safety factor covering the ~10
//!    elementary roundings of each scalar expression.
//!
//! Cone containment argument: the cross-product box is the tight center
//! cross plus a ball of certified radius `r`; the direction set of a ball
//! `B(c, r)` is the cone about `c/|c|` with half-angle `asin(r/|c|)` when
//! `|c| > r`. The patch cone is the union of the per-box cones, so every
//! exact surface normal lies inside the reported [`NormalCone`]. A box whose
//! weight interval touches zero or whose cross magnitude is not separated
//! from zero (poles, singular parametrizations) contributes the whole
//! sphere / an unbounded curvature range, never a silent omission.
//!
//! Relation to `surface_offset::offset_validity`: that report certifies a
//! fold-free bound from a *sampled* curvature grid (rounded-down sample
//! minimum of the principal radius). [`offset_globally_smooth`] is the
//! patch-wide interval counterpart: the `Valid` branch is a proof over the
//! whole parameter domain, not a sample maximum, at the price of interval
//! overestimation. The two certificates are complementary and intentionally
//! share no code.
use crate::{
    Result, check, numeric_err,
    curve_differential::Side,
    interval_eval::Interval,
    surface::Surface,
};
use math_core::{cross, dot, norm};

/// Number of interval subdivisions per nonempty knot span per axis for the
/// normal cone.
const CONE_SUBDIV: usize = 24;
/// Finer subdivision for curvature enclosures (second derivatives widen more).
const CURVATURE_SUBDIV: usize = 128;
/// Relative safety factor on every scalar bound, covering the handful of
/// elementary roundings inside each bootstrap expression.
const SAFETY: f64 = 1. + 1e-9;

/// Round-up helper: `x` inflated by one ulp and the safety factor.
fn up(x: f64) -> f64 {
    (x * SAFETY).next_up()
}

/// Outward-rounded point interval for an already rounded f64 product.
fn c_i(x: f64) -> Result<Interval> {
    Interval::new(x.next_down(), x.next_up())
}

/// Robust angle between two nonzero vectors via atan2(|a x b|, a . b).
fn angle_between(a: [f64; 3], b: [f64; 3]) -> f64 {
    norm(cross(a, b)).atan2(dot(a, b))
}

/// Interval Cox-de Boor triangle: `levels[j][i]` encloses `N_{i,j}` over `u`.
/// Degree zero is `[1,1]` only when the whole box lies inside the span
/// closure; a span merely touching the box edge takes value `{0,1}` over the
/// box, so it is marked `[0,1]` — marking it `[1,1]` would be unsound at the
/// interior points. Every operation rounds outward: each entry provably
/// contains the exact basis value for every parameter in `u`.
fn basis_levels(p: usize, knots: &[f64], u: Interval) -> Result<Vec<Vec<Interval>>> {
    let mut levels = Vec::with_capacity(p + 1);
    let mut row = Vec::with_capacity(knots.len() - 1);
    for i in 0..knots.len() - 1 {
        let nonempty = knots[i] < knots[i + 1];
        let contains = knots[i] <= u.lo && u.hi <= knots[i + 1];
        let touches = knots[i] <= u.hi && u.lo <= knots[i + 1];
        let v = if nonempty && contains {
            Interval::point(1.)
        } else if nonempty && touches {
            Interval::new(0., 1.)?
        } else {
            Interval::point(0.)
        };
        row.push(v);
    }
    levels.push(row);
    for j in 1..=p {
        let n = knots.len() - 1 - j;
        let prev = levels.last().unwrap();
        let mut row = Vec::with_capacity(n);
        for i in 0..n {
            let mut acc = Interval::point(0.);
            let d0 = knots[i + j] - knots[i];
            if d0 > 0. {
                let a = u.sub(Interval::point(knots[i]))?.div(Interval::point(d0))?;
                acc = acc.add(a.mul(prev[i])?)?;
            }
            let d1 = knots[i + j + 1] - knots[i + 1];
            if d1 > 0. {
                let b = Interval::point(knots[i + j + 1]).sub(u)?.div(Interval::point(d1))?;
                acc = acc.add(b.mul(prev[i + 1])?)?;
            }
            row.push(acc);
        }
        levels.push(row);
    }
    Ok(levels)
}

/// Enclosure of the `k`-th derivative of the degree-`p` basis over the box
/// behind `levels`, via
/// `d^k N_{i,p} = p/(t_{i+p}-t_i) d^{k-1} N_{i,p-1} - p/(t_{i+p+1}-t_{i+1}) d^{k-1} N_{i+1,p-1}`.
/// Orders above the degree are exactly zero.
fn basis_derivative(p: usize, knots: &[f64], levels: &[Vec<Interval>], k: usize) -> Result<Vec<Interval>> {
    let n = knots.len() - 1 - p;
    if k == 0 {
        return Ok(levels[p].clone());
    }
    if k > p {
        return Ok(vec![Interval::point(0.); n]);
    }
    let prev = basis_derivative(p - 1, knots, levels, k - 1)?;
    let pf = p as f64;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let mut acc = Interval::point(0.);
        let d0 = knots[i + p] - knots[i];
        if d0 > 0. {
            acc = acc.add(prev[i].mul(c_i(pf / d0)?)?)?;
        }
        let d1 = knots[i + p + 1] - knots[i + 1];
        if d1 > 0. {
            acc = acc.sub(prev[i + 1].mul(c_i(pf / d1)?)?)?;
        }
        out.push(acc);
    }
    Ok(out)
}

const ZERO: Interval = Interval { lo: 0., hi: 0. };

/// Homogeneous tensor sums `h[a][b] = sum_ij d^a N_i d^b M_j * (Pw_ij, w_ij)`
/// interval-evaluated over the box; `a + b <= 3`, last component is weight.
///
/// Dependency control (this is what keeps the bounds usable): the inner sums
/// `c_i = sum_j M_j * Pw_ij` are convex combinations (B-spline basis values
/// are nonnegative and sum to one), so each `c_i` is intersected with the
/// per-row control hull, and the value sum `h[0][0]` with the global hull.
/// Both intersections are rigorous by the convex-hull property of B-splines
/// and cut the interval-squaring that a plain tensor evaluation produces.
fn homogeneous_sums(surface: &Surface, u: Interval, v: Interval) -> Result<[[[Interval; 4]; 4]; 4]> {
    let levels_u = basis_levels(surface.degree_u, &surface.knots_u, u)?;
    let levels_v = basis_levels(surface.degree_v, &surface.knots_v, v)?;
    let ders_u: Vec<Vec<Interval>> = (0..=3)
        .map(|k| basis_derivative(surface.degree_u, &surface.knots_u, &levels_u, k))
        .collect::<Result<_>>()?;
    let ders_v: Vec<Vec<Interval>> = (0..=3)
        .map(|k| basis_derivative(surface.degree_v, &surface.knots_v, &levels_v, k))
        .collect::<Result<_>>()?;
    let nu = surface.control_points.len();
    let nv = surface.control_points[0].len();
    // Homogeneous control points and their per-row / global hulls.
    let pw: Vec<Vec<[f64; 4]>> = (0..nu)
        .map(|i| {
            (0..nv)
                .map(|j| {
                    let w = surface.weights[i][j];
                    let p = &surface.control_points[i][j];
                    [w * p[0], w * p[1], w * p[2], w]
                })
                .collect()
        })
        .collect();
    let hull = |points: &[[f64; 4]]| -> [Interval; 4] {
        std::array::from_fn(|k| {
            let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
            for p in points {
                lo = lo.min(p[k]);
                hi = hi.max(p[k]);
            }
            // Hull bounds of exact finite values; widen one ulp outward.
            Interval { lo: lo.next_down(), hi: hi.next_up() }
        })
    };
    let row_hull: Vec<[Interval; 4]> = pw.iter().map(|row| hull(row)).collect();
    let global_hull = hull(&pw.concat());
    let mut h = [[[ZERO; 4]; 4]; 4];
    for (a, du) in ders_u.iter().enumerate() {
        for (b, dv) in ders_v.iter().enumerate() {
            if a + b > 3 {
                continue;
            }
            // Inner sums along v with the row-hull intersection (only for
            // pure value sums in v, where convexity holds).
            let mut inner: Vec<[Interval; 4]> = Vec::with_capacity(nu);
            for (i, row) in row_hull.iter().enumerate().take(nu) {
                let mut acc = [ZERO; 4];
                for (j, &mj) in dv.iter().enumerate() {
                    if mj.lo == 0. && mj.hi == 0. {
                        continue;
                    }
                    for k in 0..4 {
                        acc[k] = acc[k].add(mj.mul(c_i(pw[i][j][k])?)?)?;
                    }
                }
                if b == 0 {
                    for k in 0..4 {
                        let lo = acc[k].lo.max(row[k].lo);
                        let hi = acc[k].hi.min(row[k].hi);
                        if !(lo <= hi) {
                            eprintln!("HULL MISS row {i} k{k}: acc=[{},{}] hull=[{},{}]", acc[k].lo, acc[k].hi, row[k].lo, row[k].hi);
                        }
                        acc[k] = Interval::new(lo, hi)?;
                    }
                }
                inner.push(acc);
            }
            let mut acc = [ZERO; 4];
            for (i, &ni) in du.iter().enumerate() {
                if ni.lo == 0. && ni.hi == 0. {
                    continue;
                }
                for k in 0..4 {
                    acc[k] = acc[k].add(ni.mul(inner[i][k])?)?;
                }
            }
            if a == 0 && b == 0 {
                for k in 0..4 {
                    acc[k] = Interval::new(acc[k].lo.max(global_hull[k].lo), acc[k].hi.min(global_hull[k].hi))?;
                }
            }
            h[a][b] = acc;
        }
    }
    Ok(h)
}

/// Rational surface jet from the homogeneous sums by the Leibniz recurrence
/// `S^(n) = (X^(n) - sum_{k<n} binom(n,k) S^(k) W^(n-k)) / W`, all interval.
/// Rigorous for narrow boxes and exact-order at a point box. `Ok(None)` when
/// the weight interval touches zero.
fn rational_jet(h: &[[[Interval; 4]; 4]; 4]) -> Result<Option<[[[Interval; 3]; 4]; 4]>> {
    let w00 = h[0][0][3];
    if w00.lo <= 0. {
        return Ok(None);
    }
    let binom = |n: usize, k: usize| -> f64 {
        let mut r = 1.;
        for i in 0..k {
            r = r * (n - i) as f64 / (i + 1) as f64;
        }
        r
    };
    let mut s = [[[ZERO; 3]; 4]; 4];
    for order in 0..=3 {
        for a in 0..=order {
            let b = order - a;
            let mut num = h[a][b];
            for i in 0..=a {
                for j in 0..=b {
                    if i == a && j == b {
                        continue;
                    }
                    let wder = h[a - i][b - j][3];
                    if wder.lo == 0. && wder.hi == 0. {
                        continue;
                    }
                    let cw = c_i(binom(a, i) * binom(b, j))?.mul(wder)?;
                    for k in 0..3 {
                        num[k] = num[k].sub(s[i][j][k].mul(cw)?)?;
                    }
                }
            }
            for k in 0..3 {
                s[a][b][k] = num[k].div(w00)?;
            }
        }
    }
    Ok(Some(s))
}

/// Certified per-box data: tight center jet (mid + half-width) and scalar
/// magnitude bounds of every derivative up to order three over the box.
struct BoxBounds {
    /// Center jet midpoints `c[a][b]` and half-width norms `e[a][b]`.
    c: [[[f64; 3]; 3]; 3],
    e: [[f64; 3]; 3],
    /// Certified `max |S^(a,b)|` over the box, order 1..=3.
    b: [[f64; 4]; 4],
    /// Certified lower bound of the weight over the box (> 0).
    w_lo: f64,
}

/// Magnitude of an interval (max absolute end).
fn mag(x: Interval) -> f64 {
    x.lo.abs().max(x.hi.abs())
}

/// Euclidean magnitude bound of an interval vector.
fn mag3(x: &[Interval; 3]) -> f64 {
    up(x.iter().map(|&c| mag(c) * mag(c)).sum::<f64>().sqrt())
}

fn box_bounds(surface: &Surface, u: Interval, v: Interval) -> Result<Option<BoxBounds>> {
    let h = homogeneous_sums(surface, u, v)?;
    let w_lo = h[0][0][3].lo;
    if w_lo <= 0. {
        return Ok(None);
    }
    // Scalar weight-derivative magnitudes |W^(a,b)|.
    let bw = |a: usize, b: usize| up(mag(h[a][b][3]));
    // Vector magnitudes |X^(a,b)| of the homogeneous weighted position.
    let bx = |a: usize, b: usize| mag3(&[h[a][b][0], h[a][b][1], h[a][b][2]]);
    // Bootstrap |S^(a,b)| from X = S * W (Leibniz), all outward rounded.
    let mut b = [[0.; 4]; 4];
    b[0][0] = up(bx(0, 0) / w_lo);
    b[1][0] = up((bx(1, 0) + b[0][0] * bw(1, 0)) / w_lo);
    b[0][1] = up((bx(0, 1) + b[0][0] * bw(0, 1)) / w_lo);
    b[2][0] = up((bx(2, 0) + 2. * b[1][0] * bw(1, 0) + b[0][0] * bw(2, 0)) / w_lo);
    b[1][1] = up((bx(1, 1) + b[1][0] * bw(0, 1) + b[0][1] * bw(1, 0) + b[0][0] * bw(1, 1)) / w_lo);
    b[0][2] = up((bx(0, 2) + 2. * b[0][1] * bw(0, 1) + b[0][0] * bw(0, 2)) / w_lo);
    b[3][0] = up((bx(3, 0) + 3. * b[2][0] * bw(1, 0) + 3. * b[1][0] * bw(2, 0) + b[0][0] * bw(3, 0)) / w_lo);
    b[2][1] = up((bx(2, 1)
        + b[2][0] * bw(0, 1)
        + 2. * (b[1][1] * bw(1, 0) + b[1][0] * bw(1, 1))
        + b[0][1] * bw(2, 0)
        + b[0][0] * bw(2, 1))
        / w_lo);
    b[1][2] = up((bx(1, 2)
        + b[1][0] * bw(0, 2)
        + 2. * (b[1][1] * bw(0, 1) + b[0][1] * bw(1, 1))
        + b[0][2] * bw(1, 0)
        + b[0][0] * bw(1, 2))
        / w_lo);
    b[0][3] = up((bx(0, 3) + 3. * b[0][2] * bw(0, 1) + 3. * b[0][1] * bw(0, 2) + b[0][0] * bw(0, 3)) / w_lo);
    // Tight center jet: degenerate point boxes are roundoff-narrow.
    let uc = Interval::point(u.lo / 2. + u.hi / 2.);
    let vc = Interval::point(v.lo / 2. + v.hi / 2.);
    let hc = homogeneous_sums(surface, uc, vc)?;
    let Some(sc) = rational_jet(&hc)? else {
        return Ok(None);
    };
    let mut c = [[[0.; 3]; 3]; 3];
    let mut e = [[0.; 3]; 3];
    for a in 0..3 {
        for bb in 0..3 - a {
            let mid: [f64; 3] = std::array::from_fn(|k| sc[a][bb][k].lo / 2. + sc[a][bb][k].hi / 2.);
            c[a][bb] = mid;
            e[a][bb] = up(norm(std::array::from_fn(|k| {
                sc[a][bb][k].hi / 2. - sc[a][bb][k].lo / 2.
            })));
        }
    }
    Ok(Some(BoxBounds { c, e, b, w_lo }))
}

/// Nonempty knot-span sub-boxes of the surface domain, `subdiv` per span.
fn subdivision(surface: &Surface, subdiv: usize) -> Vec<[Interval; 2]> {
    let axes = [
        (
            surface.degree_u,
            &surface.knots_u,
            surface.control_points.len(),
        ),
        (
            surface.degree_v,
            &surface.knots_v,
            surface.control_points[0].len(),
        ),
    ];
    let mut pieces: [Vec<Interval>; 2] = [Vec::new(), Vec::new()];
    for (axis, (p, k, n)) in axes.iter().enumerate() {
        for i in *p..*n {
            let (a, b) = (k[i], k[i + 1]);
            if b <= a {
                continue;
            }
            for s in 0..subdiv {
                let lo = a + (b - a) * s as f64 / subdiv as f64;
                let hi = a + (b - a) * (s + 1) as f64 / subdiv as f64;
                if let Ok(t) = Interval::new(lo, hi) {
                    pieces[axis].push(t);
                }
            }
        }
    }
    let mut out = Vec::with_capacity(pieces[0].len() * pieces[1].len());
    for &u in &pieces[0] {
        for &v in &pieces[1] {
            out.push([u, v]);
        }
    }
    out
}

/// Conservative enclosure of the normal cone of a surface patch: every exact
/// unit normal `n(u, v)` satisfies `angle(n, axis) <= half_angle`.
#[derive(Clone, Copy, Debug)]
pub struct NormalCone {
    pub axis: [f64; 3],
    pub half_angle: f64,
}

impl NormalCone {
    /// Whether unit direction `dir` lies inside the cone.
    pub fn contains(&self, dir: [f64; 3]) -> bool {
        let n = norm(dir);
        n > 0. && angle_between(self.axis, dir.map(|x| x / n)) <= self.half_angle
    }

    /// Silhouette-rejection test (checklist 506): returns `Ok(true)` when no
    /// normal inside the cone can be perpendicular to `view_dir`, hence the
    /// patch provably has no silhouette from that view. Both-sided: a cone
    /// entirely in front of or behind the view plane rejects the silhouette.
    pub fn silhouette_impossible(&self, view_dir: [f64; 3]) -> Result<bool> {
        check(
            view_dir.iter().all(|x| x.is_finite()) && norm(view_dir) > 0.,
            "View direction must be finite and nonzero",
        )?;
        if self.half_angle >= std::f64::consts::FRAC_PI_2 {
            return Ok(false);
        }
        let theta = angle_between(self.axis, view_dir);
        Ok(theta + self.half_angle < std::f64::consts::FRAC_PI_2
            || theta - self.half_angle > std::f64::consts::FRAC_PI_2)
    }
}

/// Per-box enclosure of the cross product `S_u x S_v`: tight midpoint plus a
/// certified ball radius (mean-value form with the bootstrapped bounds).
fn cross_ball(bb: &BoxBounds, hu: f64, hv: f64) -> ([f64; 3], f64) {
    let su = bb.c[1][0];
    let sv = bb.c[0][1];
    let mid = cross(su, sv);
    // |S_u| <= min(|su_c| + r_su, b_su) etc.: both bounds are certified.
    let r_su = up(hu * bb.b[2][0] + hv * bb.b[1][1] + bb.e[1][0]);
    let r_sv = up(hu * bb.b[1][1] + hv * bb.b[0][2] + bb.e[0][1]);
    let b_su = bb.b[1][0].min(up(norm(su) + r_su));
    let b_sv = bb.b[0][1].min(up(norm(sv) + r_sv));
    // |d/du (S_u x S_v)| <= |S_uu||S_v| + |S_u||S_uv|; same for v.
    let r = up(hu * (bb.b[2][0] * b_sv + b_su * bb.b[1][1])
        + hv * (bb.b[1][1] * b_sv + b_su * bb.b[0][2]));
    // Center rounding: cross of two midpoints, inflated generously.
    let r_center = up(norm(mid) * 1e-12 + norm(su) * bb.e[0][1] + norm(sv) * bb.e[1][0]);
    (mid, up(r + r_center))
}

/// Certified normal-cone enclosure of the whole patch (checklist 505).
/// See the module docs for the containment proof. A singular box (weight or
/// cross-product magnitude not separated from zero, e.g. a pole) widens the
/// cone to the full sphere; a symmetric normal field with a vanishing axis
/// sum does the same, so the result is always conservative.
pub fn normal_cone(surface: &Surface) -> Result<NormalCone> {
    surface.validate()?;
    check(
        surface
            .weights
            .iter()
            .flatten()
            .all(|&w| w > 0. && w.is_finite()),
        "Normal cones require strictly positive weights",
    )?;
    let boxes = subdivision(surface, CONE_SUBDIV);
    check(!boxes.is_empty(), "Surface has no nonempty knot cells")?;
    let mut axis_sum = [0.; 3];
    let mut per_box: Vec<([f64; 3], f64)> = Vec::with_capacity(boxes.len());
    for [u, v] in boxes {
        let Some(bb) = box_bounds(surface, u, v)? else {
            return Ok(full_sphere());
        };
        let (mid, r) = cross_ball(&bb, (u.hi - u.lo) / 2., (v.hi - v.lo) / 2.);
        let mc = norm(mid);
        if !(mc > r) {
            // The cross ball touches the origin: every direction is possible.
            return Ok(full_sphere());
        }
        let dir = mid.map(|x| x / mc);
        // Direction cone of the enclosing ball B(mid, r), outward rounded.
        let alpha = up((r / mc).min(1.).asin());
        axis_sum = std::array::from_fn(|k| axis_sum[k] + dir[k]);
        per_box.push((dir, alpha));
    }
    let la = norm(axis_sum);
    if la <= 1e-14 {
        return Ok(full_sphere());
    }
    let axis = axis_sum.map(|x| x / la);
    let mut half = 0_f64;
    for (dir, alpha) in per_box {
        half = half.max(angle_between(axis, dir) + alpha);
    }
    Ok(NormalCone {
        axis,
        half_angle: up(half).min(std::f64::consts::PI),
    })
}

fn full_sphere() -> NormalCone {
    NormalCone {
        axis: [0., 0., 1.],
        half_angle: std::f64::consts::PI,
    }
}

/// Patch-wide interval curvature enclosure (checklist 511).
#[derive(Clone, Copy, Debug)]
pub struct CurvatureEnclosure {
    /// Enclosure of the Gaussian curvature K over the whole patch.
    pub gaussian: Interval,
    /// Enclosure of the signed mean curvature H (orientation `S_u x S_v`).
    pub mean: Interval,
    /// Certified upper bound of `max(|k1|, |k2|)` over the patch; `INFINITY`
    /// when some box was singular and no finite bound was proven.
    pub max_abs_principal: f64,
    /// False when at least one sub-box was singular; the interval fields are
    /// then widened to `[-1e308, 1e308]` and carry no information.
    pub regular: bool,
}

/// Interval enclosure of the first and second fundamental forms contracted
/// into K and H over the entire patch. Per box the forms are enclosed by the
/// mean-value forms of [`cross_ball`] and the bootstrapped scalar bounds;
/// the contractions `K = (LN - M^2) / (EG - F^2)`,
/// `H = (EN + GL - 2FM) / (2(EG - F^2))` are then outward-rounded interval
/// arithmetic on rigorous component intervals.
pub fn curvature_enclosure(surface: &Surface) -> Result<CurvatureEnclosure> {
    surface.validate()?;
    check(
        surface
            .weights
            .iter()
            .flatten()
            .all(|&w| w > 0. && w.is_finite()),
        "Curvature enclosures require strictly positive weights",
    )?;
    let wide = Interval::new(-1e308, 1e308)?;
    let mut gaussian: Option<Interval> = None;
    let mut mean: Option<Interval> = None;
    let mut max_abs = 0_f64;
    let mut regular = true;
    for [u, v] in subdivision(surface, CURVATURE_SUBDIV) {
        let Some(bb) = box_bounds(surface, u, v)? else {
            regular = false;
            break;
        };
        let (hu, hv) = ((u.hi - u.lo) / 2., (v.hi - v.lo) / 2.);
        let (mid, r) = cross_ball(&bb, hu, hv);
        let (mc, cl, ch) = (norm(mid), norm(mid) - r, norm(mid) + r);
        if cl <= 0. {
            regular = false;
            break;
        }
        let det = Interval::new((cl * cl).next_down(), (ch * ch).next_up())?;
        // Radii of the first and second derivative vectors over the box.
        let r_su = up(hu * bb.b[2][0] + hv * bb.b[1][1] + bb.e[1][0]);
        let r_sv = up(hu * bb.b[1][1] + hv * bb.b[0][2] + bb.e[0][1]);
        let r_suu = up(hu * bb.b[3][0] + hv * bb.b[2][1] + bb.e[2][0]);
        let r_suv = up(hu * bb.b[2][1] + hv * bb.b[1][2] + bb.e[1][1]);
        let r_svv = up(hu * bb.b[1][2] + hv * bb.b[0][3] + bb.e[0][2]);
        let b_su = bb.b[1][0].min(up(norm(bb.c[1][0]) + r_su));
        let b_sv = bb.b[0][1].min(up(norm(bb.c[0][1]) + r_sv));
        let b_suu = bb.b[2][0].min(up(norm(bb.c[2][0]) + r_suu));
        let b_suv = bb.b[1][1].min(up(norm(bb.c[1][1]) + r_suv));
        let b_svv = bb.b[0][2].min(up(norm(bb.c[0][2]) + r_svv));
        // First form intervals: E = |S_u|^2, F = <S_u, S_v>, G = |S_v|^2.
        let ball = |c: [f64; 3], r_c: f64| -> Result<Interval> {
            Interval::new(
                (norm(c) - r_c).max(0.).powi(2).next_down(),
                up(norm(c) + r_c).powi(2).next_up(),
            )
        };
        let e = ball(bb.c[1][0], r_su)?;
        let g = ball(bb.c[0][1], r_sv)?;
        let fc = dot(bb.c[1][0], bb.c[0][1]);
        let df = up(r_su * norm(bb.c[0][1]) + r_sv * norm(bb.c[1][0]) + r_su * r_sv + fc.abs() * 1e-12);
        let f = Interval::new((fc - df).next_down(), (fc + df).next_up())?;
        // Second form numerators N_L = <S_u x S_v, S_uu> etc.: center value
        // plus variation radius, then division by |cross| in [cl, ch].
        let second = |c2: [f64; 3], b_c2: f64, r_c2: f64| -> Result<Interval> {
            let nc = dot(mid, c2);
            let dn = up(r * b_c2 + ch * r_c2 + r * r_c2 + nc.abs() * 1e-12);
            let num = Interval::new((nc - dn).next_down(), (nc + dn).next_up())?;
            num.div(Interval::new(cl.next_down(), ch.next_up())?)
        };
        let l = second(bb.c[2][0], b_suu, r_suu)?;
        let m = second(bb.c[1][1], b_suv, r_suv)?;
        let n = second(bb.c[0][2], b_svv, r_svv)?;
        let k_gauss = l.mul(n)?.sub(m.mul(m)?)?.div(det)?;
        let h_mean = e
            .mul(n)?
            .add(g.mul(l)?)?
            .sub(f.mul(m)?.mul(Interval::point(2.))?)?
            .div(det.mul(Interval::point(2.))?)?;
        gaussian = Some(match gaussian {
            None => k_gauss,
            Some(acc) => union(acc, k_gauss)?,
        });
        mean = Some(match mean {
            None => h_mean,
            Some(acc) => union(acc, h_mean)?,
        });
        // |k| <= |H| + sqrt(max(0, H^2 - K)), outward rounded.
        let h_abs = mag(h_mean);
        let disc = h_mean.mul(h_mean)?.sub(k_gauss)?;
        let root = disc.hi.max(0.).sqrt().next_up();
        max_abs = max_abs.max(up(h_abs + root));
    }
    if !regular {
        return Ok(CurvatureEnclosure {
            gaussian: wide,
            mean: wide,
            max_abs_principal: f64::INFINITY,
            regular: false,
        });
    }
    // An exactly constant authored coordinate places the entire rational
    // patch in a plane. Regularity above is still required; no tolerance
    // or sampled coplanarity can promote this zero-curvature identity.
    let origin = &surface.control_points[0][0];
    if (0..3).any(|axis| surface.control_points.iter().flatten()
        .all(|point| point[axis] == origin[axis])) {
        return Ok(CurvatureEnclosure {
            gaussian: Interval::point(0.), mean: Interval::point(0.),
            max_abs_principal: 0., regular: true,
        });
    }
    Ok(CurvatureEnclosure {
        gaussian: gaussian.ok_or_else(|| numeric_err("Surface has no nonempty knot cells"))?,
        mean: mean.unwrap_or(wide),
        max_abs_principal: max_abs.next_up(),
        regular: true,
    })
}

/// Interval union.
fn union(a: Interval, b: Interval) -> Result<Interval> {
    Interval::new(a.lo.min(b.lo), a.hi.max(b.hi))
}

/// Global offset-smoothness certificate (checklist 512).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OffsetCertificate {
    /// Proven: `|d| * max|k| < 1` over the whole patch; `margin = 1 - |d|*k_max`
    /// rounded down. The offset `S + d*n` has no focal crossing anywhere.
    Valid { margin: f64 },
    /// Proven: a sampled point carries `|d| * k >= 1`; `max_d_kappa` is the
    /// certified lower bound of `|d| * max|k|` that crossed unity.
    Invalid { max_d_kappa: f64 },
    /// Interval bounds were too loose (or the patch is singular somewhere)
    /// to decide either side.
    Unknown,
}

/// Patch-wide proof attempt for `|d| * k_max < 1`. The `Valid` side uses the
/// rigorous [`curvature_enclosure`] upper bound; the `Invalid` side samples
/// certified pointwise principal-curvature intervals
/// (`surface_differential::at`) and fires only when a *lower* bound of
/// `|d| * max|k|` provably reaches one. Complements the sampled-radius
/// certificate `surface_offset::offset_validity` (see module docs).
pub fn offset_globally_smooth(surface: &Surface, distance: f64) -> Result<OffsetCertificate> {
    surface.validate()?;
    check(distance.is_finite(), "Offset distance must be finite")?;
    let d = distance.abs();
    let enc = curvature_enclosure(surface)?;
    if enc.regular && enc.max_abs_principal.is_finite() {
        let q = d * enc.max_abs_principal;
        if q < 1. {
            return Ok(OffsetCertificate::Valid {
                margin: (1. - q).next_down(),
            });
        }
    }
    // Invalid side: certified pointwise lower bounds on max|k|.
    let du = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
    ];
    let dv = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.control_points[0].len()],
    ];
    let lower_abs = |[lo, hi]: [f64; 2]| {
        if lo <= 0. && hi >= 0. { 0. } else { lo.abs().min(hi.abs()) }
    };
    let mut k_lo = 0_f64;
    const N: usize = 7;
    for i in 0..N {
        let u = du[0] + (du[1] - du[0]) * i as f64 / (N - 1) as f64;
        for j in 0..N {
            let v = dv[0] + (dv[1] - dv[0]) * j as f64 / (N - 1) as f64;
            let report = crate::surface_differential::at(
                surface,
                [u, v],
                [Side::Automatic, Side::Automatic],
            )?;
            if let Some([k1, k2]) = report.principal {
                k_lo = k_lo.max(lower_abs(k1).max(lower_abs(k2)));
            }
        }
    }
    if d * k_lo >= 1. {
        return Ok(OffsetCertificate::Invalid {
            max_d_kappa: d * k_lo,
        });
    }
    Ok(OffsetCertificate::Unknown)
}

/// Draft-angle enclosure for DFM (checklist 511): certified interval of the
/// angle between the surface normal and `pull_dir` over the whole patch,
/// derived from the normal cone. Draft exists everywhere only when the
/// enclosure stays clear of `pi/2` on the intended side.
pub fn draft_angle_enclosure(surface: &Surface, pull_dir: [f64; 3]) -> Result<Interval> {
    check(
        pull_dir.iter().all(|x| x.is_finite()) && norm(pull_dir) > 0.,
        "Pull direction must be finite and nonzero",
    )?;
    surface.validate()?;
    // Bound each local normal ball against the pull direction directly.
    // A single global cone loses directional information on curved strips.
    let mut result = None;
    for [u, v] in subdivision(surface, 96) {
        let Some(bb) = box_bounds(surface, u, v)? else {
            return Interval::new(0., std::f64::consts::PI.next_up());
        };
        let (mid, radius) = cross_ball(&bb, (u.hi-u.lo)/2., (v.hi-v.lo)/2.);
        let magnitude = norm(mid);
        if magnitude <= radius {
            return Interval::new(0., std::f64::consts::PI.next_up());
        }
        let theta = angle_between(mid, pull_dir);
        let alpha = up((radius/magnitude).min(1.).asin());
        let local = Interval::new((theta-alpha).max(0.).next_down(),
            (theta+alpha).min(std::f64::consts::PI).next_up())?;
        result = Some(match result {None => local, Some(previous) => union(previous, local)?});
    }
    result.ok_or_else(|| numeric_err("Surface has no nonempty knot cells"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{primitives, surface};

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        }
        fn f64(&mut self) -> f64 {
            (self.next() >> 11) as f64 / (1u64 << 53) as f64
        }
        fn range(&mut self, a: f64, b: f64) -> f64 {
            a + (b - a) * self.f64()
        }
    }

    /// Regular spherical patch: a meridian arc at +/-45 deg revolved 60 deg.
    fn spherical_patch(radius: f64) -> Surface {
        let arc = primitives::circle_arc(
            [0.; 3],
            [0., 1., 0.],
            radius,
            -45.,
            90.,
        )
        .unwrap();
        surface::revolve(&arc, [0.; 3], [0., 0., 1.], 60.).unwrap()
    }

    fn sample_normals(s: &Surface, n: usize) -> Vec<[f64; 3]> {
        let (du, dv) = (
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        );
        let mut out = Vec::new();
        for i in 0..n {
            let u = du[0] + (du[1] - du[0]) * i as f64 / (n - 1) as f64;
            for j in 0..n {
                let v = dv[0] + (dv[1] - dv[0]) * j as f64 / (n - 1) as f64;
                if let Some(nrm) = s.evaluate(u, v).unwrap().unit_normal() {
                    out.push(nrm);
                }
            }
        }
        out
    }

    #[test]
    fn cone_contains_sampled_normals_property() {
        let surfaces = [
            spherical_patch(2.),
            primitives::quadratic_patch([0.5, 1., 0.5, 1.], [1., 0., 1., 0., 0., 0.]).unwrap(),
            primitives::quadratic_patch([-1., 1., -1., 1.], [1., 0., -1., 0., 0., 0.]).unwrap(),
        ];
        let mut rng = Rng(41);
        for s in &surfaces {
            let cone = normal_cone(s).unwrap();
            assert!(cone.half_angle <= std::f64::consts::PI);
            assert!(norm(cone.axis) > 0.);
            for n in sample_normals(s, 17) {
                assert!(
                    cone.contains(n),
                    "sampled normal escapes the cone (half_angle {})",
                    cone.half_angle
                );
            }
            // Random interior samples too.
            let (du, dv) = (
                [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
            );
            for _ in 0..256 {
                let u = rng.range(du[0], du[1]);
                let v = rng.range(dv[0], dv[1]);
                if let Some(n) = s.evaluate(u, v).unwrap().unit_normal() {
                    assert!(cone.contains(n));
                }
            }
        }
    }

    #[test]
    fn silhouette_rejection_on_convex_and_saddle_patches() {
        // Elliptic paraboloid z = x^2 + y^2 on [0.5, 1]^2: every normal has a
        // strictly negative x-component, so no silhouette from +x.
        let hill = primitives::quadratic_patch([0.5, 1., 0.5, 1.], [1., 0., 1., 0., 0., 0.]).unwrap();
        let cone = normal_cone(&hill).unwrap();
        assert!(
            cone.silhouette_impossible([1., 0., 0.]).unwrap(),
            "convex patch viewed along +x must reject the silhouette (cone {:?})",
            cone
        );
        // Saddle z = x^2 - y^2 on [-1,1]^2 viewed along +y: normals flip the
        // y sign across v, so a perpendicular direction exists.
        let saddle = primitives::quadratic_patch([-1., 1., -1., 1.], [1., 0., -1., 0., 0., 0.]).unwrap();
        let cone = normal_cone(&saddle).unwrap();
        assert!(
            !cone.silhouette_impossible([0., 1., 0.]).unwrap(),
            "saddle patch viewed along +y must not reject the silhouette (cone {:?})",
            cone
        );
    }

    #[test]
    fn offset_certificate_on_spherical_patch() {
        let r = 2.;
        let s = spherical_patch(r);
        // Exact principal curvature magnitude is 1/r = 0.5.
        match offset_globally_smooth(&s, 0.4).unwrap() {
            OffsetCertificate::Valid { margin } => {
                assert!(margin > 0. && margin <= 1. - 0.4 / r + 1e-12);
            }
            other => panic!("d = 0.4 << r must be certified valid, got {other:?}"),
        }
        // Zero distance is trivially smooth.
        assert!(matches!(
            offset_globally_smooth(&s, 0.).unwrap(),
            OffsetCertificate::Valid { .. }
        ));
        // d = 8 > r: focal crossing proven by sampled lower bounds.
        match offset_globally_smooth(&s, 8.).unwrap() {
            OffsetCertificate::Invalid { max_d_kappa } => assert!(max_d_kappa >= 1.),
            other => panic!("d = 8 >> r must be certified invalid, got {other:?}"),
        }
        // Plane: zero curvature, any distance is valid.
        let plane = primitives::quadratic_patch([0., 1., 0., 1.], [0.; 6]).unwrap();
        assert!(matches!(
            offset_globally_smooth(&plane, 1e6).unwrap(),
            OffsetCertificate::Valid { .. }
        ));
    }

    #[test]
    fn curvature_enclosure_brackets_exact_sphere_curvature() {
        let r = 2.;
        let s = spherical_patch(r);
        let enc = curvature_enclosure(&s).unwrap();
        assert!(enc.regular);
        // |K| = 1/r^2 = 0.25, |H| = 1/r = 0.5 (sign depends on orientation).
        assert!(enc.gaussian.contains(0.25) || enc.gaussian.contains(-0.25));
        assert!(enc.mean.contains(0.5) || enc.mean.contains(-0.5));
        assert!(enc.max_abs_principal >= 0.5);
        assert!(
            enc.max_abs_principal < 2.0,
            "sphere bound is absurdly loose: {}",
            enc.max_abs_principal
        );
    }

    #[test]
    fn draft_enclosure_on_cylinder_and_cone() {
        // Cylindrical patch (arc of ~115 deg extruded along z): every normal
        // is exactly perpendicular to the pull direction.
        let arc = primitives::circle_arc([0.; 3], [0., 0., 1.], 1., 0., 115.).unwrap();
        let cyl = surface::extrude(&arc, [0., 0., 2.]).unwrap();
        let draft = draft_angle_enclosure(&cyl, [0., 0., 1.]).unwrap();
        assert!(
            draft.contains(std::f64::consts::FRAC_PI_2),
            "cylinder draft must enclose pi/2: [{}, {}]",
            draft.lo,
            draft.hi
        );
        assert!(
            draft.lo > 0.2 && draft.hi < std::f64::consts::PI - 0.2,
            "cylinder draft should be far from 0 and pi: [{}, {}]",
            draft.lo,
            draft.hi
        );
        // Cone frustum: constant draft angle everywhere; enclosure contains
        // every sampled angle.
        let frustum = primitives::cone_frustum([0.; 3], 2., 1., 3.).unwrap();
        let draft = draft_angle_enclosure(&frustum, [0., 0., 1.]).unwrap();
        for n in sample_normals(&frustum, 9) {
            let angle = angle_between(n, [0., 0., 1.]);
            assert!(
                draft.contains(angle),
                "frustum draft [{}, {}] misses {angle}",
                draft.lo,
                draft.hi
            );
        }
    }
}

#[cfg(test)]
mod debug_tests2 {
    use super::*;
    use crate::primitives;

    #[test]
    fn debug_bounds() {
        let hill = primitives::quadratic_patch([0.5, 1., 0.5, 1.], [1., 0., 1., 0., 0., 0.]).unwrap();
        let u = Interval::new(0., 1. / 6.).unwrap();
        let v = Interval::new(0., 1. / 6.).unwrap();
        let bb = box_bounds(&hill, u, v).unwrap().unwrap();
        eprintln!("w_lo={} e={:?}", bb.w_lo, bb.e);
        eprintln!("b={:?}", bb.b);
        eprintln!("c={:?}", bb.c);
        let (mid, r) = cross_ball(&bb, 1. / 12., 1. / 12.);
        eprintln!("cross mid={mid:?} |mid|={} r={r}", norm(mid));
    }
}

#[cfg(test)]
mod debug_tests3 {
    use super::*;
    use crate::primitives;

    #[test]
    fn debug_sphere_curv() {
        let arc = primitives::circle_arc([0.; 3], [0., 1., 0.], 2., -std::f64::consts::FRAC_PI_4, std::f64::consts::FRAC_PI_2).unwrap();
        let s = crate::surface::revolve(&arc, [0.; 3], [0., 0., 1.], 60.).unwrap();
        for [u, v] in subdivision(&s, 32) {
            match box_bounds(&s, u, v) {
                Err(e) => { eprintln!("ERR {e:?} at u=[{},{}] v=[{},{}]", u.lo, u.hi, v.lo, v.hi); return; }
                Ok(None) => { eprintln!("SINGULAR w at u=[{},{}] v=[{},{}]", u.lo, u.hi, v.lo, v.hi); return; }
                Ok(Some(bb)) => {
                    let (mid, r) = cross_ball(&bb, (u.hi-u.lo)/2., (v.hi-v.lo)/2.);
                    let (mc, cl) = (norm(mid), norm(mid) - r);
                    if cl <= 0. {
                        eprintln!("DEGENERATE cl={cl} mc={mc} r={r} at u=[{:.3},{:.3}] v=[{:.3},{:.3}]", u.lo, u.hi, v.lo, v.hi);
                        eprintln!("  b={:?}", bb.b);
                        return;
                    }
                }
            }
        }
        let enc = curvature_enclosure(&s).unwrap();
        eprintln!("enc regular={} kmax={} gauss=[{},{}] mean=[{},{}]", enc.regular, enc.max_abs_principal, enc.gaussian.lo, enc.gaussian.hi, enc.mean.lo, enc.mean.hi);
    }

    #[test]
    fn debug_frustum() {
        let f = primitives::cone_frustum([0.; 3], 2., 1., 3.).unwrap();
        eprintln!("frustum nu={} nv={} pu={} pv={} ku={:?} kv={:?}",
            f.control_points.len(), f.control_points[0].len(), f.degree_u, f.degree_v,
            f.knots_u, f.knots_v);
        eprintln!("row0 y: {:?}", f.control_points[0].iter().map(|p| p[1]).collect::<Vec<_>>());
        eprintln!("col0 y: {:?}", f.control_points.iter().map(|r| r[0][1]).collect::<Vec<_>>());
        for [u, v] in subdivision(&f, 24) {
            if let Err(e) = box_bounds(&f, u, v) {
                eprintln!("ERR {e:?} at u=[{},{}] v=[{},{}]", u.lo, u.hi, v.lo, v.hi);
                let h = homogeneous_sums(&f, u, v);
                match h {
                    Ok(h) => eprintln!("  h00w=[{},{}]", h[0][0][3].lo, h[0][0][3].hi),
                    Err(e) => eprintln!("  h err {e:?}"),
                }
                return;
            }
        }
        eprintln!("frustum nu={} nv={} pu={} pv={} ku={:?} kv={:?} w00={} p00={:?}",
            f.control_points.len(), f.control_points[0].len(), f.degree_u, f.degree_v,
            f.knots_u, f.knots_v, f.weights[0][0], f.control_points[0][0]);
        eprintln!("row0 y: {:?}", f.control_points[0].iter().map(|p| p[1]).collect::<Vec<_>>());
        eprintln!("col0 y: {:?}", f.control_points.iter().map(|r| r[0][1]).collect::<Vec<_>>());
        match draft_angle_enclosure(&f, [0., 0., 1.]) {
            Ok(d) => eprintln!("draft [{}, {}]", d.lo, d.hi),
            Err(e) => eprintln!("ERR {e:?}"),
        }
    }
}
