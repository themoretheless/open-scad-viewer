//! Bicubic Gregory patch for four-sided holes with prescribed G1 skeleton.
//!
//! Each boundary carries a cubic boundary curve plus a cubic cross-boundary
//! derivative field (the G1 skeleton). The four interior control points of
//! the bicubic net are rational Gregory blends of two edge candidates:
//! `P(s, t) = (s·C_s + t·C_t) / (s + t)` in the local corner coordinates,
//! which makes the patch interpolate every boundary curve exactly and match
//! every prescribed cross derivative along the open edge; at a corner with
//! incompatible twists the two limits differ and the blend mediates them.
//!
//! Scope decision: only `n = 4` is implemented. For `n = 3, 5, 6` the
//! constructor returns an `INVALID_INPUT` error instead of fan-splitting —
//! a fan of four-sided patches changes the boundary parameterization and is
//! a modeling operation, not a primitive. The patch is stored as an
//! evaluating structure ([`GregoryPatch::evaluate`]); baking into a NURBS
//! surface is available as [`GregoryPatch::to_nurbs_approx`], an adaptive
//! bilinear interpolation grid (foundation `interpolate_surface_grid_report`)
//! refined until midpoint deviation meets the tolerance.
//!
//! Input contract: every curve must be polynomial (uniform weights),
//! non-periodic, of degree at most 3, and reducible to a single Bézier
//! segment. Edges are ordered counterclockwise around the hole:
//! E0 bottom (u: 0→1), E1 right (v: 0→1), E2 top (u: 1→0), E3 left
//! (v: 1→0); each cross field points INTO the hole interior.
use crate::{
    Result, check, numeric,
    curve::Curve,
    foundation::fitting::interpolate_surface_grid_report,
    surface::Surface,
};

/// One side of the hole: the boundary curve and the inward cross-boundary
/// derivative field, both parameterized along the edge.
#[derive(Clone, Debug)]
pub struct GregoryBoundary {
    pub curve: Curve,
    /// Control points are derivative vectors (not positions).
    pub cross_derivative: Curve,
}

