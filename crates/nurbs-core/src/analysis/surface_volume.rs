//! Certified enclosed volume of a closed set of NURBS patches.
//!
//! By the divergence theorem the volume bounded by a closed, consistently
//! oriented surface is `V = 1/3 Σ ∫∫ S·(S_u × S_v) du dv` over every patch.
//! Each active knot rectangle is enclosed with the original rational jets of
//! [`super::surface_measure`]: the integrand range over the cell, intersected
//! with a tensor trapezoid rule whose remainder is bounded by the interval
//! second derivatives `f_uu`, `f_vv` (Peano kernel `1/12` per axis). Cells are
//! split into quarters adaptively, largest uncertainty first, until the
//! requested absolute tolerance (cubic model units) is certified or the
//! budget ends.
//!
//! The result is signed: positive when the patch normals `S_u × S_v` point
//! outward. Closure and consistent orientation are the caller's contract and
//! are not verified here; trimmed faces are not supported.
use super::surface_measure::{StopReason, jets};
use crate::{
    Result, check,
    distance_bounds::Interval,
    numeric,
    numerics::interval_vec3::{add, cross, dot, scale},
    resource,
    surface::Surface,
};
use std::{cmp::Ordering, collections::BinaryHeap};

#[derive(Clone, Debug)]
pub struct VolumeReport {
    /// Midpoint of the certified enclosure.
    pub value: f64,
    /// Outward-rounded enclosure of the exact signed volume.
    pub bounds: [f64; 2],
    /// `|exact - value| <= error_upper`.
    pub error_upper: f64,
    pub within_tolerance: bool,
    /// Constructed cell enclosures, including replaced parents.
    pub cells: usize,
    pub stop_reason: StopReason,
}

struct Cell {
    patch: usize,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
    flux: Interval,
    serial: usize,
}
impl Cell {
    fn gap(&self) -> f64 {
        self.flux.hi - self.flux.lo
    }
}
impl PartialEq for Cell {
    fn eq(&self, b: &Self) -> bool {
        self.gap() == b.gap() && self.serial == b.serial
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
        self.gap().total_cmp(&b.gap()).then(self.serial.cmp(&b.serial))
    }
}

type V = [Interval; 3];
fn lin(terms: &[(V, V, f64)]) -> Result<V> {
    let mut out = [Interval::point(0.); 3];
    for &(a, b, c) in terms {
        out = add(out, scale(cross(a, b)?, Interval::point(c))?)?;
    }
    Ok(out)
}
/// Jets are relative to the span's first control point; restore absolute
/// position because the divergence integrand depends on it.
fn position(s: &Surface, span: [usize; 2], j: &jets::Jets) -> Result<V> {
    let origin = &s.control_points[span[0] - s.degree_u][span[1] - s.degree_v];
    add(j[0][0], std::array::from_fn(|k| Interval::point(origin[k])))
}

fn cell(s: &Surface, patch: usize, span: [usize; 2], domain: [[f64; 2]; 2], serial: usize) -> Result<Cell> {
    // Local unit-square coordinates: the jets carry the chain-rule factors, so
    // the cell flux equals the unit-square integral of f = S·(S_u × S_v).
    let j = jets::calculate(s, span, domain, None)?;
    let p = position(s, span, &j)?;
    let n = cross(j[1][0], j[0][1])?;
    let f = dot(p, n)?;
    let (mut lo, mut hi) = (f.lo, f.hi);
    let nu = lin(&[(j[2][0], j[0][1], 1.), (j[1][0], j[1][1], 1.)])?;
    let nv = lin(&[(j[1][1], j[0][1], 1.), (j[1][0], j[0][2], 1.)])?;
    let nuu = lin(&[(j[3][0], j[0][1], 1.), (j[2][0], j[1][1], 2.), (j[1][0], j[2][1], 1.)])?;
    let nvv = lin(&[(j[1][2], j[0][1], 1.), (j[1][1], j[0][2], 2.), (j[1][0], j[0][3], 1.)])?;
    // f_uu = S_uu·N + 2 S_u·N_u + S·N_uu, likewise along v.
    let two = Interval::point(2.);
    let fuu = dot(j[2][0], n)?.add(dot(j[1][0], nu)?.mul(two)?)?.add(dot(p, nuu)?)?;
    let fvv = dot(j[0][2], n)?.add(dot(j[0][1], nv)?.mul(two)?)?.add(dot(p, nvv)?)?;
    let mag = |x: Interval| x.lo.abs().max(x.hi.abs());
    let remainder = ((mag(fuu) + mag(fvv)) / 12.).next_up();
    if remainder.is_finite() {
        let mut corners = Interval::point(0.);
        for u in 0..2 {
            for v in 0..2 {
                let c = jets::calculate(s, span, domain, Some([u, v]))?;
                corners = corners.add(dot(position(s, span, &c)?, cross(c[1][0], c[0][1])?)?)?;
            }
        }
        let t = corners.mul(Interval::point(0.25))?;
        lo = lo.max((t.lo - remainder).next_down());
        hi = hi.min((t.hi + remainder).next_up());
    }
    numeric(lo <= hi, "Volume cell enclosures are inconsistent")?;
    Ok(Cell { patch, span, domain, flux: Interval::new(lo, hi)?, serial })
}

