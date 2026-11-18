//! Original oriented flux bounds, separate from global embedded-volume authority.
use super::*;
use nurbs_core::interval_eval::Interval as I;
use std::{cmp::Ordering, collections::BinaryHeap};
#[derive(Clone, Copy)]
pub struct FluxLimits {
    pub axis: usize,
    pub origin: f64,
    pub absolute_error: f64,
    pub cells: usize,
    pub spans: usize,
    pub planar_cells: usize,
}
pub struct FluxReport {
    pub signed_flux_mm3: Option<[f64; 2]>,
    pub converged: bool,
    pub cells: usize,
    pub spans: usize,
    pub planar_cells: usize,
    pub reason: &'static str,
}
struct Cell {
    face: usize,
    uv: [[f64; 2]; 2],
    mask: bool,
    bound: I,
}
impl PartialEq for Cell {
    fn eq(&self, b: &Self) -> bool {
        self.cmp(b) == Ordering::Equal
    }
}
impl Eq for Cell {}
impl PartialOrd for Cell {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Ord for Cell {
    fn cmp(&self, b: &Self) -> Ordering {
        (self.bound.hi - self.bound.lo).total_cmp(&(b.bound.hi - b.bound.lo))
    }
}
fn contribution(
    m: &Model,
    face: usize,
    uv: [[f64; 2]; 2],
    mask: bool,
    l: FluxLimits,
    work: &mut FluxReport,
) -> Result<Option<I>> {
    if work.cells == l.cells || work.spans == l.spans {
        return Ok(None);
    }
    work.cells += 1;
    let r = nurbs_core::surface_flux::bound(
        &m.faces[face].surface,
        uv,
        l.axis,
        l.origin,
        (l.spans - work.spans).min(100000),
    )?;
    work.spans += r.spans;
    let Some(b) = r.bound else {
        return Ok(None);
    };
    let mut value = if mask {
        let area = I::point(uv[0][1])
            .sub(I::point(uv[0][0]))?
            .mul(I::point(uv[1][1]).sub(I::point(uv[1][0]))?)?;
        I::new(b.density[0].min(0.), b.density[1].max(0.))?.mul(area)?
    } else {
        I::new(b.integral[0], b.integral[1])?
    };
    if m.shells[0]
        .faces
        .iter()
        .find(|u| u.face == face)
        .unwrap()
        .reversed
    {
        value = value.mul(I::point(-1.))?;
    }
    Ok(Some(value))
}
/// A four-sided authored trim with two literal vertical sides and two affine
/// cross-sections encloses one inner rectangle and lies in one outer rectangle.
/// The two remainder bands retain their entire possible signed contribution.
fn trapezoid(m: &Model, face: usize) -> Result<Option<Vec<([[f64; 2]; 2], bool)>>> {
    let f = &m.faces[face];
    if !f.holes.is_empty() {
        return Ok(None);
    }
    let pc = &m.loops[f.outer].coedges;
    if pc.len() != 4 {
        return Ok(None);
    }
    let mut vertical = Vec::new();
    let mut cross = Vec::new();
    for c in pc {
        let p = &c.pcurve;
        if p.control_points
            .iter()
            .all(|v| v[0] == p.control_points[0][0])
        {
            let first = &p.control_points[0];
            let last = p.control_points.last().unwrap();
            if p.control_points
                .iter()
                .any(|v| v[1] < first[1].min(last[1]) || v[1] > first[1].max(last[1]))
            {
                return Ok(None);
            }
            vertical.push(first[0]);
        } else if p.degree == 1 && p.control_points.len() == 2 && p.weights[0] == p.weights[1] {
            cross.push(p);
        } else {
            return Ok(None);
        }
    }
    if vertical.len() != 2 || cross.len() != 2 || vertical[0] == vertical[1] {
        return Ok(None);
    }
    let u = [vertical[0].min(vertical[1]), vertical[0].max(vertical[1])];
    let at = |p: &Curve, end: usize| -> Option<f64> {
        p.control_points
            .iter()
            .find(|p| p[0] == u[end])
            .map(|p| p[1])
    };
    let (Some(a0), Some(a1), Some(b0), Some(b1)) = (
        at(cross[0], 0),
        at(cross[0], 1),
        at(cross[1], 0),
        at(cross[1], 1),
    ) else {
        return Ok(None);
    };
    let (low, high) = if a0 < b0 && a1 < b1 {
        ([a0, a1], [b0, b1])
    } else if b0 < a0 && b1 < a1 {
        ([b0, b1], [a0, a1])
    } else {
        return Ok(None);
    };
    let inner = [low[0].max(low[1]), high[0].min(high[1])];
    if inner[0] >= inner[1] {
        return Ok(None);
    }
    let mut out = vec![([u, inner], false)];
    let outer = [low[0].min(low[1]), high[0].max(high[1])];
    if outer[0] < inner[0] {
        out.push(([u, [outer[0], inner[0]]], true));
    }
    if inner[1] < outer[1] {
        out.push(([u, [inner[1], outer[1]]], true));
    }
    Ok(Some(out))
}
pub(super) fn inspect(m: &Model, tol: f64, l: FluxLimits) -> Result<FluxReport> {
    require(
        l.axis < 3
            && l.origin.is_finite()
            && l.absolute_error.is_finite()
            && l.absolute_error > 0.
            && (1..=100000).contains(&l.cells)
            && (1..=100000).contains(&l.spans)
            && (1..=100000).contains(&l.planar_cells),
        "Bound original corner flux work",
    )?;
    let mut out = FluxReport {
        signed_flux_mm3: None,
        converged: false,
        cells: 0,
        spans: 0,
        planar_cells: 0,
        reason: "original-flux-unproven",
    };
    let mut fixed = I::point(0.);
    let mut heap = BinaryHeap::new();
    for (face, f) in m.faces.iter().enumerate() {
        if let Ok(frame) = planar::frame(&f.surface) {
            let axis = 3 - frame[0].0 - frame[1].0;
            if axis != l.axis || f.surface.control_points[0][0][axis] == l.origin {
                continue;
            }
            let mut chains = Vec::new();
            for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                chains.push(
                    m.loops[wire]
                        .coedges
                        .iter()
                        .map(|c| c.pcurve.clone())
                        .collect::<Vec<_>>(),
                );
            }
            let normal_sign = if (frame[0].0 + 1) % 3 == frame[1].0 {
                1.
            } else {
                -1.
            };
            let mut scale = I::point(f.surface.control_points[0][0][axis])
                .sub(I::point(l.origin))?
                .mul(I::point(frame[0].2))?
                .mul(I::point(frame[1].2))?
                .div(I::point(f.surface.knots_u[2]).sub(I::point(f.surface.knots_u[1]))?)?
                .div(I::point(f.surface.knots_v[2]).sub(I::point(f.surface.knots_v[1]))?)?
                .mul(I::point(normal_sign))?;
            if m.shells[0]
                .faces
                .iter()
                .find(|u| u.face == face)
                .unwrap()
                .reversed
            {
                scale = scale.mul(I::point(-1.))?;
            }
            for curves in chains {
                if out.planar_cells == l.planar_cells {
                    out.reason = "planar-flux-work-exhausted";
                    return Ok(out);
                }
                let error = l.absolute_error
                    / (4. * m.faces.len() as f64 * scale.lo.abs().max(scale.hi.abs()).max(1.));
                let r = nurbs_core::planar_area::measure(
                    &curves,
                    error,
                    l.planar_cells - out.planar_cells,
                )?;
                out.planar_cells += r.cells;
                fixed = fixed
                    .add(I::new(r.area_interval_mm2[0], r.area_interval_mm2[1])?.mul(scale)?)?;
            }
            continue;
        }
        let d = crate::face_domain::FaceDomain::new(m, face, tol)?;
        let s = &f.surface;
        let natural = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        let rectangles = if d.contains_closed_rectangle(natural) {
            vec![(natural, false)]
        } else if let Some(r) = trapezoid(m, face)? {
            r
        } else {
            out.reason = "unsupported-original-flux-trim";
            return Ok(out);
        };
        for (rectangle, mask) in rectangles {
            let mut cut = [Vec::new(), Vec::new()];
            for k in 0..2 {
                let knots = if k == 0 { &s.knots_u } else { &s.knots_v };
                cut[k].push(rectangle[k][0]);
                cut[k].extend(
                    knots
                        .iter()
                        .copied()
                        .filter(|&x| rectangle[k][0] < x && x < rectangle[k][1]),
                );
                cut[k].push(rectangle[k][1]);
                cut[k].sort_by(f64::total_cmp);
                cut[k].dedup();
            }
            for u in cut[0].windows(2) {
                for v in cut[1].windows(2) {
                    let uv = [[u[0], u[1]], [v[0], v[1]]];
                    let Some(bound) = contribution(m, face, uv, mask, l, &mut out)? else {
                        out.reason = "flux-initial-cover-work-exhausted";
                        return Ok(out);
                    };
                    heap.push(Cell {
                        face,
                        uv,
                        mask,
                        bound,
                    });
                }
            }
        }
    }
    loop {
        let total = heap.iter().try_fold(fixed, |sum, c| sum.add(c.bound))?;
        out.signed_flux_mm3 = Some([total.lo, total.hi]);
        if I::point(total.hi).sub(I::point(total.lo))?.hi <= l.absolute_error {
            out.converged = true;
            out.reason = "original-oriented-flux-enclosed";
            return Ok(out);
        }
        if l.cells - out.cells < 2 || l.spans - out.spans < 2 {
            out.reason = "flux-work-exhausted";
            return Ok(out);
        }
        let Some(old) = heap.pop() else {
            out.reason = "planar-area-error-retained";
            return Ok(out);
        };
        let uv = old.uv;
        let mid = [(uv[0][0] + uv[0][1]) * 0.5, (uv[1][0] + uv[1][1]) * 0.5];
        // Samples propose only a split direction. They never replace a bound,
        // and a missing derivative at a C0 knot cannot invalidate an enclosure.
        let density = |u: f64, v: f64| -> Option<f64> {
            let e = m.faces[old.face].surface.evaluate(u, v).ok()?;
            let (a, b) = e.first_derivatives()?;
            let k = (l.axis + 1) % 3;
            let j = (l.axis + 2) % 3;
            Some((e.point[l.axis] - l.origin) * (a[k] * b[j] - a[j] * b[k]))
        };
        let proposed = (|| {
            let center = density(mid[0], mid[1])?;
            let du = (density(uv[0][0] * 0.75 + uv[0][1] * 0.25, mid[1])? - 2. * center
                + density(uv[0][0] * 0.25 + uv[0][1] * 0.75, mid[1])?)
            .abs();
            let dv = (density(mid[0], uv[1][0] * 0.75 + uv[1][1] * 0.25)? - 2. * center
                + density(mid[0], uv[1][0] * 0.25 + uv[1][1] * 0.75)?)
            .abs();
            (du > 0. || dv > 0.).then_some(usize::from(dv > du))
        })();
        let axis =
            proposed.unwrap_or_else(|| usize::from(uv[1][1] - uv[1][0] > uv[0][1] - uv[0][0]));
        let mid = old.uv[axis][0] * 0.5 + old.uv[axis][1] * 0.5;
        if !(old.uv[axis][0] < mid && mid < old.uv[axis][1]) {
            heap.push(old);
            out.reason = "flux-parameter-precision";
            return Ok(out);
        }
        let mut children = Vec::new();
        for side in 0..2 {
            let mut uv = old.uv;
            if side == 0 {
                uv[axis][1] = mid;
            } else {
                uv[axis][0] = mid;
            }
            if let Some(bound) = contribution(m, old.face, uv, old.mask, l, &mut out)? {
                children.push(Cell {
                    face: old.face,
                    uv,
                    mask: old.mask,
                    bound,
                });
            }
        }
        if children.len() == 2 {
            heap.extend(children);
        } else {
            heap.push(old);
            out.reason = "flux-child-work-exhausted-parent-retained";
            return Ok(out);
        }
    }
}
