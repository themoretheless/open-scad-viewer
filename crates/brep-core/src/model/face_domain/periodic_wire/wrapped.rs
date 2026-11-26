//! A pair of oppositely oriented once-around paths bounds a periodic belt.
use super::*;
use crate::model::planar_pcurve_regions::{
    planar_boundaries_are_disjoint, reverse_exact, subtract_planar_pcurves,
};

fn refuse(message: &str) -> Error {
    Error::new("BREP_TRIM_DOMAIN", message)
}

fn endpoint(curve: &Curve, end: bool) -> Vec<f64> {
    curve.control_points[if end {
        curve.control_points.len() - 1
    } else {
        0
    }]
    .clone()
}

fn translate(wire: &[Curve], domain: &[[Q; 2]; 2], axis: usize, shift: &Q) -> Result<Vec<Curve>> {
    let mut translated = wire.to_vec();
    for p in translated.iter_mut().flat_map(|c| &mut c.control_points) {
        for k in 0..2 {
            let mut x = Q::from_float(p[k]).unwrap();
            if k == axis {
                x -= shift;
            }
            if x < domain[k][0] || x > domain[k][1] {
                return Err(refuse("Wrapped UV contour leaves its chart copy"));
            }
            p[k] = representable(x)?;
        }
    }
    Ok(translated)
}

fn rotate_at_seam(
    wire: &[Curve],
    domain: &[[Q; 2]; 2],
    axis: usize,
    shift: &Q,
    forward: bool,
) -> Result<Vec<Curve>> {
    let [lo, hi] = &domain[axis];
    let seam = hi + shift;
    let point = |end: usize| -> Result<Vec<f64>> {
        let mut p = vec![0.; 2];
        p[axis] = representable(seam.clone())?;
        p[1 - axis] = representable(domain[1 - axis][end].clone())?;
        Ok(p)
    };
    let divider = crate::line(point(0)?, point(1)?);
    let mut before = Vec::new();
    let mut after = Vec::new();
    let is_before = |p: &[f64]| {
        let q = Q::from_float(p[axis]).unwrap();
        if forward { q <= seam } else { q >= seam }
    };
    for curve in wire {
        let start = endpoint(curve, false);
        let end = endpoint(curve, true);
        if is_before(&end) {
            before.push(curve.clone());
        } else if !is_before(&start) || Q::from_float(start[axis]).unwrap() == seam {
            after.push(curve.clone());
        } else {
            let [left, right] = crate::model::planar_pcurve_regions::split_pcurve_on_line(curve, &divider)?;
            before.extend(left);
            after.extend(right);
        }
    }
    let upper_shift = shift + (hi - lo);
    let (before_shift, after_shift) = if forward {
        (shift, &upper_shift)
    } else {
        (&upper_shift, shift)
    };
    let mut rotated = translate(&after, domain, axis, after_shift)?;
    rotated.extend(translate(&before, domain, axis, before_shift)?);
    if rotated
        .windows(2)
        .any(|pair| endpoint(&pair[0], true) != endpoint(&pair[1], false))
    {
        return Err(refuse("Rotated wrapped UV path has an inexact join"));
    }
    Ok(rotated)
}

fn lift_path(wire: &[Curve], domain: &[[Q; 2]; 2], axis: usize) -> Result<(Vec<Curve>, bool)> {
    if wire.is_empty()
        || wire.iter().any(|c| {
            let [a, b] = c.domain();
            c.periodic
                || c.knots[..=c.degree].iter().any(|&k| k != a)
                || c.knots[c.control_points.len()..].iter().any(|&k| k != b)
        })
        || wire
            .windows(2)
            .any(|pair| endpoint(&pair[0], true) != endpoint(&pair[1], false))
    {
        return Err(refuse(
            "Wrapped UV path needs exact internal joins and clamped curves",
        ));
    }
    let first = endpoint(&wire[0], false);
    let last = endpoint(wire.last().unwrap(), true);
    let [lo, hi] = &domain[axis];
    let period = hi - lo;
    let q = |v| Q::from_float(v).unwrap();
    let delta = q(last[axis]) - q(first[axis]);
    let forward = delta == period;
    if (!forward && delta != -&period) || first[1 - axis] != last[1 - axis] {
        return Err(refuse(
            "Wrapped UV path needs one exact period of displacement",
        ));
    }
    // A monotone control sequence makes each path a graph across the
    // fundamental period. Audit below proves the two graphs are disjoint.
    let mut poles = wire.iter().flat_map(|c| &c.control_points).map(|p| p[axis]);
    let mut previous = poles.next().unwrap();
    for next in poles {
        if (forward && previous > next) || (!forward && previous < next) {
            return Err(refuse("Wrapped UV path is not monotone along the period"));
        }
        previous = next;
    }
    let offset = (q(first[axis].min(last[axis])) - lo) / &period;
    let shift = offset.floor() * &period;
    let translated = if offset.is_integer() {
        translate(wire, domain, axis, &shift)?
    } else {
        rotate_at_seam(wire, domain, axis, &shift, forward)?
    };
    Ok((translated, forward))
}

