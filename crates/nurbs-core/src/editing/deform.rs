//! Geometry deformations (items 852–859): free-form deformation (FFD) with a
//! trivariate B-spline lattice, Barr's global deformations (taper, twist,
//! bend) applied to NURBS control points, and wire deformations after
//! Singh & Fiume.
//!
//! FFD. The lattice is a clamped tensor-product B-spline volume of degree
//! (l, m, n) over the axis-aligned bounding box of the geometry, with
//! controls placed at the Greville abscissae so the freshly built lattice is
//! an exact identity map. Points are parameterized into lattice (s, t, u)
//! coordinates through the box mapping with a small point→parameter cache;
//! deformation evaluates the tensor-product B-spline locally, so moving one
//! lattice control changes only points whose parameters fall inside that
//! control's knot-span support. Applying the lattice to a NURBS curve or
//! surface deforms its control points (weights and knots are untouched).
//!
//! Barr deformations are closed-form maps applied to the control net. The
//! taper additionally reports corrected surface normals (evaluated at the
//! Greville parameters of every control point on the undeformed surface and
//! pushed through the transposed inverse Jacobian of the taper map). The
//! bend is confined to a band along the deformed axis; outside the band the
//! map is rigid.
//!
//! Wire deformation. Each wire pairs a base curve with a target curve and a
//! radius. A point projects onto the base curve (coarse sampling plus a
//! budgeted Newton refinement); the displacement of the closest base point
//! to its image on the target curve, weighted by the radial falloff
//! exp(−d²/r²), pulls the point. Multiple wires blend by normalized weight
//! division, so duplicating a wire does not double its effect.
use std::cell::RefCell;
use std::collections::HashMap;

use crate::{
    Result, check, numeric,
    curve::{Curve, basis_funs_ders, find_span},
    foundation::guards::{Budget, canonical_f64},
    surface::Surface,
};

/// Newton budget for the wire closest-point refinement.
const NEWTON_BUDGET: usize = 16;
/// Coarse samples per active knot span for the wire projection seed.
const COARSE_PER_SPAN: usize = 8;
/// Cap on the point→parameter cache; beyond it the cache stops growing
/// (deterministic, insertion-order independent behaviour is unaffected).
const CACHE_CAP: usize = 4096;

