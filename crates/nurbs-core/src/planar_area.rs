//! Outward interval Green area of a closed rational XY curve chain.
//! This integral does not establish simplicity or material roles.
use crate::{Result, check, curve::Curve, curve_jets, distance_bounds::Interval, trim_domain};
use std::{cmp::Ordering, collections::BinaryHeap};
#[derive(Debug)]
pub struct Report {
    pub area_interval_mm2: [f64; 2],
    pub converged: bool,
    pub cells: usize,
    pub reason: &'static str,
}
struct Cell {
    curve: usize,
    span: usize,
    domain: [f64; 2],
    area: Interval,
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
        (self.area.hi - self.area.lo).total_cmp(&(b.area.hi - b.area.lo))
    }
}
fn cross(a: &[Interval], b: &[Interval]) -> Result<Interval> {
    a[0].mul(b[1])?.sub(a[1].mul(b[0])?)
}
fn cell(c: &Curve, span: usize, domain: [f64; 2], origin: [f64; 2]) -> Result<Interval> {
    let shift = |mut jet: Vec<Vec<Interval>>| -> Result<Vec<Vec<Interval>>> {
        for k in 0..2 {
            jet[0][k] = jet[0][k].sub(Interval::point(origin[k]))?;
        }
        Ok(jet)
    };
    let left = shift(curve_jets::endpoint(c, span, domain, false)?)?;
    let right = shift(curve_jets::endpoint(c, span, domain, true)?)?;
    let jets = shift(curve_jets::enclose(c, span, domain)?)?;
    // Jets use the local cell parameter in [0,1]. Thus no source knot width
    // multiplier is needed. The trapezoid remainder is bounded by sup|F''|/12.
    let trapezoid = cross(&left[0], &left[1])?
        .add(cross(&right[0], &right[1])?)?
        .mul(Interval::point(0.25))?;
    let second = cross(&jets[1], &jets[2])?
        .add(cross(&jets[0], &jets[3])?)?
        .mul(Interval::point(0.5))?;
    let error = (second.lo.abs().max(second.hi.abs()) / 12.).next_up();
    Interval::new(
        (trapezoid.lo - error).next_down(),
        (trapezoid.hi + error).next_up(),
    )
}
pub fn measure(curves: &[Curve], tolerance_mm2: f64, max_cells: usize) -> Result<Report> {
    check(
        (2..=256).contains(&curves.len())
            && tolerance_mm2.is_finite()
            && tolerance_mm2 > 0.
            && (1..=100000).contains(&max_cells),
        "Planar area requires a closed XY chain, positive area tolerance and bounded work",
    )?;
    check(
        trim_domain::exact_loop_joins(curves)? == Some(true),
        "Planar area requires proven exact closed joins",
    )?;
    let origin = [
        curves[0].control_points[0][0],
        curves[0].control_points[0][1],
    ];
    let mut heap = BinaryHeap::new();
    let (mut lo, mut hi, mut cells) = (0_f64, 0_f64, 0_usize);
    let span_count = curves
        .iter()
        .map(|c| {
            (c.degree..c.control_points.len())
                .filter(|&i| c.knots[i] < c.knots[i + 1])
                .count()
        })
        .sum::<usize>();
    check(
        span_count <= max_cells,
        "Area work budget cannot cover all source spans",
    )?;
    for (index, c) in curves.iter().enumerate() {
        for span in c.degree..c.control_points.len() {
            if c.knots[span] >= c.knots[span + 1] {
                continue;
            }
            let domain = [c.knots[span], c.knots[span + 1]];
            let area = cell(c, span, domain, origin)?;
            cells += 1;
            lo = (lo + area.lo).next_down();
            hi = (hi + area.hi).next_up();
            heap.push(Cell {
                curve: index,
                span,
                domain,
                area,
            });
        }
    }
    let mut reason = "work-limit";
    while (hi - lo).next_up() > tolerance_mm2 && cells + 2 <= max_cells {
        let old = heap.pop().unwrap();
        let mid = old.domain[0] * 0.5 + old.domain[1] * 0.5;
        if mid <= old.domain[0] || mid >= old.domain[1] {
            heap.push(old);
            reason = "parameter-precision";
            break;
        }
        lo = (lo - old.area.lo).next_down();
        hi = (hi - old.area.hi).next_up();
        for domain in [[old.domain[0], mid], [mid, old.domain[1]]] {
            let area = cell(&curves[old.curve], old.span, domain, origin)?;
            cells += 1;
            lo = (lo + area.lo).next_down();
            hi = (hi + area.hi).next_up();
            heap.push(Cell {
                curve: old.curve,
                span: old.span,
                domain,
                area,
            });
        }
    }
    let converged = (hi - lo).next_up() <= tolerance_mm2;
    Ok(Report {
        area_interval_mm2: [lo, hi],
        converged,
        cells,
        reason: if converged { "area-tolerance" } else { reason },
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn chain() -> Vec<Curve> {
        let mut c = vec![Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., -1.], vec![2., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        }];
        for (a, b) in [
            ([2., 0.], [2., 2.]),
            ([2., 2.], [0., 2.]),
            ([0., 2.], [0., 0.]),
        ] {
            c.push(Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap());
        }
        c
    }
    #[test]
    fn rational_circle_and_multispan_parameterization_enclose_area() {
        let points = [
            [[2., 0.], [2., 2.], [0., 2.]],
            [[0., 2.], [-2., 2.], [-2., 0.]],
            [[-2., 0.], [-2., -2.], [0., -2.]],
            [[0., -2.], [2., -2.], [2., 0.]],
        ];
        let circle = points
            .iter()
            .map(|p| Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: p.iter().map(|v| v.to_vec()).collect(),
                weights: vec![1., 0.5_f64.sqrt(), 1.],
                periodic: false,
            })
            .collect::<Vec<_>>();
        for split in [false, true] {
            let input = if split {
                circle
                    .iter()
                    .map(|c| c.insert(0.5, 1))
                    .collect::<Result<Vec<_>>>()
                    .unwrap()
            } else {
                circle.clone()
            };
            let r = measure(&input, 1e-4, 10000).unwrap();
            let expected = 4. * std::f64::consts::PI;
            assert!(r.converged, "{r:?}");
            assert!(
                r.area_interval_mm2[0] <= expected && r.area_interval_mm2[1] >= expected,
                "{r:?}"
            );
        }
    }
    #[test]
    fn knot_domain_change_and_coordinate_scale_preserve_units() {
        let mut input = chain();
        for curve in &mut input {
            for knot in &mut curve.knots {
                *knot = 7. + 13. * *knot;
            }
            for point in &mut curve.control_points {
                point[0] *= 3.;
                point[1] *= 3.;
            }
        }
        let r = measure(&input, 1e-5, 10000).unwrap();
        assert!(r.converged, "{r:?}");
        assert!(
            r.area_interval_mm2[0] <= 42. && r.area_interval_mm2[1] >= 42.,
            "{r:?}"
        );
    }
    #[test]
    fn signed_integral_does_not_claim_simple_material() {
        let points = [[0., 0.], [2., 2.], [0., 2.], [2., 0.]];
        let input = (0..4)
            .map(|i| {
                Curve::from_polyline(vec![points[i].to_vec(), points[(i + 1) % 4].to_vec()])
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let r = measure(&input, 1e-6, 10000).unwrap();
        assert!(r.converged);
        assert!(r.area_interval_mm2[0] <= 0. && r.area_interval_mm2[1] >= 0.);
        assert!(
            !crate::trim_simplicity::inspect(&input, 1e-8, 1000, 10000)
                .unwrap()
                .proven_simple
        );
    }
    #[test]
    fn open_chain_and_invalid_budget_are_rejected() {
        let mut c = chain();
        c[0].control_points[0][0] = 0.01;
        assert!(measure(&c, 1e-6, 10000).is_err());
        assert!(measure(&chain(), 0., 10000).is_err());
        assert!(measure(&chain(), 1e-6, 1).is_err());
    }
    #[test]
    fn polynomial_area_reverse_translation_and_work_limit() {
        let c = chain();
        let before = format!("{c:?}");
        for reverse in [false, true] {
            let mut input = if reverse {
                c.iter()
                    .rev()
                    .map(Curve::reverse)
                    .collect::<Result<Vec<_>>>()
                    .unwrap()
            } else {
                c.clone()
            };
            for curve in &mut input {
                for p in &mut curve.control_points {
                    p[0] += 13.;
                    p[1] -= 7.;
                }
            }
            let r = measure(&input, 1e-5, 10000).unwrap();
            let expected = if reverse { -14. / 3. } else { 14. / 3. };
            assert!(r.converged, "{r:?}");
            assert!(r.area_interval_mm2[0] <= expected && r.area_interval_mm2[1] >= expected);
            assert!(r.cells <= 10000);
        }
        assert!(!measure(&c, 1e-12, 4).unwrap().converged);
        assert_eq!(format!("{c:?}"), before);
    }
}
