//! Exact projection of a lifted trimmed face into its native periodic chart.
use crate::{Curve, Error, Result, Surface};
use num_rational::BigRational as Q;
use num_traits::{ToPrimitive, Zero};
mod wrapped;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Projection {
    Translated,
    Clipped,
    Wrapped,
}

fn natural(s: &Surface) -> Result<[[Q; 2]; 2]> {
    s.validate()?;
    Ok([
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
    .map(|range| range.map(|x| Q::from_float(x).unwrap())))
}

pub(super) fn verify_lifts(s: &Surface, active: &[Curve], authored: [[i32; 2]; 2]) -> Result<()> {
    let endpoints = [
        &active[0].control_points[0],
        active.last().unwrap().control_points.last().unwrap(),
    ];
    let domain = natural(s)?;
    let periodic = [s.periodic_u, s.periodic_v];
    for (point, lifts) in endpoints.into_iter().zip(authored) {
        for axis in 0..2 {
            let [lo, hi] = &domain[axis];
            let expected = if periodic[axis] {
                ((Q::from_float(point[axis]).unwrap() - lo) / (hi - lo))
                    .floor()
                    .to_integer()
                    .to_i32()
            } else {
                Some(0)
            };
            if expected != Some(lifts[axis]) {
                return Err(Error::new(
                    "BREP_TRIM_PERIODIC_LIFT",
                    "Restricted pcurve endpoints disagree with their authored lifts",
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn project(s: &Surface, loops: &[Vec<Curve>]) -> Result<(Vec<Vec<Curve>>, Projection)> {
    let domain = natural(s)?;
    for curve in loops.iter().flatten() {
        curve.validate()?;
        if curve.control_points.iter().any(|p| p.len() != 2) {
            return Err(Error::new(
                "BREP_TRIM_DOMAIN",
                "Periodic chart translation needs 2D curves",
            ));
        }
    }
    let periodic = [s.periodic_u, s.periodic_v];
    let mut open = false;
    for wire in loops {
        open |= nurbs_core::trim_domain::exact_loop_joins(wire)? == Some(false);
    }
    if open {
        return Ok((
            wrapped::belt(s, loops, &domain, periodic)?,
            Projection::Wrapped,
        ));
    }
    let linear = loops.iter().flatten().all(|c| {
        c.degree == 1 && !c.periodic && c.control_points.len() == 2 && c.weights[0] == c.weights[1]
    });
    let mut shifts = [vec![Q::zero()], vec![Q::zero()]];
    let mut needs_overlap_check = false;
    for axis in 0..2 {
        let [lo, hi] = &domain[axis];
        let mut values = loops
            .iter()
            .flatten()
            .flat_map(|curve| &curve.control_points)
            .map(|p| Q::from_float(p[axis]).unwrap());
        let first = values
            .next()
            .ok_or_else(|| Error::new("BREP_TRIM_DOMAIN", "Empty lifted UV face"))?;
        let (min, max) = values.fold((first.clone(), first), |(min, max), value| {
            (min.min(value.clone()), max.max(value))
        });
        let period = hi - lo;
        if periodic[axis] {
            shifts[axis][0] = ((&min - lo) / &period).floor() * &period;
        }
        let shifted_range = [&min - &shifts[axis][0], &max - &shifts[axis][0]];
        if shifted_range[0] < *lo || shifted_range[1] > *hi {
            let wide = &max - &min >= period;
            if !periodic[axis] {
                return Err(Error::new(
                    "BREP_TRIM_DOMAIN",
                    "Restricted UV face exceeds supported periodic coverage",
                ));
            }
            needs_overlap_check |= wide;
            let count = ((&shifted_range[1] - lo) / &period)
                .ceil()
                .to_integer()
                .to_usize()
                .filter(|&n| (2..=4).contains(&n))
                .ok_or_else(|| {
                    Error::new(
                        "BREP_RESOURCE_LIMIT",
                        "Periodic trim exceeds its chart-copy budget",
                    )
                })?;
            let first = shifts[axis][0].clone();
            shifts[axis] = (0..count)
                .map(|k| &first + &period * Q::from_integer(k.into()))
                .collect();
        }
    }
    if shifts.iter().all(|axis| axis.len() == 1) {
        let mut translated = loops.to_vec();
        for p in translated
            .iter_mut()
            .flatten()
            .flat_map(|c| &mut c.control_points)
        {
            for axis in 0..2 {
                p[axis] = representable(Q::from_float(p[axis]).unwrap() - &shifts[axis][0])?;
            }
        }
        return Ok((translated, Projection::Translated));
    }
    let clipped = if linear {
        clip_linear_copies(loops, domain, shifts, needs_overlap_check)?
    } else {
        clip_curved_copies(loops, domain, shifts, needs_overlap_check)?
    };
    Ok((clipped, Projection::Clipped))
}

fn clip_curved_copies(
    loops: &[Vec<Curve>],
    domain: [[Q; 2]; 2],
    shifts: [Vec<Q>; 2],
    needs_overlap_check: bool,
) -> Result<Vec<Vec<Curve>>> {
    if loops
        .iter()
        .any(|wire| nurbs_core::trim_domain::exact_loop_joins(wire).ok() != Some(Some(true)))
    {
        return Err(Error::new(
            "BREP_TRIM_DOMAIN",
            "Cross-seam projection needs exact UV joins",
        ));
    }
    let corners = [(0, 0), (1, 0), (1, 1), (0, 1)].map(|(i, j)| {
        Ok(vec![
            representable(domain[0][i].clone())?,
            representable(domain[1][j].clone())?,
        ])
    });
    let corners = corners.into_iter().collect::<Result<Vec<_>>>()?;
    let region = [(0..4)
        .map(|i| crate::line(corners[i].clone(), corners[(i + 1) % 4].clone()))
        .collect()];
    let mut covered: Vec<Vec<Vec<Curve>>> = Vec::new();
    for u in 0..shifts[0].len() {
        for v in 0..shifts[1].len() {
            let mut translated = loops.to_vec();
            for p in translated
                .iter_mut()
                .flatten()
                .flat_map(|c| &mut c.control_points)
            {
                for (axis, slot) in [u, v].into_iter().enumerate() {
                    p[axis] = representable(Q::from_float(p[axis]).unwrap() - &shifts[axis][slot])?;
                }
            }
            let patch =
                crate::model::planar_pcurve_regions::intersect_planar_pcurves([&translated, &region], 4096)?;
            if patch.is_empty() {
                continue;
            }
            if needs_overlap_check {
                for previous in &covered {
                    if !crate::model::planar_pcurve_regions::intersect_planar_pcurves(
                        [previous, &patch],
                        4096,
                    )?
                    .is_empty()
                    {
                        return Err(Error::new(
                            "BREP_TRIM_DOMAIN",
                            "Translated periodic trim copies overlap in their interiors",
                        ));
                    }
                }
            }
            covered.push(patch);
        }
    }
    if covered.is_empty() {
        return Err(Error::new(
            "BREP_TRIM_DOMAIN",
            "Empty periodic trim after chart projection",
        ));
    }
    Ok(covered.into_iter().flatten().collect())
}

fn representable(v: Q) -> Result<f64> {
    v.to_f64()
        .filter(|&f| Q::from_float(f).as_ref() == Some(&v))
        .ok_or_else(|| {
            Error::new(
                "BREP_TRIM_DOMAIN",
                "Periodic chart coordinate is not exactly representable",
            )
        })
}

fn clip_linear_copies(
    loops: &[Vec<Curve>],
    domain: [[Q; 2]; 2],
    shifts: [Vec<Q>; 2],
    needs_overlap_check: bool,
) -> Result<Vec<Vec<Curve>>> {
    type Ring = Vec<[Q; 2]>;
    let mut rings = Vec::<Ring>::new();
    for (index, wire) in loops.iter().enumerate() {
        if !wire.iter().all(|c| {
            c.degree == 1
                && !c.periodic
                && c.control_points.len() == 2
                && c.weights[0] == c.weights[1]
        }) || nurbs_core::trim_domain::exact_loop_joins(wire)? != Some(true)
        {
            return Err(Error::new(
                "BREP_TRIM_DOMAIN",
                "Cross-seam projection requires exact straight UV loops",
            ));
        }
        let ring: Ring = wire
            .iter()
            .map(|c| [0, 1].map(|k| Q::from_float(c.control_points[0][k]).unwrap()))
            .collect();
        let area: Q = ring
            .iter()
            .zip(ring.iter().cycle().skip(1))
            .map(|(a, b)| &a[0] * &b[1] - &a[1] * &b[0])
            .sum();
        if area.is_zero() || (area > Q::zero()) != (index == 0) {
            return Err(Error::new(
                "BREP_TRIM_DOMAIN",
                "Cross-seam projection needs material-left UV loops",
            ));
        }
        rings.push(ring);
    }
    let rect = vec![
        [(0, 0), (1, 0), (1, 1), (0, 1)]
            .map(|(i, j)| [domain[0][i].clone(), domain[1][j].clone()])
            .to_vec(),
    ];
    let mut clipped = Vec::new();
    let mut covered: Vec<Vec<Ring>> = Vec::new();
    for u in &shifts[0] {
        for v in &shifts[1] {
            let translated = rings
                .iter()
                .map(|ring| ring.iter().map(|p| [&p[0] - u, &p[1] - v]).collect())
                .collect();
            let groups =
                crate::profile_certificate::intersection_rings(translated, rect.clone(), 100_000)?;
            let patch: Vec<_> = groups.into_iter().flatten().collect();
            if patch.is_empty() {
                continue;
            }
            if needs_overlap_check {
                for previous in &covered {
                    if !crate::profile_certificate::intersection_rings(
                        previous.clone(),
                        patch.clone(),
                        100_000,
                    )?
                    .is_empty()
                    {
                        return Err(Error::new(
                            "BREP_TRIM_DOMAIN",
                            "Translated periodic trim copies overlap in their interiors",
                        ));
                    }
                }
                covered.push(patch.clone());
            }
            for ring in patch {
                let points: Vec<Vec<f64>> = ring
                    .into_iter()
                    .map(|p| p.into_iter().map(representable).collect())
                    .collect::<Result<_>>()?;
                clipped.push(
                    points
                        .iter()
                        .zip(points.iter().cycle().skip(1))
                        .map(|(a, b)| crate::line(a.clone(), b.clone()))
                        .collect(),
                );
            }
        }
    }
    if clipped.is_empty() {
        return Err(Error::new(
            "BREP_TRIM_DOMAIN",
            "Empty periodic trim after chart projection",
        ));
    }
    Ok(clipped)
}
