//! Native control-space edits preserve rational definitions. A nonlinear edit of
//! controls is not asserted to equal pointwise deformation of the evaluated surface.
use crate::{Result, curve::Curve, input, surface::Surface};
fn map_curve(c: &Curve, f: impl Fn([f64; 3]) -> Result<[f64; 3]>) -> Result<Curve> {
    c.validate()?;
    let mut out = c.clone();
    for p in &mut out.control_points {
        if p.len() != 3 {
            return Err(input("Control edit requires 3D coordinates"));
        }
        *p = f([p[0], p[1], p[2]])?.to_vec();
    }
    out.validate()?;
    Ok(out)
}
fn map_surface(s: &Surface, f: impl Fn([f64; 3]) -> Result<[f64; 3]>) -> Result<Surface> {
    s.validate()?;
    let mut out = s.clone();
    for p in out.control_points.iter_mut().flatten() {
        *p = f([p[0], p[1], p[2]])?.to_vec();
    }
    out.validate()?;
    Ok(out)
}
pub fn deform_curve(c: &Curve, d: &geometry_ops::Deformation) -> Result<Curve> {
    d.validate()?;
    map_curve(c, |p| d.apply(p))
}
pub fn deform_surface(s: &Surface, d: &geometry_ops::Deformation) -> Result<Surface> {
    d.validate()?;
    map_surface(s, |p| d.apply(p))
}
pub fn brush_curve(c: &Curve, b: &geometry_ops::Brush) -> Result<Curve> {
    b.validate()?;
    map_curve(c, |p| b.apply(p))
}
pub fn brush_surface(s: &Surface, b: &geometry_ops::Brush) -> Result<Surface> {
    b.validate()?;
    map_surface(s, |p| b.apply(p))
}

use geometry_ops::unit_or_zero;
fn point3(p: &[f64]) -> Result<[f64; 3]> {
    if p.len() != 3 {
        return Err(input("Control edit requires 3D coordinates"));
    }
    Ok([p[0], p[1], p[2]])
}
/// Control-polygon sculpt data: neighbors are the previous/next controls
/// (wrapping for periodic curves) and the "normal" is the discrete curvature
/// direction `p - mean(neighbors)`, zero where the polygon is straight.
pub fn curve_sculpt_target(c: &Curve) -> Result<geometry_ops::SculptTarget> {
    c.validate()?;
    let positions = c
        .control_points
        .iter()
        .map(|p| point3(p))
        .collect::<Result<Vec<_>>>()?;
    let n = positions.len();
    let adjacency: Vec<Vec<usize>> = (0..n)
        .map(|i| {
            let mut ring = Vec::new();
            if i > 0 {
                ring.push(i - 1);
            } else if c.periodic {
                ring.push(n - 1);
            }
            if i + 1 < n {
                ring.push(i + 1);
            } else if c.periodic {
                ring.push(0);
            }
            ring
        })
        .collect();
    let normals = (0..n)
        .map(|i| {
            let ring = &adjacency[i];
            if ring.len() < 2 {
                return [0.; 3];
            }
            let mean = math_core::scale(
                ring.iter()
                    .fold([0.; 3], |acc, &j| math_core::add(acc, positions[j])),
                1. / ring.len() as f64,
            );
            unit_or_zero(math_core::sub(positions[i], mean))
        })
        .collect();
    Ok(geometry_ops::SculptTarget {
        positions,
        normals,
        adjacency,
    })
}
/// Control-net sculpt data: 4-neighborhood over the (u, v) grid (wrapping on
/// periodic axes) and normals from central/one-sided differences of the net.
pub fn surface_sculpt_target(s: &Surface) -> Result<geometry_ops::SculptTarget> {
    s.validate()?;
    let nu = s.control_points.len();
    let nv = s.control_points[0].len();
    let idx = |i: usize, j: usize| i * nv + j;
    let positions = s
        .control_points
        .iter()
        .flatten()
        .map(|p| point3(p))
        .collect::<Result<Vec<_>>>()?;
    let step = |i: usize, n: usize, periodic: bool, dir: isize| -> Option<usize> {
        let k = i as isize + dir;
        if k < 0 || k >= n as isize {
            periodic.then(|| k.rem_euclid(n as isize) as usize)
        } else {
            Some(k as usize)
        }
    };
    let mut adjacency = vec![Vec::new(); nu * nv];
    let mut normals = vec![[0.; 3]; nu * nv];
    for i in 0..nu {
        for j in 0..nv {
            let (ip, im) = (step(i, nu, s.periodic_u, 1), step(i, nu, s.periodic_u, -1));
            let (jp, jm) = (step(j, nv, s.periodic_v, 1), step(j, nv, s.periodic_v, -1));
            let here = positions[idx(i, j)];
            let du = math_core::sub(
                ip.map_or(here, |k| positions[idx(k, j)]),
                im.map_or(here, |k| positions[idx(k, j)]),
            );
            let dv = math_core::sub(
                jp.map_or(here, |k| positions[idx(i, k)]),
                jm.map_or(here, |k| positions[idx(i, k)]),
            );
            normals[idx(i, j)] = unit_or_zero(math_core::cross(du, dv));
            for k in [im, ip].into_iter().flatten() {
                adjacency[idx(i, j)].push(idx(k, j));
            }
            for k in [jm, jp].into_iter().flatten() {
                adjacency[idx(i, j)].push(idx(i, k));
            }
        }
    }
    Ok(geometry_ops::SculptTarget {
        positions,
        normals,
        adjacency,
    })
}
pub fn sculpt_curve(c: &Curve, b: &geometry_ops::SculptBrush) -> Result<Curve> {
    let moved = curve_sculpt_target(c)?.sculpt(b)?;
    let mut out = c.clone();
    out.control_points = moved.into_iter().map(|p| p.to_vec()).collect();
    out.validate()?;
    Ok(out)
}
pub fn sculpt_surface(s: &Surface, b: &geometry_ops::SculptBrush) -> Result<Surface> {
    let moved = surface_sculpt_target(s)?.sculpt(b)?;
    let nv = s.control_points[0].len();
    let mut out = s.clone();
    for (k, p) in moved.into_iter().enumerate() {
        out.control_points[k / nv][k % nv] = p.to_vec();
    }
    out.validate()?;
    Ok(out)
}