fn sum(heap: &BinaryHeap<Cell>) -> Result<Interval> {
    heap.iter().try_fold(Interval::point(0.), |t, c| t.add(c.flux))
}
fn report(flux: Interval, cells: usize, tolerance: f64, reason: StopReason) -> Result<VolumeReport> {
    let total = Interval::new((flux.lo / 3.).next_down(), (flux.hi / 3.).next_up())?;
    let value = total.lo + (total.hi - total.lo) / 2.;
    let error_upper = (value - total.lo).max(total.hi - value).next_up();
    numeric(value.is_finite() && error_upper.is_finite(), "Enclosed volume exhausted numeric range")?;
    let within_tolerance = error_upper <= tolerance;
    Ok(VolumeReport {
        value,
        bounds: [total.lo, total.hi],
        error_upper,
        within_tolerance,
        cells,
        stop_reason: if within_tolerance { StopReason::Tolerance } else { reason },
    })
}

/// Enclose the signed volume bounded by `surfaces`. A budget stop returns a
/// valid enclosure with `within_tolerance == false`.
pub fn enclosed_volume(surfaces: &[Surface], tolerance: f64, max_cells: usize) -> Result<VolumeReport> {
    check(!surfaces.is_empty(), "Enclosed volume needs at least one patch")?;
    check(tolerance.is_finite() && tolerance > 0., "Volume tolerance must be finite and positive")?;
    check((1..=200_000).contains(&max_cells), "Volume work budget must be 1..200000 cell enclosures")?;
    let mut heap = BinaryHeap::new();
    let mut cells = 0;
    for (patch, s) in surfaces.iter().enumerate() {
        s.validate()?;
        for u in s.degree_u..s.control_points.len() {
            for v in s.degree_v..s.control_points[0].len() {
                if s.knots_u[u] < s.knots_u[u + 1] && s.knots_v[v] < s.knots_v[v + 1] {
                    if cells == max_cells {
                        return Err(resource("Initial volume knot rectangles exceed the work budget"));
                    }
                    let domain = [[s.knots_u[u], s.knots_u[u + 1]], [s.knots_v[v], s.knots_v[v + 1]]];
                    heap.push(cell(s, patch, [u, v], domain, cells)?);
                    cells += 1;
                }
            }
        }
    }
    loop {
        let r = report(sum(&heap)?, cells, tolerance, StopReason::WorkLimit)?;
        if r.within_tolerance || cells + 4 > max_cells {
            return Ok(r);
        }
        let parent = heap.pop().unwrap();
        let s = &surfaces[parent.patch];
        let [[u0, u1], [v0, v1]] = parent.domain;
        let (um, vm) = (u0 + (u1 - u0) / 2., v0 + (v1 - v0) / 2.);
        if !(u0 < um && um < u1 && v0 < vm && vm < v1) {
            heap.push(parent);
            return report(sum(&heap)?, cells, tolerance, StopReason::PrecisionLimit);
        }
        for d in [[[u0, um], [v0, vm]], [[um, u1], [v0, vm]], [[u0, um], [vm, v1]], [[um, u1], [vm, v1]]] {
            heap.push(cell(s, parent.patch, parent.span, d, cells)?);
            cells += 1;
        }
    }
}