/// Point and first partials of a Gregory patch evaluation.
#[derive(Clone, Copy, Debug)]
pub struct GregoryEvaluation {
    pub point: [f64; 3],
    pub du: [f64; 3],
    pub dv: [f64; 3],
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn norm(a: [f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

/// Reduce a curve to a single cubic Bézier segment on [0, 1]; returns its
/// four control points.
fn cubic_bezier(curve: &Curve, role: &str) -> Result<[[f64; 3]; 4]> {
    curve.validate()?;
    check(
        curve.control_points[0].len() == 3 && !curve.periodic,
        "Gregory boundaries must be non-periodic 3D curves",
    )?;
    check(
        curve.degree <= 3,
        "Gregory boundaries must have degree at most 3",
    )?;
    let elevated = curve.elevate(3)?;
    let segments = elevated.decompose()?;
    check(
        segments.len() == 1,
        "Gregory boundaries must reduce to a single Bezier segment; split the hole first",
    )?;
    let mut segment = segments[0].definition().clone();
    let (min, max) = segment
        .weights
        .iter()
        .fold((f64::INFINITY, f64::MIN), |(lo, hi), &w| (lo.min(w), hi.max(w)));
    check(
        (max - min) <= 1e-12 * max.abs().max(1.),
        "Gregory boundaries must be polynomial (uniform weights); rational edges are not supported",
    )?;
    let [a, b] = segment.domain();
    check(b > a, "Gregory boundary needs a nonzero domain")?;
    if a != 0. || b != 1. {
        for k in &mut segment.knots {
            *k = (*k - a) / (b - a);
        }
    }
    check(
        segment.control_points.len() == 4,
        "Gregory boundary reduction failed",
    )?;
    let _ = role;
    Ok(std::array::from_fn(|i| {
        let p = &segment.control_points[i];
        [p[0], p[1], p[2]]
    }))
}

/// A four-sided Gregory patch: boundary ring of a bicubic net plus two
/// interior candidates per corner.
#[derive(Clone, Debug)]
pub struct GregoryPatch {
    /// Boundary control ring, net[i][j] with i along u, j along v; interior
    /// entries are unused placeholders.
    boundary: [[ [f64; 3]; 4 ]; 4],
    /// Per corner: (s-edge candidate, t-edge candidate) in local coordinates
    /// corner0 (0,0): s=u, t=v; corner1 (1,0): s=v, t=1-u;
    /// corner2 (1,1): s=1-v, t=1-u; corner3 (0,1): s=u, t=1-v.
    interior: [([f64; 3], [f64; 3]); 4],
}

impl GregoryPatch {
    /// Build a patch from exactly four loop-closed boundaries. Returns
    /// `INVALID_INPUT` for any other side count: fan splitting of n=3/5/6
    /// holes is deliberately not implemented (see module docs).
    pub fn new(boundaries: &[GregoryBoundary], closure_tolerance: f64) -> Result<Self> {
        check(
            boundaries.len() == 4,
            "Gregory patch supports exactly 4 boundaries; n=3, 5 and 6 via fan splitting is not supported",
        )?;
        check(
            closure_tolerance.is_finite() && closure_tolerance >= 0.,
            "Gregory closure tolerance must be finite and nonnegative",
        )?;
        let mut edges = Vec::with_capacity(4);
        let mut cross = Vec::with_capacity(4);
        for (index, b) in boundaries.iter().enumerate() {
            edges.push(cubic_bezier(&b.curve, "boundary")?);
            cross.push(cubic_bezier(&b.cross_derivative, "cross")?);
            let _ = index;
        }
        // Loop closure: E_i.end == E_{i+1}.start around the cycle.
        for i in 0..4 {
            let gap = norm(sub(edges[i][3], edges[(i + 1) % 4][0]));
            check(
                gap <= closure_tolerance,
                "Gregory boundaries do not form a closed loop",
            )?;
        }
        // Interior candidates: every cross field already points INTO the
        // hole, so each interior row point is boundary + D/3 on both edges.
        let third = |p: [f64; 3], d: [f64; 3]| add(p, scale(d, 1. / 3.));
        let mut boundary: [[ [f64; 3]; 4 ]; 4] = [[ [0.; 3]; 4 ]; 4];
        for i in 0..4 {
            boundary[i][0] = edges[0][i];
            boundary[3][i] = edges[1][i];
            boundary[i][3] = edges[2][3 - i];
            boundary[0][i] = edges[3][3 - i];
        }
        // Interior candidates; cross fields point inward, so the offset is
        // uniformly +D/3 on every edge.
        let interior = [
            // corner (0,0): bottom edge candidate, left edge candidate
            (third(edges[0][1], cross[0][1]), third(edges[3][2], cross[3][2])),
            // corner (1,0): right edge candidate, bottom edge candidate
            (third(edges[1][1], cross[1][1]), third(edges[0][2], cross[0][2])),
            // corner (1,1): right edge candidate, top edge candidate
            (third(edges[1][2], cross[1][2]), third(edges[2][1], cross[2][1])),
            // corner (0,1): top edge candidate, left edge candidate
            (third(edges[2][2], cross[2][2]), third(edges[3][1], cross[3][1])),
        ];
        Ok(GregoryPatch { boundary, interior })
    }

    /// Interior control point and its partials at corner `c` for local
    /// coordinates (s, t) with ds/du, ds/dv, dt/du, dt/dv signs.
    fn interior_point(
        &self,
        c: usize,
        s: f64,
        t: f64,
        ds_du: f64,
        ds_dv: f64,
        dt_du: f64,
        dt_dv: f64,
    ) -> ([f64; 3], [f64; 3], [f64; 3]) {
        let (cs, ct) = self.interior[c];
        let sum = s + t;
        if sum.abs() < 1e-12 {
            // Corner singularity of the rational blend: use the averaged
            // candidate; the boundary ring alone defines the corner jet.
            return (scale(add(cs, ct), 0.5), [0.; 3], [0.; 3]);
        }
        let point = scale(add(scale(cs, s), scale(ct, t)), 1. / sum);
        let diff = sub(cs, ct);
        let den = sum * sum;
        // dP/ds = t(cs - ct)/(s+t)^2, dP/dt = s(ct - cs)/(s+t)^2.
        let dp_ds = scale(diff, t / den);
        let dp_dt = scale(diff, -s / den);
        let du = add(scale(dp_ds, ds_du), scale(dp_dt, dt_du));
        let dv = add(scale(dp_ds, ds_dv), scale(dp_dt, dt_dv));
        (point, du, dv)
    }

    /// Interior net point (i, j) ∈ {1, 2}² with partials at (u, v).
    fn net_interior(&self, i: usize, j: usize, u: f64, v: f64) -> ([f64; 3], [f64; 3], [f64; 3]) {
        match (i, j) {
            (1, 1) => self.interior_point(0, u, v, 1., 0., 0., 1.),
            (2, 1) => self.interior_point(1, v, 1. - u, 0., 1., -1., 0.),
            (2, 2) => self.interior_point(2, 1. - v, 1. - u, 0., -1., -1., 0.),
            (1, 2) => self.interior_point(3, u, 1. - v, 1., 0., 0., -1.),
            _ => unreachable!("interior indices are 1..=2"),
        }
    }

    /// Evaluate the patch and its first partials at (u, v) ∈ [0, 1]².
    pub fn evaluate(&self, u: f64, v: f64) -> Result<GregoryEvaluation> {
        check(
            u.is_finite() && v.is_finite() && (0. ..=1.).contains(&u) && (0. ..=1.).contains(&v),
            "Gregory evaluation parameters must lie in [0, 1]",
        )?;
        let bu = [
            (1. - u).powi(3),
            3. * u * (1. - u).powi(2),
            3. * u * u * (1. - u),
            u.powi(3),
        ];
        let bv = [
            (1. - v).powi(3),
            3. * v * (1. - v).powi(2),
            3. * v * v * (1. - v),
            v.powi(3),
        ];
        let dbu = [
            -3. * (1. - u).powi(2),
            3. * (1. - u) * (1. - 3. * u),
            3. * u * (2. - 3. * u),
            3. * u * u,
        ];
        let dbv = [
            -3. * (1. - v).powi(2),
            3. * (1. - v) * (1. - 3. * v),
            3. * v * (2. - 3. * v),
            3. * v * v,
        ];
        let (mut point, mut du, mut dv) = ([0.; 3], [0.; 3], [0.; 3]);
        for i in 0..4 {
            for j in 0..4 {
                let (p, p_du, p_dv) = if (1..=2).contains(&i) && (1..=2).contains(&j) {
                    self.net_interior(i, j, u, v)
                } else {
                    (self.boundary[i][j], [0.; 3], [0.; 3])
                };
                let w = bu[i] * bv[j];
                point = add(point, scale(p, w));
                du = add(du, add(scale(p, dbu[i] * bv[j]), scale(p_du, w)));
                dv = add(dv, add(scale(p, bu[i] * dbv[j]), scale(p_dv, w)));
            }
        }
        numeric(
            point.iter().chain(du.iter()).chain(dv.iter()).all(|v| v.is_finite()),
            "Gregory evaluation exceeded finite numeric bounds",
        )?;
        Ok(GregoryEvaluation { point, du, dv })
    }

    /// Bake the patch into a polynomial NURBS surface: adaptive bilinear
    /// interpolation grid (5×5 up to 32×32), refined until the midpoint
    /// deviation meets `tolerance`. The result is a degree-1 approximation;
    /// use it for tessellation/exchange, not as a smooth replacement.
    pub fn to_nurbs_approx(&self, tolerance: f64) -> Result<Surface> {
        check(
            tolerance.is_finite() && tolerance > 0.,
            "Gregory baking tolerance must be finite and positive",
        )?;
        for size in [5_usize, 9, 17, 32] {
            let mut points = Vec::with_capacity(size);
            for i in 0..size {
                let mut row = Vec::with_capacity(size);
                for j in 0..size {
                    let u = i as f64 / (size - 1) as f64;
                    let v = j as f64 / (size - 1) as f64;
                    row.push(self.evaluate(u, v)?.point);
                }
                points.push(row);
            }
            let surface = interpolate_surface_grid_report(points, false, None)?.surface;
            let mut worst = 0_f64;
            for i in 0..size - 1 {
                for j in 0..size - 1 {
                    let u = (i as f64 + 0.5) / (size - 1) as f64;
                    let v = (j as f64 + 0.5) / (size - 1) as f64;
                    let exact = self.evaluate(u, v)?.point;
                    let approx = surface.evaluate(u, v)?.point;
                    worst = f64::max(worst, norm(sub(exact, approx)));
                }
            }
            if worst <= tolerance {
                return Ok(surface);
            }
        }
        Err(crate::numeric_err(
            "Gregory baking did not meet the tolerance within a 32x32 grid",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bernstein(t: f64) -> ([f64; 4], [f64; 4]) {
        (
            [
                (1. - t).powi(3),
                3. * t * (1. - t).powi(2),
                3. * t * t * (1. - t),
                t.powi(3),
            ],
            [
                -3. * (1. - t).powi(2),
                3. * (1. - t) * (1. - 3. * t),
                3. * t * (2. - 3. * t),
                3. * t * t,
            ],
        )
    }

    /// Reference bicubic Bézier with nonzero twists.
    fn net() -> [[ [f64; 3]; 4 ]; 4] {
        std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                let x = i as f64 / 3.;
                let y = j as f64 / 3.;
                [x, y, 0.3 * x * y + 0.2 * (x * x - y) + 0.1 * (3. * x * y).sin()]
            })
        })
    }

    fn eval_reference(u: f64, v: f64) -> [f64; 3] {
        let (bu, _) = bernstein(u);
        let (bv, _) = bernstein(v);
        let net = net();
        let mut p = [0.; 3];
        for i in 0..4 {
            for j in 0..4 {
                p = add(p, scale(net[i][j], bu[i] * bv[j]));
            }
        }
        p
    }

    /// Row of the net: boundary curve controls at fixed v0, and the
    /// cross-derivative (d/dv) control row.
    fn v_rows(v0: f64) -> ([[f64; 3]; 4], [[f64; 3]; 4]) {
        let (bv, dbv) = bernstein(v0);
        let net = net();
        let mut r = [[0.; 3]; 4];
        let mut d = [[0.; 3]; 4];
        for i in 0..4 {
            for j in 0..4 {
                r[i] = add(r[i], scale(net[i][j], bv[j]));
                d[i] = add(d[i], scale(net[i][j], dbv[j]));
            }
        }
        (r, d)
    }

    fn u_rows(u0: f64) -> ([[f64; 3]; 4], [[f64; 3]; 4]) {
        let (bu, dbu) = bernstein(u0);
        let net = net();
        let mut r = [[0.; 3]; 4];
        let mut d = [[0.; 3]; 4];
        for j in 0..4 {
            for i in 0..4 {
                r[j] = add(r[j], scale(net[i][j], bu[i]));
                d[j] = add(d[j], scale(net[i][j], dbu[i]));
            }
        }
        (r, d)
    }

    fn bezier_curve(points: [[f64; 3]; 4]) -> Curve {
        Curve {
            degree: 3,
            knots: [vec![0.; 4], vec![1.; 4]].concat(),
            control_points: points.iter().map(|p| p.to_vec()).collect(),
            weights: vec![1.; 4],
            periodic: false,
        }
    }

    /// Four Gregory boundaries of the sub-rectangle [ua, ub] × [vc, vd] cut
    /// out of the reference bicubic, cross fields pointing inward.
    fn hole(ua: f64, ub: f64, vc: f64, vd: f64) -> Vec<GregoryBoundary> {
        // Boundary curves are the reference iso-curves trimmed to the spans;
        // cross fields are the reference partials rescaled to the LOCAL edge
        // parameters (d/du_local = (ub-ua)·d/du, d/dv_local = (vd-vc)·d/dv).
        let (b_row, b_der) = v_rows(vc);
        let (t_row, t_der) = v_rows(vd);
        let (l_row, l_der) = u_rows(ua);
        let (r_row, r_der) = u_rows(ub);
        let su = ub - ua;
        let sv = vd - vc;
        let scaled = |row: [[f64; 3]; 4], f: f64| std::array::from_fn(|k| scale(row[k], f));
        // Bottom: u from ua..ub, inward +v_local.
        let bottom = GregoryBoundary {
            curve: bezier_curve(b_row).trim(ua, ub).unwrap(),
            cross_derivative: bezier_curve(scaled(b_der, sv)).trim(ua, ub).unwrap(),
        };
        // Right: v from vc..vd, inward -u_local.
        let right = GregoryBoundary {
            curve: bezier_curve(r_row).trim(vc, vd).unwrap(),
            cross_derivative: bezier_curve(scaled(r_der, -su)).trim(vc, vd).unwrap(),
        };
        // Top: u from ub..ua (reversed), inward -v_local.
        let top = GregoryBoundary {
            curve: bezier_curve(t_row).trim(ua, ub).unwrap().reverse().unwrap(),
            cross_derivative: bezier_curve(scaled(t_der, -sv))
                .trim(ua, ub)
                .unwrap()
                .reverse()
                .unwrap(),
        };
        // Left: v from vd..vc (reversed), inward +u_local.
        let left = GregoryBoundary {
            curve: bezier_curve(l_row).trim(vc, vd).unwrap().reverse().unwrap(),
            cross_derivative: bezier_curve(scaled(l_der, su))
                .trim(vc, vd)
                .unwrap()
                .reverse()
                .unwrap(),
        };
        vec![bottom, right, top, left]
    }

    #[test]
    fn reproduces_reference_surface_and_edges_exactly() {
        let (ua, ub, vc, vd) = (0.2, 0.8, 0.25, 0.75);
        let patch = GregoryPatch::new(&hole(ua, ub, vc, vd), 1e-12).unwrap();
        for i in 0..=20 {
            for j in 0..=20 {
                let u = i as f64 / 20.;
                let v = j as f64 / 20.;
                let p = patch.evaluate(u, v).unwrap().point;
                let q = eval_reference(ua + (ub - ua) * u, vc + (vd - vc) * v);
                for k in 0..3 {
                    assert!((p[k] - q[k]).abs() < 1e-11, "({i}, {j}), axis {k}");
                }
            }
        }
        // Edges coincide with the input boundary curves.
        let boundaries = hole(ua, ub, vc, vd);
        for i in 0..=16 {
            let t = i as f64 / 16.;
            let on_edge = [
                patch.evaluate(t, 0.).unwrap().point,
                patch.evaluate(1., t).unwrap().point,
                patch.evaluate(1. - t, 1.).unwrap().point,
                patch.evaluate(0., 1. - t).unwrap().point,
            ];
            for (edge, p) in on_edge.into_iter().enumerate() {
                let c = &boundaries[edge].curve;
                let [a, b] = c.domain();
                let q = c.evaluate(a + (b - a) * t).unwrap().point;
                for k in 0..3 {
                    assert!((p[k] - q[k]).abs() < 1e-11, "edge {edge}, t {t}");
                }
            }
        }
    }

    #[test]
    fn cross_derivatives_match_the_g1_skeleton() {
        let (ua, ub, vc, vd) = (0.2, 0.8, 0.25, 0.75);
        let boundaries = hole(ua, ub, vc, vd);
        let patch = GregoryPatch::new(&boundaries, 1e-12).unwrap();
        let mut worst = 0_f64;
        let pt = |p: &[f64]| [p[0], p[1], p[2]];
        let on = |c: &Curve, t: f64| {
            let [a, b] = c.domain();
            pt(&c.evaluate(a + (b - a) * t).unwrap().point)
        };
        for i in 0..=16 {
            let t = i as f64 / 16.;
            // Bottom edge: patch dv vs prescribed cross field.
            let e = patch.evaluate(t, 0.).unwrap();
            worst = f64::max(worst, norm(sub(e.dv, on(&boundaries[0].cross_derivative, t))));
            // Left edge: patch du vs prescribed cross field (E3 reversed).
            let e = patch.evaluate(0., 1. - t).unwrap();
            worst = f64::max(worst, norm(sub(e.du, on(&boundaries[3].cross_derivative, t))));
        }
        assert!(worst < 1e-10, "G1 jump {worst}");
    }

    #[test]
    fn validates_closure_degree_and_side_count() {
        let (ua, ub, vc, vd) = (0.2, 0.8, 0.25, 0.75);
        let mut broken = hole(ua, ub, vc, vd);
        broken[1].curve.control_points[0][0] += 0.01;
        let err = GregoryPatch::new(&broken, 1e-9).unwrap_err().to_string();
        assert!(err.contains("closed loop"), "{err}");
        let three = hole(ua, ub, vc, vd)[..3].to_vec();
        let err = GregoryPatch::new(&three, 1e-9).unwrap_err().to_string();
        assert!(err.contains("exactly 4"), "{err}");
        let mut high = hole(ua, ub, vc, vd);
        high[0].curve = high[0].curve.elevate(4).unwrap();
        let err = GregoryPatch::new(&high, 1e-9).unwrap_err().to_string();
        assert!(err.contains("degree at most 3"), "{err}");
        let mut multi = hole(ua, ub, vc, vd);
        multi[0].curve = multi[0].curve.insert(0.5, 1).unwrap();
        let err = GregoryPatch::new(&multi, 1e-9).unwrap_err().to_string();
        assert!(err.contains("single Bezier segment"), "{err}");
    }

    #[test]
    fn bakes_into_nurbs_within_tolerance() {
        let (ua, ub, vc, vd) = (0.2, 0.8, 0.25, 0.75);
        let patch = GregoryPatch::new(&hole(ua, ub, vc, vd), 1e-12).unwrap();
        let tolerance = 1e-3;
        let surface = patch.to_nurbs_approx(tolerance).unwrap();
        surface.validate().unwrap();
        let mut worst = 0_f64;
        for i in 0..=24 {
            for j in 0..=24 {
                let u = i as f64 / 24.;
                let v = j as f64 / 24.;
                let p = patch.evaluate(u, v).unwrap().point;
                let q = surface.evaluate(u, v).unwrap().point;
                worst = f64::max(worst, norm(sub(p, q)));
            }
        }
        assert!(worst <= tolerance * 1.5, "baked deviation {worst}");
        assert!(patch.to_nurbs_approx(0.).is_err());
    }
}
