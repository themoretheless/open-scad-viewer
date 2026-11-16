//! Relative material classification for an oriented boundary on a UV quotient.
use super::*;

/// A certified change of material on the supplied UV path. Absolute material
/// comes from the caller's independently established reference classification.
pub struct QualifiedRootPeriodicFacePath {
    closure: QualifiedRootPeriodicUvClosure,
    paths: Vec<ss::QualifiedOriginalSsPathParity>,
}

impl QualifiedRootPeriodicFacePath {
    pub fn odd_crossings(&self) -> bool {
        self.paths
            .iter()
            .fold(false, |odd, path| odd ^ path.odd_crossings())
    }

    pub fn material_at_end(&self, material_at_start: bool) -> bool {
        material_at_start ^ self.odd_crossings()
    }

    pub fn reverify(&self) -> Result<()> {
        self.closure.reverify()?;
        for path in &self.paths {
            path.reverify()?;
        }
        Ok(())
    }
}

/// Absolute material classification on a singly periodic face's UV cylinder.
pub struct QualifiedRootPeriodicFaceMembership {
    path: QualifiedRootPeriodicFacePath,
}

impl QualifiedRootPeriodicFaceMembership {
    pub fn inside(&self) -> bool {
        self.path.material_at_end(false)
    }

    pub fn reverify(&self) -> Result<()> {
        self.path.reverify()
    }
}

/// Uniform material on an entire quotient UV rectangle.
pub struct QualifiedRootPeriodicFaceMembershipBox {
    source: Arc<RootBrep>,
    face: usize,
    query: [[f64; 2]; 2],
    limits: ss::OriginalSsPathLimits,
    membership: QualifiedRootPeriodicFaceMembership,
}

impl QualifiedRootPeriodicFaceMembershipBox {
    pub fn inside(&self) -> bool {
        self.membership.inside()
    }

    pub fn reverify(&self) -> Result<()> {
        let fresh =
            self.source
                .qualify_periodic_face_membership_box(self.face, self.query, self.limits)?;
        if fresh.inside() != self.inside() {
            return Err(unproved("Periodic box material changed"));
        }
        Ok(())
    }
}