/// 3x3 matrix inverse by the adjugate; singular matrices are numeric errors.
fn inverse3(m: [[f64; 3]; 3]) -> Result<[[f64; 3]; 3]> {
    let c = |a: usize, b: usize, i: usize, j: usize| {
        m[a][i] * m[b][j] - m[a][j] * m[b][i]
    };
    let det = m[0][0] * c(1, 2, 1, 2) - m[0][1] * c(1, 2, 0, 2) + m[0][2] * c(1, 2, 0, 1);
    numeric(
        det.is_finite() && det.abs() > 1e-30,
        "Deformation Jacobian is singular",
    )?;
    let inv = 1. / det;
    Ok([
        [c(1, 2, 1, 2) * inv, -c(0, 2, 1, 2) * inv, c(0, 1, 1, 2) * inv],
        [-c(1, 2, 0, 2) * inv, c(0, 2, 0, 2) * inv, -c(0, 1, 0, 2) * inv],
        [c(1, 2, 0, 1) * inv, -c(0, 2, 0, 1) * inv, c(0, 1, 0, 1) * inv],
    ])
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn norm(a: [f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

fn to3(point: &[f64]) -> Result<[f64; 3]> {
    check(point.len() == 3, "Expected a 3D point")?;
    Ok([point[0], point[1], point[2]])
}

/// Clamped uniform knot vector on [0,1] for `count` controls of degree `p`.
fn clamped_uniform(count: usize, p: usize) -> Vec<f64> {
    let spans = count - p;
    let mut knots = vec![0.; p + 1];
    for s in 1..spans {
        knots.push(s as f64 / spans as f64);
    }
    knots.extend(vec![1.; p + 1]);
    knots
}

/// Greville abscissa of control `i`.
fn greville(knots: &[f64], p: usize, i: usize) -> f64 {
    (1..=p).map(|d| knots[i + d]).sum::<f64>() / p as f64
}

// ---------------------------------------------------------------------------
// (a) Free-form deformation
// ---------------------------------------------------------------------------

/// Trivariate B-spline FFD lattice over an axis-aligned box. Control index
/// (i, j, k) maps to slot `i + nu·(j + nv·k)`; slot layout is exposed only
/// through [`FfdLattice::control_index`].
#[derive(Clone, Debug)]
pub struct FfdLattice {
    degrees: [usize; 3],
    knots: [Vec<f64>; 3],
    counts: [usize; 3],
    min: [f64; 3],
    extent: [f64; 3],
    controls: Vec<[f64; 3]>,
    cache: RefCell<HashMap<[u64; 3], [f64; 3]>>,
}

impl FfdLattice {
    /// Identity lattice over `min..max` with the given degrees and segment
    /// (span) counts per axis. Controls sit at the Greville abscissae, so
    /// the box maps onto itself exactly.
    pub fn new(min: [f64; 3], max: [f64; 3], degrees: [usize; 3], segments: [usize; 3]) -> Result<Self> {
        let mut extent = [0.; 3];
        for axis in 0..3 {
            check(
                min[axis].is_finite() && max[axis].is_finite() && max[axis] > min[axis],
                "FFD box must be finite and nondegenerate on every axis",
            )?;
            check(
                (1..=7).contains(&degrees[axis]),
                "FFD lattice degrees must lie in [1, 7]",
            )?;
            check(segments[axis] >= 1, "FFD lattice needs at least one span per axis")?;
            extent[axis] = max[axis] - min[axis];
        }
        let counts = [
            degrees[0] + segments[0],
            degrees[1] + segments[1],
            degrees[2] + segments[2],
        ];
        let knots = [
            clamped_uniform(counts[0], degrees[0]),
            clamped_uniform(counts[1], degrees[1]),
            clamped_uniform(counts[2], degrees[2]),
        ];
        let mut controls = Vec::with_capacity(counts[0] * counts[1] * counts[2]);
        for k in 0..counts[2] {
            for j in 0..counts[1] {
                for i in 0..counts[0] {
                    controls.push([
                        min[0] + extent[0] * greville(&knots[0], degrees[0], i),
                        min[1] + extent[1] * greville(&knots[1], degrees[1], j),
                        min[2] + extent[2] * greville(&knots[2], degrees[2], k),
                    ]);
                }
            }
        }
        Ok(Self {
            degrees,
            knots,
            counts,
            min,
            extent,
            controls,
            cache: RefCell::new(HashMap::new()),
        })
    }

    /// Flat slot of lattice control (i, j, k).
    pub fn control_index(&self, i: usize, j: usize, k: usize) -> Result<usize> {
        check(
            i < self.counts[0] && j < self.counts[1] && k < self.counts[2],
            "FFD control index out of range",
        )?;
        Ok(i + self.counts[0] * (j + self.counts[1] * k))
    }

    /// Lattice control counts per axis.
    pub fn counts(&self) -> [usize; 3] {
        self.counts
    }

    /// Overwrite one lattice control point.
    pub fn set_control(&mut self, slot: usize, position: [f64; 3]) -> Result<()> {
        check(slot < self.controls.len(), "FFD control slot out of range")?;
        check(
            position.iter().all(|v| v.is_finite()),
            "FFD control position must be finite",
        )?;
        self.controls[slot] = position;
        Ok(())
    }

    pub fn control(&self, slot: usize) -> Result<[f64; 3]> {
        check(slot < self.controls.len(), "FFD control slot out of range")?;
        Ok(self.controls[slot])
    }

    /// Box parameterization point → (s, t, u) ∈ [0,1]³, cached on canonical
    /// bit patterns of the point coordinates: [`canonical_f64`] collapses
    /// `-0.0` into `+0.0` so signed zeros share one cache entry.
    pub fn parameterize(&self, point: [f64; 3]) -> Result<[f64; 3]> {
        check(
            point.iter().all(|v| v.is_finite()),
            "FFD query point must be finite",
        )?;
        let key = [
            canonical_f64(point[0]).to_bits(),
            canonical_f64(point[1]).to_bits(),
            canonical_f64(point[2]).to_bits(),
        ];
        if let Some(hit) = self.cache.borrow().get(&key) {
            return Ok(*hit);
        }
        let mut stp = [0.; 3];
        for axis in 0..3 {
            let t = (point[axis] - self.min[axis]) / self.extent[axis];
            let tolerance = 1e-12;
            check(
                t >= -tolerance && t <= 1. + tolerance,
                "FFD query point is outside the lattice box",
            )?;
            stp[axis] = t.clamp(0., 1.);
        }
        let mut cache = self.cache.borrow_mut();
        if cache.len() < CACHE_CAP {
            cache.insert(key, stp);
        }
        Ok(stp)
    }

    /// Deform a world point through the lattice. Evaluation is local: only
    /// the (l+1)(m+1)(n+1) controls on the active spans contribute.
    pub fn deform_point(&self, point: [f64; 3]) -> Result<[f64; 3]> {
        let stp = self.parameterize(point)?;
        let mut windows = [Vec::new(), Vec::new(), Vec::new()];
        let mut spans = [0usize; 3];
        for axis in 0..3 {
            spans[axis] = find_span(
                self.degrees[axis],
                &self.knots[axis],
                self.counts[axis],
                stp[axis],
            )?;
            windows[axis] = basis_funs_ders(
                self.degrees[axis],
                &self.knots[axis],
                spans[axis],
                stp[axis],
                0,
            )?
            .values;
        }
        let mut out = [0.; 3];
        for (dk, &bk) in windows[2].iter().enumerate() {
            if bk == 0. {
                continue;
            }
            let k = spans[2] - self.degrees[2] + dk;
            for (dj, &bj) in windows[1].iter().enumerate() {
                if bj == 0. {
                    continue;
                }
                let j = spans[1] - self.degrees[1] + dj;
                for (di, &bi) in windows[0].iter().enumerate() {
                    if bi == 0. {
                        continue;
                    }
                    let i = spans[0] - self.degrees[0] + di;
                    let weight = bi * bj * bk;
                    let control = self.controls[i + self.counts[0] * (j + self.counts[1] * k)];
                    for axis in 0..3 {
                        out[axis] += weight * control[axis];
                    }
                }
            }
        }
        numeric(
            out.iter().all(|v| v.is_finite()),
            "FFD evaluation produced non-finite data",
        )?;
        Ok(out)
    }

    /// Deform world points in place.
    pub fn apply_to_points(&self, points: &mut [[f64; 3]]) -> Result<()> {
        for point in points.iter_mut() {
            *point = self.deform_point(*point)?;
        }
        Ok(())
    }

    /// Deform the control points of a 3D NURBS curve; knots, weights and
    /// periodicity are untouched.
    pub fn apply_to_curve(&self, curve: &Curve) -> Result<Curve> {
        curve.validate()?;
        let mut control_points = curve.control_points.clone();
        for point in control_points.iter_mut() {
            check(point.len() == 3, "FFD applies to 3D curves only")?;
            let deformed = self.deform_point([point[0], point[1], point[2]])?;
            point.copy_from_slice(&deformed);
        }
        Ok(Curve {
            control_points,
            ..curve.clone()
        })
    }

    /// Deform the control points of a 3D NURBS surface; knots, weights and
    /// periodicity are untouched.
    pub fn apply_to_surface(&self, surface: &Surface) -> Result<Surface> {
        surface.validate()?;
        let mut control_points = surface.control_points.clone();
        for row in control_points.iter_mut() {
            for point in row.iter_mut() {
                check(point.len() == 3, "FFD applies to 3D surfaces only")?;
                let deformed = self.deform_point([point[0], point[1], point[2]])?;
                point.copy_from_slice(&deformed);
            }
        }
        Ok(Surface {
            control_points,
            ..surface.clone()
        })
    }
}

// ---------------------------------------------------------------------------
// (b) Barr deformations
// ---------------------------------------------------------------------------

/// The two axes perpendicular to `axis`, in deterministic order.
fn perpendicular_axes(axis: usize) -> Result<(usize, usize)> {
    check(axis < 3, "Barr deformation axis must be 0, 1 or 2")?;
    Ok(((axis + 1) % 3, (axis + 2) % 3))
}

/// Normalized band coordinate: 0 below the band, 1 above it, affine inside.
fn band_coordinate(value: f64, band: [f64; 2]) -> Result<f64> {
    check(
        band[0].is_finite() && band[1].is_finite() && band[1] > band[0],
        "Barr band must be finite and nondegenerate",
    )?;
    Ok(((value - band[0]) / (band[1] - band[0])).clamp(0., 1.))
}

fn deform_surface_controls(
    surface: &Surface,
    map: impl Fn([f64; 3]) -> Result<[f64; 3]>,
) -> Result<Surface> {
    surface.validate()?;
    let mut control_points = surface.control_points.clone();
    for row in control_points.iter_mut() {
        for point in row.iter_mut() {
            check(point.len() == 3, "Barr deformations apply to 3D surfaces only")?;
            let mapped = map([point[0], point[1], point[2]])?;
            numeric(
                mapped.iter().all(|v| v.is_finite()),
                "Barr deformation produced non-finite control data",
            )?;
            point.copy_from_slice(&mapped);
        }
    }
    Ok(Surface {
        control_points,
        ..surface.clone()
    })
}

/// Barr twist about `axis` through `center`: the cross-section rotates by an
/// angle growing affinely from 0 at `band[0]` to `angle` at `band[1]` and
/// staying constant beyond the band.
pub fn barr_twist(
    surface: &Surface,
    axis: usize,
    center: [f64; 3],
    band: [f64; 2],
    angle: f64,
) -> Result<Surface> {
    check(angle.is_finite(), "Twist angle must be finite")?;
    let (a0, a1) = perpendicular_axes(axis)?;
    band_coordinate(center[axis], band).map(|_| ())?;
    deform_surface_controls(surface, |p| {
        let q = sub(p, center);
        let theta = angle * band_coordinate(q[axis] + center[axis], band)?;
        let (cos, sin) = (theta.cos(), theta.sin());
        let mut out = q;
        out[a0] = q[a0] * cos - q[a1] * sin;
        out[a1] = q[a0] * sin + q[a1] * cos;
        Ok([
            out[0] + center[0],
            out[1] + center[1],
            out[2] + center[2],
        ])
    })
}

/// Barr taper along `axis`: the perpendicular coordinates scale by a factor
/// growing affinely from 1 at `band[0]` to `scale` at `band[1]`. Also
/// returns surface normals evaluated at the Greville parameters of every
/// control point on the *undeformed* surface, corrected by the transposed
/// inverse Jacobian of the taper map and renormalized; entries are `None`
/// where the source surface has no regular normal.
pub fn barr_taper(
    surface: &Surface,
    axis: usize,
    center: [f64; 3],
    band: [f64; 2],
    scale: f64,
) -> Result<(Surface, Vec<Vec<Option<[f64; 3]>>>)> {
    check(
        scale.is_finite() && scale > 0.,
        "Taper scale must be positive and finite",
    )?;
    let (a0, a1) = perpendicular_axes(axis)?;
    // Corrected normals from the undeformed surface jets.
    let nu = surface.control_points.len();
    let nv = surface.control_points[0].len();
    let mut normals = vec![vec![None; nv]; nu];
    let band_width = band[1] - band[0];
    for i in 0..nu {
        for j in 0..nv {
            let u = greville(&surface.knots_u, surface.degree_u, i);
            let v = greville(&surface.knots_v, surface.degree_v, j);
            let Some(normal) = surface.evaluate(u, v)?.unit_normal() else {
                continue;
            };
            // Taper Jacobian at the control position: p'_a = r·q_a for the
            // perpendicular axes, p'_axis = q_axis, r affine in q_axis.
            let q = sub(
                [
                    surface.control_points[i][j][0],
                    surface.control_points[i][j][1],
                    surface.control_points[i][j][2],
                ],
                center,
            );
            let t = band_coordinate(q[axis] + center[axis], band)?;
            let r = 1. + (scale - 1.) * t;
            let dt = if q[axis] + center[axis] > band[0] && q[axis] + center[axis] < band[1] {
                1. / band_width
            } else {
                0.
            };
            let dr = (scale - 1.) * dt;
            let mut jacobian = [[0.; 3]; 3];
            jacobian[a0][a0] = r;
            jacobian[a1][a1] = r;
            jacobian[axis][axis] = 1.;
            jacobian[a0][axis] = q[a0] * dr;
            jacobian[a1][axis] = q[a1] * dr;
            let inv = inverse3(jacobian)?;
            // n' = J⁻ᵀ n, then renormalize.
            let mut corrected = [0.; 3];
            for row in 0..3 {
                corrected[row] = inv[0][row] * normal[0]
                    + inv[1][row] * normal[1]
                    + inv[2][row] * normal[2];
            }
            let length = norm(corrected);
            numeric(length > 0., "Taper collapsed a surface normal")?;
            normals[i][j] = Some([
                corrected[0] / length,
                corrected[1] / length,
                corrected[2] / length,
            ]);
        }
    }
    let deformed = deform_surface_controls(surface, |p| {
        let q = sub(p, center);
        let r = 1. + (scale - 1.) * band_coordinate(q[axis] + center[axis], band)?;
        let mut out = q;
        out[a0] = q[a0] * r;
        out[a1] = q[a1] * r;
        Ok([
            out[0] + center[0],
            out[1] + center[1],
            out[2] + center[2],
        ])
    })?;
    Ok((deformed, normals))
}

/// Barr bend confined to `band` along `axis`, bending toward `bend_dir`
/// around `center` with signed curvature `rate` (1/radius). Inside the band
/// the axis follows a circular arc of radius 1/|rate|; beyond the band the
/// map is a rigid rotation/translation of the cross-sections. `rate` must
/// be nonzero (a zero rate is the identity, not a bend).
pub fn barr_bend(
    surface: &Surface,
    axis: usize,
    bend_dir: usize,
    center: [f64; 3],
    band: [f64; 2],
    rate: f64,
) -> Result<Surface> {
    check(
        rate.is_finite() && rate != 0.,
        "Bend rate must be finite and nonzero",
    )?;
    check(axis < 3 && bend_dir < 3 && axis != bend_dir, "Bend axes must be distinct")?;
    let width = band[1] - band[0];
    check(
        width.is_finite() && width > 0.,
        "Bend band must be finite and nondegenerate",
    )?;
    // The band arc must fit the circle: |rate|·width < π avoids folding.
    check(
        (rate * width).abs() < std::f64::consts::PI,
        "Bend folds the band past half a turn",
    )?;
    let radius = 1. / rate;
    deform_surface_controls(surface, |p| {
        let q = sub(p, center);
        let clamped = q[axis].clamp(band[0], band[1]);
        let dz = q[axis] - clamped;
        let phi = rate * (clamped - band[0]);
        let (cos, sin) = (phi.cos(), phi.sin());
        // Centerline arc of radius |radius| anchored at the band start:
        // (0, zc) → (R(1−cosφ), z0 + R sinφ); cross-sections stay rigid and
        // perpendicular to the centerline, offsets beyond the band continue
        // along the end tangent.
        let mut out = q;
        out[bend_dir] = radius * (1. - cos) + q[bend_dir] * cos + dz * sin;
        out[axis] = band[0] + radius * sin - q[bend_dir] * sin + dz * cos;
        Ok([
            out[0] + center[0],
            out[1] + center[1],
            out[2] + center[2],
        ])
    })
}

// ---------------------------------------------------------------------------
// (c) Wire deformation (Singh–Fiume)
// ---------------------------------------------------------------------------

/// One wire: a base curve, its deformed target, and the falloff radius.
#[derive(Clone, Debug)]
pub struct Wire {
    pub base: Curve,
    pub target: Curve,
    pub radius: f64,
}

/// Multi-wire deformer with normalized weight blending.
#[derive(Clone, Debug)]
pub struct WireDeformer {
    wires: Vec<Wire>,
}

impl WireDeformer {
    pub fn new(wires: Vec<Wire>) -> Result<Self> {
        check(!wires.is_empty(), "Wire deformer needs at least one wire")?;
        check(wires.len() <= 64, "Wire deformer supports at most 64 wires")?;
        for wire in &wires {
            wire.base.validate()?;
            wire.target.validate()?;
            check(
                wire.base.domain() == wire.target.domain(),
                "Wire base and target must share a parameter domain",
            )?;
            check(
                wire.radius.is_finite() && wire.radius > 0.,
                "Wire radius must be positive and finite",
            )?;
        }
        Ok(Self { wires })
    }

    /// Closest point on the base curve: coarse per-span sampling seeds a
    /// budgeted Newton refinement on g(u) = (C(u)−p)·C′(u).
    fn project(base: &Curve, point: [f64; 3]) -> Result<(f64, [f64; 3])> {
        let [a, b] = base.domain();
        let spans: Vec<usize> = (base.degree..base.control_points.len())
            .filter(|&s| base.knots[s + 1] > base.knots[s])
            .collect();
        check(!spans.is_empty(), "Wire base curve has no active span")?;
        let mut best_u = a;
        let mut best_d2 = f64::INFINITY;
        for (si, &span) in spans.iter().enumerate() {
            let lo = base.knots[span];
            let hi = base.knots[span + 1];
            for s in 0..COARSE_PER_SPAN {
                // Cell midpoints; exact knot hits are nudged into the span.
                let mut u = lo + (hi - lo) * (2 * s + 1) as f64 / (2 * COARSE_PER_SPAN) as f64;
                for _ in 0..4 {
                    if base.knots.iter().any(|&k| k == u) && si + 1 < spans.len() {
                        u = u.next_up();
                    } else {
                        break;
                    }
                }
                let c = base.evaluate(u)?.point;
                let d2: f64 = (0..3).map(|i| (c[i] - point[i]).powi(2)).sum();
                if d2 < best_d2 {
                    best_d2 = d2;
                    best_u = u;
                }
            }
        }
        // Newton refinement with damping toward the bracketing domain, under
        // a unified budget guard (item 1065, stage `deform.wire-newton`).
        let mut guard = Budget::with_iterations(NEWTON_BUDGET)?
            .guard("deform.wire-newton");
        let mut u = best_u;
        for _ in 0..NEWTON_BUDGET {
            guard.tick()?;
            let evaluation = base.evaluate(u)?;
            let (Some(d1), Some(d2)) = (evaluation.d1, evaluation.d2) else {
                break;
            };
            let diff = sub(to3(&evaluation.point)?, point);
            let g: f64 = (0..3).map(|i| diff[i] * d1[i]).sum();
            let gp: f64 = (0..3).map(|i| d1[i] * d1[i] + diff[i] * d2[i]).sum();
            if gp.abs() < 1e-30 {
                break;
            }
            let step = g / gp;
            let candidate = (u - step).clamp(a, b);
            if (candidate - u).abs() < 1e-14 * (b - a).max(1.) {
                u = candidate;
                break;
            }
            u = candidate;
        }
        let refined = base.evaluate(u)?.point;
        let refined_d2: f64 = (0..3).map(|i| (refined[i] - point[i]).powi(2)).sum();
        // Keep the better of the coarse seed and the refined parameter.
        let (u, c) = if refined_d2 <= best_d2 {
            (u, to3(&refined)?)
        } else {
            (best_u, to3(&base.evaluate(best_u)?.point)?)
        };
        numeric(
            c.iter().all(|v| v.is_finite()),
            "Wire projection produced non-finite data",
        )?;
        Ok((u, c))
    }

    /// Displace one world point by the normalized blend of all wires.
    pub fn deform_point(&self, point: [f64; 3]) -> Result<[f64; 3]> {
        check(
            point.iter().all(|v| v.is_finite()),
            "Wire query point must be finite",
        )?;
        let mut displacement = [0.; 3];
        let mut total_weight = 0.;
        let mut max_weight: f64 = 0.;
        for wire in &self.wires {
            let (u, closest) = Self::project(&wire.base, point)?;
            let target = to3(&wire.target.evaluate(u)?.point)?;
            let delta = sub(target, closest);
            let d2: f64 = (0..3).map(|i| (point[i] - closest[i]).powi(2)).sum();
            let weight = (-d2 / (wire.radius * wire.radius)).exp();
            total_weight += weight;
            max_weight = max_weight.max(weight);
            for axis in 0..3 {
                displacement[axis] += weight * delta[axis];
            }
        }
        // Normalized blending with magnitude capped by the strongest wire:
        // the displacement direction is the weight-normalized blend, while
        // its magnitude follows the maximum wire weight. A single far wire
        // therefore attenuates, and duplicating a wire changes nothing.
        if total_weight < 1e-300 {
            return Ok(point);
        }
        let mut out = point;
        for axis in 0..3 {
            out[axis] += displacement[axis] * max_weight / total_weight;
        }
        numeric(
            out.iter().all(|v| v.is_finite()),
            "Wire deformation produced non-finite data",
        )?;
        Ok(out)
    }

    /// Deform world points in place.
    pub fn apply_to_points(&self, points: &mut [[f64; 3]]) -> Result<()> {
        for point in points.iter_mut() {
            *point = self.deform_point(*point)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clamped_knots(n: usize, p: usize) -> Vec<f64> {
        let spans = n - p;
        let mut knots = vec![0.; p + 1];
        for s in 1..spans {
            knots.push(s as f64 / spans as f64);
        }
        knots.extend(vec![1.; p + 1]);
        knots
    }

    /// Bilinear plane patch on [0,1]² at height z, 5x5 cubic net.
    fn plane_surface(z: f64) -> Surface {
        let (n, p) = (5usize, 3usize);
        let knots = clamped_knots(n, p);
        let g: Vec<f64> = (0..n).map(|i| greville(&knots, p, i)).collect();
        let mut control_points = vec![vec![vec![0.; 3]; n]; n];
        for i in 0..n {
            for j in 0..n {
                control_points[i][j] = vec![g[i], g[j], z];
            }
        }
        Surface {
            degree_u: p,
            degree_v: p,
            knots_u: knots.clone(),
            knots_v: knots,
            control_points,
            weights: vec![vec![1.; n]; n],
            periodic_u: false,
            periodic_v: false,
        }
    }

    fn straight_wire(dx: f64) -> Wire {
        // Linear B-spline along x at y=z=0; the target is lifted by dx in z.
        let base = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let target = Curve {
            control_points: vec![vec![0., 0., dx], vec![1., 0., dx]],
            ..base.clone()
        };
        Wire {
            base,
            target,
            radius: 0.5,
        }
    }

    #[test]
    fn identity_lattice_leaves_geometry_unchanged() {
        let lattice = FfdLattice::new([0., 0., 0.], [1., 1., 1.], [3, 3, 3], [2, 2, 2]).unwrap();
        let points = [
            [0.1, 0.2, 0.3],
            [0.5, 0.5, 0.5],
            [0.99, 0.01, 0.47],
            [0., 0., 0.],
            [1., 1., 1.],
        ];
        for p in points {
            let q = lattice.deform_point(p).unwrap();
            for axis in 0..3 {
                assert!(
                    (q[axis] - p[axis]).abs() < 1e-12,
                    "identity lattice moved {p:?} to {q:?}"
                );
            }
        }
        // Same for a curve and a surface through the lattice.
        let surface = plane_surface(0.4);
        let deformed = lattice.apply_to_surface(&surface).unwrap();
        for i in 0..5 {
            for j in 0..5 {
                for axis in 0..3 {
                    assert!(
                        (deformed.control_points[i][j][axis]
                            - surface.control_points[i][j][axis])
                            .abs()
                            < 1e-12
                    );
                }
            }
        }
    }

    #[test]
    fn ffd_locality_single_control_moves_only_its_support() {
        let mut lattice = FfdLattice::new([0., 0., 0.], [1., 1., 1.], [2, 2, 2], [4, 4, 4]).unwrap();
        // Control (2,2,2) of degree-2 axes with 4 spans: support on each
        // axis is knots[2..5] = [0, 0.75]; move it up.
        let slot = lattice.control_index(2, 2, 2).unwrap();
        let mut position = lattice.control(slot).unwrap();
        position[2] += 0.3;
        lattice.set_control(slot, position).unwrap();
        // Point deep inside the support moves.
        let inside = lattice.deform_point([0.4, 0.4, 0.4]).unwrap();
        assert!((inside[2] - 0.4).abs() > 1e-6);
        // Point at (0.9, 0.9, 0.9): basis of control index 2 on axis with
        // knots {0,0,0,.25,.5,.75,1,1,1} has support [k2, k5) = [0, .75);
        // a parameter of 0.9 is outside on every axis.
        let outside = lattice.deform_point([0.9, 0.9, 0.9]).unwrap();
        for axis in 0..3 {
            assert!(
                (outside[axis] - 0.9).abs() < 1e-12,
                "point outside the support moved: {outside:?}"
            );
        }
    }

    #[test]
    fn twist_rotates_by_the_requested_angle() {
        let mut surface = plane_surface(0.5);
        // Column of control points along z at x=0.5, y=0.
        for (i, row) in surface.control_points.iter_mut().enumerate() {
            for (j, point) in row.iter_mut().enumerate() {
                *point = vec![0.5, 0., (i * 5 + j) as f64 / 24.];
            }
        }
        let angle = std::f64::consts::FRAC_PI_2;
        let twisted = barr_twist(&surface, 2, [0.5, 0., 0.], [0., 1.], angle).unwrap();
        // A point at z = 1 rotated by π/2 about the z-axis through
        // (0.5, 0): offset (0, 0) stays; use a point with y-offset instead.
        let mut surface2 = plane_surface(0.5);
        for (i, row) in surface2.control_points.iter_mut().enumerate() {
            for (j, point) in row.iter_mut().enumerate() {
                *point = vec![0.7, 0., (i * 5 + j) as f64 / 24.];
            }
        }
        let twisted2 = barr_twist(&surface2, 2, [0.5, 0., 0.], [0., 1.], angle).unwrap();
        // z = 0: no rotation.
        assert!((twisted2.control_points[0][0][0] - 0.7).abs() < 1e-12);
        assert!(twisted2.control_points[0][0][1].abs() < 1e-12);
        // z = 1 (last row): offset (0.2, 0) rotates to (0, 0.2).
        let top = &twisted2.control_points[4][4];
        assert!((top[0] - 0.5).abs() < 1e-12, "x={}", top[0]);
        assert!((top[1] - 0.2).abs() < 1e-12, "y={}", top[1]);
        // Untouched original column sanity: x stays 0.5 everywhere.
        for row in &twisted.control_points {
            for point in row {
                assert!((point[0] - 0.5).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn taper_scales_and_corrects_normals() {
        let surface = plane_surface(0.5);
        let (tapered, normals) =
            barr_taper(&surface, 2, [0., 0., 0.], [0., 1.], 2.).unwrap();
        // z = 0.5 (mid band): r = 1.5; x = 1 scales to 1.5.
        assert!((tapered.control_points[4][0][0] - 1.5).abs() < 1e-12);
        // Plane normals (0,0,±1) are eigen-directions of the taper
        // Jacobian transpose: they stay axis-aligned after correction.
        for i in 0..5 {
            for j in 0..5 {
                let n = normals[i][j].unwrap();
                assert!((n[0]).abs() < 1e-9 && (n[1]).abs() < 1e-9);
                assert!((n[2].abs() - 1.).abs() < 1e-9);
            }
        }
        // Transposed-inverse correctness on a tilted normal: build a tilted
        // plane z = x and verify n'·(J t) ≈ 0 for tangent t = (1,0,1)/√2.
        let mut tilted = plane_surface(0.);
        for i in 0..5 {
            for j in 0..5 {
                tilted.control_points[i][j][2] = tilted.control_points[i][j][0];
            }
        }
        let (_, tilted_normals) = barr_taper(&tilted, 0, [0., 0., 0.], [0., 1.], 2.).unwrap();
        // Control (2, 0) sits at x = z = 0.5, strictly inside the band:
        // r = 1.5, dr/dx = 1, so J = [[1,0,0],[0,1.5,0],[0.5,0,1.5]] and
        // the tangent t = (1,0,1) maps to J t = (1, 0, 2).
        let n = tilted_normals[2][0].unwrap();
        let jt = [1., 0., 2.];
        let dot: f64 = (0..3).map(|a| n[a] * jt[a]).sum();
        assert!(dot.abs() < 1e-9, "corrected normal not orthogonal: {dot}");
    }

    #[test]
    fn bend_is_rigid_beyond_the_band() {
        // Grid of control points spread in x (bend_dir) and z (axis).
        let mut surface = plane_surface(0.);
        for (i, row) in surface.control_points.iter_mut().enumerate() {
            for (j, point) in row.iter_mut().enumerate() {
                *point = vec![0.1 * i as f64, 0., 0.25 * j as f64];
            }
        }
        let rate = 1.0;
        let bent = barr_bend(&surface, 2, 0, [0., 0., 0.], [0., 0.5], rate).unwrap();
        // Band origin stays fixed: point (x=0, z=0) → centerline arc start.
        let origin = &bent.control_points[0][0];
        assert!((origin[0]).abs() < 1e-12 && (origin[2]).abs() < 1e-12);
        // Rigidity beyond the band: distances between two points both above
        // z = 0.5 with equal x are preserved (rigid rotation/translation).
        let before = {
            let a = &surface.control_points[2][3];
            let b = &surface.control_points[2][4];
            norm(sub([a[0], a[1], a[2]], [b[0], b[1], b[2]]))
        };
        let after = {
            let a = &bent.control_points[2][3];
            let b = &bent.control_points[2][4];
            norm(sub([a[0], a[1], a[2]], [b[0], b[1], b[2]]))
        };
        assert!((before - after).abs() < 1e-12, "{before} vs {after}");
        // Centerline in the band follows the circle of radius 1/rate:
        // point (0, y, 0.5) → (1−cos(0.5), y, sin(0.5)).
        let tip = &bent.control_points[0][2];
        let phi = 0.25 * 2. * rate; // z = 0.5 → φ = 0.5
        assert!((tip[0] - (1. - phi.cos())).abs() < 1e-9, "x={}", tip[0]);
        assert!((tip[2] - phi.sin()).abs() < 1e-9, "z={}", tip[2]);
    }

    #[test]
    fn wire_with_zero_difference_is_identity() {
        let deformer = WireDeformer::new(vec![straight_wire(0.)]).unwrap();
        let points = [[0.3, 0.1, 0.2], [0.7, -0.4, 0.1], [0.5, 0., 0.]];
        for p in points {
            let q = deformer.deform_point(p).unwrap();
            for axis in 0..3 {
                assert!((q[axis] - p[axis]).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn wire_pulls_points_toward_the_target_curve() {
        let deformer = WireDeformer::new(vec![straight_wire(0.4)]).unwrap();
        // Point on the wire gets (nearly) the full displacement.
        let on = deformer.deform_point([0.5, 0., 0.]).unwrap();
        assert!((on[2] - 0.4).abs() < 1e-9, "z={}", on[2]);
        // Far point is barely affected (d = 2, r = 0.5 → w = e⁻¹⁶).
        let far = deformer.deform_point([0.5, 2., 0.]).unwrap();
        assert!(far[2] < 1e-5, "far point moved by {}", far[2]);
        assert!(far[2] > 0.);
    }

    #[test]
    fn duplicate_wires_do_not_double_displace() {
        let single = WireDeformer::new(vec![straight_wire(0.4)]).unwrap();
        let doubled = WireDeformer::new(vec![straight_wire(0.4), straight_wire(0.4)]).unwrap();
        let p = [0.4, 0.2, 0.1];
        let a = single.deform_point(p).unwrap();
        let b = doubled.deform_point(p).unwrap();
        for axis in 0..3 {
            assert!(
                (a[axis] - b[axis]).abs() < 1e-15,
                "duplicate wires changed the result: {a:?} vs {b:?}"
            );
        }
    }

    #[test]
    fn ffd_cache_canonicalizes_signed_zero() {
        let lattice = FfdLattice::new([0., 0., 0.], [1., 1., 1.], [3, 3, 3], [2, 2, 2]).unwrap();
        // -0.0 и +0.0 — одна каноническая запись кэша.
        let a = lattice.parameterize([0.0, 0.25, 0.75]).unwrap();
        let b = lattice.parameterize([-0.0, 0.25, 0.75]).unwrap();
        assert_eq!(a, b);
        assert_eq!(
            lattice.cache.borrow().len(),
            1,
            "signed zeros must share one cache entry"
        );
        // Различные координаты — разные записи.
        lattice.parameterize([0.5, 0.25, 0.75]).unwrap();
        assert_eq!(lattice.cache.borrow().len(), 2);
    }

    #[test]
    fn ffd_rejects_nan_query_point() {
        let lattice = FfdLattice::new([0., 0., 0.], [1., 1., 1.], [3, 3, 3], [2, 2, 2]).unwrap();
        let err = lattice.parameterize([f64::NAN, 0., 0.]).unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(lattice.deform_point([0.5, f64::INFINITY, 0.5]).is_err());
    }

    #[test]
    fn wire_newton_budget_is_a_named_stage() {
        // Истощение ньютоновского бюджета носит имя стадии.
        let mut guard = Budget::with_iterations(2)
            .unwrap()
            .guard("deform.wire-newton");
        guard.tick().unwrap();
        guard.tick().unwrap();
        let err = guard.tick().unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
        assert!(err.contains("deform.wire-newton"), "{err}");
    }
}
