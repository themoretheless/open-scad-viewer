//! Metric contact locality at an owned edge of an original trimmed rectangle.
//! Not exact contact ownership, exact absence, or global solid embedding.
use crate::{Model, face_domain::FaceDomain};
use nurbs_core::{
    Error, Result, interval_eval::Interval as I, surface_coordinate_flow, surface_g2_pcurves,
};
pub struct Certificate {
    pub faces: [usize; 2],
    pub edge: usize,
    pub trimmed_face: usize,
    pub coordinate: usize,
    pub boundary_axis: usize,
    pub boundary_end: usize,
    pub distance_upper: f64,
    pub hull_penetration_upper: f64,
}
pub struct Attempt {
    pub certificate: Option<Certificate>,
    pub cells: usize,
    pub exact_work: u64,
}
/// Fresh whole-interval proof. Positive control hulls bound the other face
/// relative to the owned edge's coordinate plane, including actual penetration.
/// Strict coordinate flow on
/// the ORIGINAL trimmed support bounds every possible contact back to that
/// edge, with both actual saved edge lifts independently verified.
pub fn inspect(
    model: &Model,
    faces: [usize; 2],
    max_cells: usize,
    max_exact: u64,
) -> Result<Attempt> {
    model.validate_boundary_diagnostic_inputs()?;
    if faces[0] == faces[1]
        || faces.iter().any(|&f| f >= model.faces.len())
        || max_cells > 100000
        || max_exact > 1000000
    {
        return Err(Error::new(
            "BREP_TRIMMED_CONTACT_LIMITS",
            "Invalid pair or shared contact-locality work",
        ));
    }
    if faces.iter().any(|&f| {
        let s = &model.faces[f].surface;
        s.control_points
            .len()
            .checked_mul(s.control_points[0].len())
            .is_none_or(|n| n > 4096)
    }) {
        return Err(Error::new(
            "BREP_TRIMMED_CONTACT_LIMITS",
            "Original contact surface control budget exceeded",
        ));
    }
    let mut out = Attempt {
        certificate: None,
        cells: 0,
        exact_work: 0,
    };
    for [a, b] in [faces, [faces[1], faces[0]]] {
        if out.cells == max_cells {
            break;
        }
        let domain = FaceDomain::new(model, a, 1e-8)?;
        let Some(rect) = domain.literal_rectangle() else {
            continue;
        };
        let source = &model.faces[a];
        let other = &model.faces[b];
        let s = &other.surface;
        let natural = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        let other_uses = std::iter::once(other.outer)
            .chain(other.holes.iter().copied())
            .flat_map(|w| model.loops[w].coedges.iter())
            .collect::<Vec<_>>();
        if other_uses.iter().any(|c| {
            c.pcurve
                .control_points
                .iter()
                .any(|p| (0..2).any(|k| p[k] < natural[k][0] || p[k] > natural[k][1]))
        }) {
            continue;
        }
        for c in &model.loops[source.outer].coedges {
            let uses = other_uses
                .iter()
                .filter(|u| u.edge == c.edge)
                .collect::<Vec<_>>();
            if uses.len() != 1 {
                continue;
            }
            let pc = &c.pcurve;
            let n = pc.control_points.len();
            let d = pc.domain();
            if pc.periodic
                || pc.degree != 1
                || n != 2
                || pc.weights[0] != pc.weights[1]
                || pc.knots[..=pc.degree].iter().any(|&k| k != d[0])
                || pc.knots[n..].iter().any(|&k| k != d[1])
            {
                continue;
            }
            for axis in 0..2 {
                for end in 0..2 {
                    let at = rect[axis][end];
                    let free = 1 - axis;
                    let endpoints = [pc.control_points[0][free], pc.control_points[1][free]];
                    if pc.control_points.iter().any(|p| p[axis] != at)
                        || (endpoints != rect[free] && endpoints != [rect[free][1], rect[free][0]])
                    {
                        continue;
                    }
                    for coordinate in 0..3 {
                        if out.cells == max_cells {
                            return Ok(out);
                        }
                        out.cells += 1;
                        let edge = &model.edges[c.edge].curve;
                        let value = edge.control_points[0][coordinate];
                        if edge.control_points.iter().any(|p| p[coordinate] != value) {
                            continue;
                        }
                        let flow = surface_coordinate_flow::inspect(
                            &source.surface,
                            axis,
                            coordinate,
                            max_cells - out.cells,
                        )?;
                        out.cells += flow.cells;
                        let (Some(direction), Some(factor)) =
                            (flow.direction, flow.inverse_distance_factor)
                        else {
                            continue;
                        };
                        if !flow.complete {
                            continue;
                        }
                        let positive = (direction > 0) == (end == 1);
                        let low = other
                            .surface
                            .control_points
                            .iter()
                            .flatten()
                            .map(|p| p[coordinate])
                            .fold(f64::INFINITY, f64::min);
                        let high = other
                            .surface
                            .control_points
                            .iter()
                            .flatten()
                            .map(|p| p[coordinate])
                            .fold(f64::NEG_INFINITY, f64::max);
                        let penetration = if positive && low < value {
                            I::point(value).sub(I::point(low))?.hi
                        } else if !positive && high > value {
                            I::point(high).sub(I::point(value))?.hi
                        } else {
                            0.
                        };
                        if I::point(factor).mul(I::point(penetration))?.lo > model.tolerance_mm {
                            continue;
                        }
                        if out.cells == max_cells {
                            return Ok(out);
                        }
                        let positions = surface_g2_pcurves::verify_positions(
                            edge,
                            [pc, &uses[0].pcurve],
                            [&source.surface, &other.surface],
                            [c.reversed, uses[0].reversed],
                            model.tolerance_mm * 0.25,
                            max_cells - out.cells,
                            max_exact - out.exact_work,
                        )?;
                        out.cells += positions.cells;
                        out.exact_work += positions.exact_work;
                        if !positions.proven {
                            continue;
                        }
                        let delta = I::point(positions.upper.unwrap());
                        let upper = delta
                            .add(I::point(penetration))?
                            .mul(I::point(factor))?
                            .add(delta)?
                            .hi;
                        if upper > model.tolerance_mm {
                            continue;
                        }
                        let mut valid = true;
                        for face in [a, b] {
                            if max_cells - out.cells < 2 {
                                valid = false;
                                break;
                            }
                            let f = &model.faces[face];
                            let loops = std::iter::once(f.outer)
                                .chain(f.holes.iter().copied())
                                .map(|w| {
                                    model.loops[w]
                                        .coedges
                                        .iter()
                                        .map(|c| c.pcurve.clone())
                                        .collect()
                                })
                                .collect::<Vec<Vec<_>>>();
                            let stage = (max_cells - out.cells) / 2;
                            let r = nurbs_core::trim_region_audit::inspect(
                                &loops, 1e-8, 10000, stage, stage,
                            )?;
                            out.cells += r.cells + r.domain_cells;
                            if r.valid != Some(true) {
                                valid = false;
                                break;
                            }
                        }
                        if !valid {
                            continue;
                        }
                        out.certificate = Some(Certificate {
                            faces,
                            edge: c.edge,
                            trimmed_face: a,
                            coordinate,
                            boundary_axis: axis,
                            boundary_end: end,
                            distance_upper: upper,
                            hull_penetration_upper: penetration,
                        });
                        return Ok(out);
                    }
                }
            }
        }
    }
    Ok(out)
}