impl RootBrep {
    /// Exclude every periodic image of every boundary arc from a closed UV
    /// rectangle before extending the material at its center to the whole box.
    pub fn qualify_periodic_face_membership_box(
        self: &Arc<Self>,
        face_id: usize,
        query: [[f64; 2]; 2],
        limits: ss::OriginalSsPathLimits,
    ) -> Result<QualifiedRootPeriodicFaceMembershipBox> {
        if query
            .iter()
            .any(|[lo, hi]| !lo.is_finite() || !hi.is_finite() || lo > hi)
            || !(1..=16384).contains(&limits.max_segments)
            || !(1..=65536).contains(&limits.max_periodic_copies)
        {
            return Err(unproved(
                "Invalid periodic membership box or work allowance",
            ));
        }
        let t = self.topology();
        let face = t
            .faces
            .get(face_id)
            .ok_or_else(|| unproved("Missing periodic membership face"))?;
        let periodic = [face.surface.periodic_u, face.surface.periodic_v];
        let natural = crate::boolean_robustness::surface_uv_domains(&face.surface);
        if (0..2).any(|axis| {
            !periodic[axis]
                && (query[axis][0] < natural[axis][0] || query[axis][1] > natural[axis][1])
        }) {
            return Err(unproved(
                "Periodic membership box leaves the finite native support",
            ));
        }
        let center = query.map(|[lo, hi]| lo * 0.5 + hi * 0.5);
        let membership = self.qualify_periodic_face_membership(face_id, center, limits)?;
        let periods = winding::face_periods(&face.surface);
        let mut geometry = limits.geometry;
        let mut segments = limits.max_segments;
        let mut copies = limits.max_periodic_copies;
        for loop_id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for coedge in &t.loops[loop_id].coedges {
                let edge = &t.edges[coedge.edge].curve;
                let bounds = edge.parameter_bounds();
                let domain = edge.curve().parameter_domain();
                let mut pending = vec![[
                    winding::outward(&bounds[0], true)?.max(domain[0]),
                    winding::outward(&bounds[1], false)?.min(domain[1]),
                ]];
                while let Some(range) = pending.pop() {
                    segments = segments
                        .checked_sub(1)
                        .ok_or_else(|| unproved("Periodic membership box subdivision exhausted"))?;
                    let uv = winding::project(
                        edge.curve(),
                        range,
                        coedge.pcurve.support,
                        &mut geometry,
                    )?;
                    let images =
                        face_region::overlapping_period_offsets(uv, query, &periods, periodic)?;
                    let mut overlaps = false;
                    face_region::visit_period_offsets(images, |offset| {
                        copies = copies.checked_sub(1).ok_or_else(|| {
                            unproved("Periodic membership box image allowance exhausted")
                        })?;
                        let shift = std::array::from_fn(|axis| {
                            &periods[axis]
                                * num_rational::BigRational::from_integer(offset[axis].into())
                        });
                        let image = winding::translate(uv, &shift)?;
                        if (0..2).all(|axis| {
                            query[axis][0] <= image[axis][0] && image[axis][1] <= query[axis][1]
                        }) {
                            return Err(unproved(
                                "Periodic membership box contacts a boundary arc",
                            ));
                        }
                        overlaps |= (0..2).all(|axis| {
                            query[axis][0] <= image[axis][1] && image[axis][0] <= query[axis][1]
                        });
                        Ok(())
                    })?;
                    if overlaps {
                        let mid = range[0] * 0.5 + range[1] * 0.5;
                        if !(range[0] < mid && mid < range[1]) {
                            return Err(unproved(
                                "Periodic membership box boundary separation unresolved",
                            ));
                        }
                        pending.extend([[mid, range[1]], [range[0], mid]]);
                    }
                }
            }
        }
        Ok(QualifiedRootPeriodicFaceMembershipBox {
            source: self.clone(),
            face: face_id,
            query,
            limits,
            membership,
        })
    }

    /// Classify a finite UV point by a path from outside the original
    /// nonperiodic parameter interval. Every original boundary arc lies
    /// within that interval, so this exterior reference is independent of
    /// the candidate face orientation. Work limits apply per contour and path.
    pub fn qualify_periodic_face_membership(
        self: &Arc<Self>,
        face_id: usize,
        point: [f64; 2],
        limits: ss::OriginalSsPathLimits,
    ) -> Result<QualifiedRootPeriodicFaceMembership> {
        let surface = &self
            .topology()
            .faces
            .get(face_id)
            .ok_or_else(|| unproved("Missing periodic membership face"))?
            .surface;
        let transverse = match (surface.periodic_u, surface.periodic_v) {
            (true, false) => 1,
            (false, true) => 0,
            _ => {
                return Err(unproved(
                    "Exterior reference needs exactly one periodic UV axis",
                ));
            }
        };
        let ranges = crate::boolean_robustness::surface_uv_domains(surface);
        let [lo, hi] = ranges[transverse];
        if !point.iter().all(|value| value.is_finite())
            || !(lo <= point[transverse] && point[transverse] <= hi)
        {
            return Err(unproved(
                "Periodic membership query leaves the finite native support",
            ));
        }
        let along = 1 - transverse;
        let [start, end] = ranges[along];
        let step = end * 0.25 - start * 0.25;
        // Oblique alternatives avoid running a straight path through an
        // exact contour joint when the query lies on a periodic shore.
        for reference in [lo - (hi - lo), hi + (hi - lo)] {
            if !reference.is_finite() || (lo <= reference && reference <= hi) {
                continue;
            }
            for offset in [0., step, -step] {
                let mut from = point;
                from[transverse] = reference;
                from[along] += offset;
                if !from[along].is_finite() {
                    continue;
                }
                if let Ok(path) = self.qualify_periodic_face_path(face_id, from, point, limits) {
                    return Ok(QualifiedRootPeriodicFaceMembership { path });
                }
            }
        }
        Err(unproved("No proved path from the periodic face exterior"))
    }

    /// Classify a straight lifted UV path against a face's complete boundary.
    /// Individual boundary components may wind around the quotient, provided
    /// the oriented total winding vanishes. Work limits apply per contour.
    pub fn qualify_periodic_face_path(
        self: &Arc<Self>,
        face_id: usize,
        from: [f64; 2],
        to: [f64; 2],
        limits: ss::OriginalSsPathLimits,
    ) -> Result<QualifiedRootPeriodicFacePath> {
        let t = self.topology();
        let face = t
            .faces
            .get(face_id)
            .ok_or_else(|| unproved("Missing quotient path face"))?;
        let joint_count = t
            .faces
            .iter()
            .flat_map(|face| std::iter::once(face.outer).chain(face.holes.iter().copied()))
            .map(|loop_id| t.loops[loop_id].coedges.len())
            .sum();
        let closure = QualifiedRootPeriodicUvClosure::qualify(self.clone(), joint_count)?;
        let winding = closure
            .loops()
            .iter()
            .filter(|lift| lift.face == face_id)
            .fold([0_i64; 2], |sum, lift| {
                std::array::from_fn(|axis| sum[axis] + i64::from(lift.winding[axis]))
            });
        if winding != [0, 0] {
            return Err(unproved("Periodic face boundary has nonzero total winding"));
        }

        let mut paths = Vec::new();
        for loop_id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            let coedges = &t.loops[loop_id].coedges;
            let support = coedges
                .first()
                .ok_or_else(|| unproved("Empty quotient path contour"))?
                .pcurve
                .support;
            let edges = coedges
                .iter()
                .map(|coedge| {
                    if coedge.pcurve.support != support {
                        return Err(unproved("Quotient path loop needs one source support"));
                    }
                    let edge = &t.edges[coedge.edge].curve;
                    if coedge.reversed {
                        ss::OriginalSsRootEdge::new(edge.end().clone(), edge.start().clone())
                    } else {
                        Ok(edge.clone())
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            let wire = ss::OriginalSsRootWire::new(edges)?;
            paths.push(ss::qualify_original_ss_path_parity(
                &wire, support, from, to, limits,
            )?);
        }
        Ok(QualifiedRootPeriodicFacePath { closure, paths })
    }
}
