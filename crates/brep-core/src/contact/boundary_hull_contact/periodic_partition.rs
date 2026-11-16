//! Paired hole and patch trims on one injective periodic carrier.
use super::{Certificate, Model};
use num_rational::BigRational as Q;

pub(super) fn certify(model: &Model, faces: [usize; 2]) -> Option<Certificate> {
    for [wall, patch] in [faces, [faces[1], faces[0]]] {
        let wall = &model.faces[wall];
        let patch = &model.faces[patch];
        let surface = &wall.surface;
        if surface != &patch.surface || !patch.holes.is_empty() || wall.holes.is_empty() {
            continue;
        }
        let axis = match (surface.periodic_u, surface.periodic_v) {
            (true, false) => 0,
            (false, true) => 1,
            _ => continue,
        };
        let domain = crate::boolean_robustness::surface_uv_domains(surface);
        let transverse = 1 - axis;
        let patch_wire = &model.loops[patch.outer].coedges;
        if !(3..=256).contains(&patch_wire.len()) {
            continue;
        }
        let literal = |c: &crate::Coedge| {
            super::full_use(model, c)
                && !c.pcurve.periodic
                && c.pcurve.degree == 1
                && c.pcurve.control_points.len() == 2
                && c.pcurve.control_points.iter().all(|p| {
                    domain[transverse][0] < p[transverse] && p[transverse] < domain[transverse][1]
                })
        };
        if !patch_wire.iter().all(literal)
            || !std::iter::once(wall.outer)
                .chain(wall.holes.iter().copied())
                .flat_map(|wire| &model.loops[wire].coedges)
                .all(|c| {
                    super::full_use(model, c)
                        && c.pcurve.control_points.iter().all(|p| {
                            domain[transverse][0] <= p[transverse]
                                && p[transverse] <= domain[transverse][1]
                        })
                })
        {
            continue;
        }
        let [min, max] = patch_wire
            .iter()
            .flat_map(|c| &c.pcurve.control_points)
            .map(|p| p[axis])
            .fold([f64::INFINITY, f64::NEG_INFINITY], |[lo, hi], x| {
                [lo.min(x), hi.max(x)]
            });
        let q = |x| Q::from_float(x).unwrap();
        if q(max) - q(min) >= q(domain[axis][1]) - q(domain[axis][0]) {
            continue;
        }
        let matched = wall.holes.iter().any(|&hole| {
            let hole = &model.loops[hole].coedges;
            if hole.len() != patch_wire.len() || !hole.iter().all(literal) {
                return false;
            }
            let Some(start) = hole.iter().position(|c| c.edge == patch_wire[0].edge) else {
                return false;
            };
            patch_wire.iter().enumerate().all(|(i, a)| {
                let b = &hole[(start + hole.len() - i) % hole.len()];
                a.edge == b.edge
                    && a.reversed != b.reversed
                    && a.pcurve.control_points[0] == b.pcurve.control_points[1]
                    && a.pcurve.control_points[1] == b.pcurve.control_points[0]
            })
        });
        if !matched {
            continue;
        }
        let proof =
            nurbs_core::surface_injectivity::certify_periodic_extrusion(surface, axis, 256, 4096)
                .ok()?;
        if !proof.proven || !proof.regular_interior {
            continue;
        }
        let edges = patch_wire.iter().map(|c| c.edge).collect::<Vec<_>>();
        let mut contact_enclosure = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
        for point in edges
            .iter()
            .flat_map(|&e| &model.edges[e].curve.control_points)
        {
            for (bounds, &coordinate) in contact_enclosure.iter_mut().zip(point) {
                bounds[0] = bounds[0].min(coordinate);
                bounds[1] = bounds[1].max(coordinate);
            }
        }
        return Some(Certificate {
            faces,
            contact_enclosure,
            edges,
            vertex: None,
            joined_proof: None,
            polar_join: None,
            source_join: None,
        });
    }
    None
}