pub(super) fn belt(
    surface: &Surface,
    loops: &[Vec<Curve>],
    domain: &[[Q; 2]; 2],
    periodic: [bool; 2],
) -> Result<Vec<Vec<Curve>>> {
    if periodic[0] == periodic[1] {
        return Err(refuse("Wrapped UV belt needs a single periodic axis"));
    }
    let axis = usize::from(periodic[1]);
    let mut paths = Vec::new();
    let mut holes = Vec::new();
    let mut seam_holes = Vec::new();
    for wire in loops {
        match nurbs_core::trim_domain::exact_loop_joins(wire)? {
            Some(false) => paths.push(wire),
            Some(true) => {
                let positive = reverse_wire(wire)?;
                audit(std::slice::from_ref(&positive))?;
                let (projected, kind) = project(surface, &[positive])?;
                if kind == Projection::Translated {
                    holes.push(reverse_wire(&projected[0])?);
                } else {
                    seam_holes.push(projected);
                }
            }
            None => return Err(refuse("Wrapped UV belt has unresolved contour closure")),
        }
    }
    let [a, b] = paths.as_slice() else {
        return Err(refuse("Wrapped UV belt needs two boundary paths"));
    };
    let (mut a, forward_a) = lift_path(a, domain, axis)?;
    let (b, forward_b) = lift_path(b, domain, axis)?;
    if forward_a == forward_b {
        return Err(refuse(
            "Wrapped UV belt paths must traverse opposite directions",
        ));
    }
    let start_a = endpoint(&a[0], false);
    let end_a = endpoint(a.last().unwrap(), true);
    let start_b = endpoint(&b[0], false);
    let end_b = endpoint(b.last().unwrap(), true);
    if start_a == end_b || start_b == end_a {
        return Err(refuse("Wrapped UV belt has collapsed seam closure"));
    }
    let authored = [a.clone(), b.clone()];
    a.push(crate::line(end_a, start_b));
    a.extend(b);
    a.push(crate::line(end_b, start_a));
    let mut previous = holes.clone();
    let mut loops: Vec<_> = std::iter::once(a).chain(holes).collect();
    audit(&loops)?;
    for hole in seam_holes {
        if !planar_boundaries_are_disjoint([&authored, &hole], 4096)?
            || !planar_boundaries_are_disjoint([&previous, &hole], 4096)?
            || !subtract_planar_pcurves([&hole, &loops], 4096)?.is_empty()
        {
            return Err(refuse(
                "Seam-crossing hole must lie strictly inside the periodic belt",
            ));
        }
        loops = subtract_planar_pcurves([&loops, &hole], 4096)?;
        previous.extend(hole);
    }
    audit(&loops)?;
    Ok(loops)
}

fn reverse_wire(wire: &[Curve]) -> Result<Vec<Curve>> {
    wire.iter().rev().cloned().map(reverse_exact).collect()
}

fn audit(loops: &[Vec<Curve>]) -> Result<()> {
    let report = nurbs_core::trim_region_audit::inspect(loops, 1e-9, 4096, 4096, 32768)?;
    if report.valid != Some(true)
        || report.winding.first() != Some(&Some(1))
        || report.winding[1..].iter().any(|w| *w != Some(-1))
    {
        return Err(refuse(
            "Wrapped UV belt has unproved or incorrectly oriented material",
        ));
    }
    Ok(())
}