/// Six outward biquadratic flat patches bounding `[0,a]×[0,b]×[0,c]`; shared
/// edges carry identical control points, so the shell is closed.
#[cfg(test)]
pub(crate) fn test_cuboid(a: f64, b: f64, c: f64) -> Vec<Surface> {
    let flat = |c: [[f64; 3]; 4]| {
        let lerp = |a: [f64; 3], b: [f64; 3], t: f64| -> [f64; 3] { std::array::from_fn(|k| a[k] + (b[k] - a[k]) * t) };
        let control_points = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let (s, t) = (i as f64 / 2., j as f64 / 2.);
                        lerp(lerp(c[0], c[1], s), lerp(c[2], c[3], s), t).to_vec()
                    })
                    .collect()
            })
            .collect();
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points,
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    };
    let p = |x: f64, y: f64, z: f64| [x * a, y * b, z * c];
    // Corner order (u0v0, u1v0, u0v1, u1v1); S_u × S_v points outward.
    [
        [p(0., 0., 0.), p(0., 1., 0.), p(1., 0., 0.), p(1., 1., 0.)],
        [p(0., 0., 1.), p(1., 0., 1.), p(0., 1., 1.), p(1., 1., 1.)],
        [p(0., 0., 0.), p(1., 0., 0.), p(0., 0., 1.), p(1., 0., 1.)],
        [p(0., 1., 0.), p(0., 1., 1.), p(1., 1., 0.), p(1., 1., 1.)],
        [p(0., 0., 0.), p(0., 0., 1.), p(0., 1., 0.), p(0., 1., 1.)],
        [p(1., 0., 0.), p(1., 1., 0.), p(1., 0., 1.), p(1., 1., 1.)],
    ]
    .into_iter()
    .map(flat)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{curve_measure, surface_measure};
    use std::f64::consts::PI;

    #[test]
    fn cuboid_volume_is_enclosed_and_translation_invariant() {
        let r = enclosed_volume(&test_cuboid(1., 2., 3.), 1e-9, 1000).unwrap();
        assert!(r.within_tolerance && r.bounds[0] <= 6. && 6. <= r.bounds[1], "{r:?}");
        let moved: Vec<_> = test_cuboid(1., 2., 3.)
            .into_iter()
            .map(|mut s| {
                s.control_points.iter_mut().flatten().for_each(|p| p[0] += 5.);
                s
            })
            .collect();
        let r = enclosed_volume(&moved, 1e-9, 1000).unwrap();
        assert!(r.within_tolerance && r.bounds[0] <= 6. && 6. <= r.bounds[1], "{r:?}");
    }

    #[test]
    fn rational_sphere_volume_and_area_enclose_analytic_values() {
        let r = 1.5;
        let sphere = crate::primitives::sphere([0.3, -0.2, 0.1], r).unwrap();
        let exact = 4. / 3. * PI * r * r * r;
        let v = enclosed_volume(std::slice::from_ref(&sphere), 2e-2, 20_000).unwrap();
        assert!(v.within_tolerance, "{v:?}");
        let signed = exact.copysign(v.value);
        assert!(v.bounds[0] <= signed && signed <= v.bounds[1], "{v:?} vs {exact}");
        let area = surface_measure::area(&sphere, 1e-2, 20_000).unwrap();
        let exact = 4. * PI * r * r;
        // Pole cells have a zero Jacobian, so the area enclosure tightens only
        // linearly there; the enclosure must still hold.
        assert!(area.error_upper < 0.2 && area.bounds[0] <= exact && exact <= area.bounds[1], "{area:?}");
    }

    #[test]
    fn rational_circle_length_encloses_analytic_value() {
        let circle = crate::primitives::circle([0.; 3], [0., 0., 1.], 2.5).unwrap();
        let l = curve_measure::length(&circle, 1e-7, 10_000).unwrap();
        let exact = 5. * PI;
        assert!(l.within_tolerance && l.bounds[0] <= exact && exact <= l.bounds[1], "{l:?}");
        assert!((l.value - exact).abs() <= l.error_upper);
    }

    #[test]
    fn rejects_bad_inputs_and_keeps_enclosure_on_budget_stop() {
        assert!(enclosed_volume(&[], 1e-3, 100).is_err());
        assert!(enclosed_volume(&test_cuboid(1., 1., 1.), 0., 100).is_err());
        let sphere = crate::primitives::sphere([0.; 3], 1.).unwrap();
        let r = enclosed_volume(&[sphere], 1e-12, 64).unwrap();
        assert!(!r.within_tolerance && r.stop_reason == StopReason::WorkLimit);
        let exact = (4. / 3. * PI).copysign(r.value);
        assert!(r.bounds[0] <= exact && exact <= r.bounds[1], "{r:?}");
    }
}
