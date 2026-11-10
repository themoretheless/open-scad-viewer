use super::*;
use crate::graph_body::QualifiedGraphBody;
use nurbs_core::{curve::Curve, surface::Surface};
pub(super) struct Strip {
    pub surface: Surface,
    pub wire: Vec<face::Boundary>,
    pub keys: Vec<u64>,
}
fn oriented(m: &Model, c: &crate::Coedge) -> Result<Curve> {
    if c.reversed {
        m.edges[c.edge].curve.reverse()
    } else {
        Ok(m.edges[c.edge].curve.clone())
    }
}
fn ends(c: &Curve) -> [Vec<f64>; 2] {
    [
        c.control_points[0].clone(),
        c.control_points.last().unwrap().clone(),
    ]
}
fn xy(p: &[f64]) -> [f64; 2] {
    [p[0], p[1]]
}
pub(super) fn build(
    bodies: [&QualifiedGraphBody<'_>; 2],
    upper: usize,
    used: &mut usize,
    max: usize,
) -> Result<Vec<Strip>> {
    let lower = 1 - upper;
    let a = bodies[upper].source();
    let b = bodies[lower].source();
    let ar = &a.loops[a.faces[bodies[upper].roof()].outer].coedges;
    let br = &b.loops[b.faces[bodies[lower].roof()].outer].coedges;
    let mut strips = Vec::new();
    for ac in ar {
        charge(used, max, 1)?;
        let plane = bodies[upper]
            .walls()
            .iter()
            .position(|&f| {
                a.loops[a.faces[f].outer]
                    .coedges
                    .iter()
                    .any(|c| c.edge == ac.edge)
            })
            .ok_or_else(|| unproven("Roof edge lost original footprint wall"))?;
        let wall = bodies[lower].walls()[plane];
        let bc = br
            .iter()
            .find(|c| {
                b.loops[b.faces[wall].outer]
                    .coedges
                    .iter()
                    .any(|w| w.edge == c.edge)
            })
            .ok_or_else(|| unproven("Opposite roof edge lost original wall identity"))?;
        let high = oriented(a, ac)?;
        let mut low = oriented(b, bc)?;
        let hp = ends(&high);
        let mut bp = ends(&low);
        let mut reversed = bc.reversed;
        if xy(&hp[0]) != xy(&bp[0]) {
            low = low.reverse()?;
            reversed = !reversed;
            bp = ends(&low);
        }
        if xy(&hp[0]) != xy(&bp[0]) || xy(&hp[1]) != xy(&bp[1]) {
            return Err(unproven("Original wall corner XY identities differ"));
        }
        let axis = plane / 2;
        let fixed = bodies[upper].footprint()[axis][plane % 2];
        for p in high.control_points.iter().chain(&low.control_points) {
            charge(used, max, 1)?;
            if p[axis] != fixed {
                return Err(unproven(
                    "Original wall rails are not in the exact footprint plane",
                ));
            }
        }
        let surface = Surface::ruled_surface(&low, &high)?;
        for p in surface.control_points.iter().flatten() {
            charge(used, max, 1)?;
            if p[axis] != fixed {
                return Err(unproven("Ruled strip lost the exact source plane"));
            }
        }
        let u0 = surface.knots_u[surface.degree_u];
        let u1 = surface.knots_u[surface.control_points.len()];
        let pc = |a: [f64; 2], b: [f64; 2]| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]);
        let vertices = a.edges[ac.edge].vertices;
        let vertices = if ac.reversed {
            [vertices[1], vertices[0]]
        } else {
            vertices
        };
        strips.push(Strip {
            surface,
            wire: vec![
                face::Boundary {
                    curve: b.edges[bc.edge].curve.clone(),
                    pcurve: pc([u0, 0.], [u1, 0.])?,
                    reversed,
                },
                face::Boundary {
                    curve: Curve::from_polyline(vec![bp[1].clone(), hp[1].clone()])?,
                    pcurve: pc([u1, 0.], [u1, 1.])?,
                    reversed: false,
                },
                face::Boundary {
                    curve: a.edges[ac.edge].curve.clone(),
                    pcurve: pc([u1, 1.], [u0, 1.])?,
                    reversed: !ac.reversed,
                },
                face::Boundary {
                    curve: Curve::from_polyline(vec![bp[0].clone(), hp[0].clone()])?,
                    pcurve: pc([u0, 1.], [u0, 0.])?,
                    reversed: true,
                },
            ],
            keys: vec![
                source_key(lower, bc.edge),
                (3u64 << 60) | vertices[1] as u64,
                source_key(upper, ac.edge),
                (3u64 << 60) | vertices[0] as u64,
            ],
        });
    }
    Ok(strips)
}
