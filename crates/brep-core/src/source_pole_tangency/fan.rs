//! Complete pole atlas on an already admitted immutable source body.
//! Topological vertex links and exclusion of extra inter-face contacts come
//! from that private body's geometry, never from caller diagnostic flags.
use super::*;
use nurbs_core::surface_quotient_injectivity::{PolarCollar, certify_polar_collar};

pub struct FanLimits {
    pub inward_extent: f64,
    pub subdivisions: usize,
    pub exact_work: u64,
    pub cells: usize,
}
/// Every incident original polar face has an injective, rank-two punctured
/// collar; all pole limits agree, and the body owns an embedded disk link.
/// This does not assert smoothness across ordinary seams or a differentiable
/// graph/curvature limit at the pole. Those require additional proof.
pub struct BodyPoleAtlas<'a, E: Carrier> {
    planes: BodyPlaneLimits<'a, E>,
    collars: Vec<(Address, PolarCollar<'a>)>,
    orientations: Vec<PoleOrientation>,
}
/// Direction of the common outward limiting normal, witnessed by one
/// separated world component. This is not a caller-supplied normal vector.
pub struct PoleOrientation {
    vertex: usize,
    axis: usize,
    positive: bool,
}
impl PoleOrientation {
    pub fn vertex(&self) -> usize {
        self.vertex
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn positive(&self) -> bool {
        self.positive
    }
}
pub(super) fn oriented_component(bounds: [f64; 2], winding: i32, reverse: bool) -> Option<bool> {
    if winding != 1 && winding != -1 {
        return None;
    }
    let positive = if bounds[0] > 0. {
        true
    } else if bounds[1] < 0. {
        false
    } else {
        return None;
    };
    Some(positive ^ (winding == -1) ^ reverse)
}
impl<'a, E: Carrier> BodyPoleAtlas<'a, E> {
    pub fn body(&self) -> &Body<E> {
        self.planes.body()
    }
    pub fn planes(&self) -> &BodyPlaneLimits<'a, E> {
        &self.planes
    }
    pub fn orientations(&self) -> &[PoleOrientation] {
        &self.orientations
    }
    pub fn collars(&self) -> &[(Address, PolarCollar<'a>)] {
        &self.collars
    }
}
pub struct FanReport<'a, E: Carrier> {
    pub certificate: Option<BodyPoleAtlas<'a, E>>,
    pub exact_work: u64,
    pub cells: usize,
    pub completed_poles: usize,
    pub uncertain_pole: Option<Address>,
    pub reason: &'static str,
}
/// Recompute the complete common-plane fan proof and every actual original
/// collar. All collars share ONE cell allowance; any failed collar or exhausted
/// budget prevents admission of the entire atlas. The geometry and topology
/// are bound by borrowing the same private body for the complete certificate.
pub fn qualify_body_pole_atlas<E: Carrier>(
    body: &Body<E>,
    limits: FanLimits,
) -> Result<FanReport<'_, E>> {
    if !limits.inward_extent.is_finite()
        || limits.inward_extent <= 0.
        || limits.inward_extent > 1.
        || !(1..=64).contains(&limits.subdivisions)
        || !(1..=100000).contains(&limits.cells)
    {
        return Err(Error::new(
            "BREP_SOURCE_POLE_ATLAS",
            "Choose bounded complete pole atlas work",
        ));
    }
    let proof = qualify_body_plane_limits(body, limits.exact_work)?;
    let mut out = FanReport {
        certificate: None,
        exact_work: proof.work,
        cells: 0,
        completed_poles: 0,
        uncertain_pole: None,
        reason: proof.reason,
    };
    let Some(planes) = proof.certificate else {
        return Ok(out);
    };
    let shell = body.geometry().shell();
    // These are private admission invariants, checked defensively rather than
    // accepting a report written by the caller. No closed disconnected link
    // or an unclassified contact can be upgraded by local collar successes.
    if !body.geometry().topology().all_manifold
        || planes.vertices().iter().any(|v| {
            !body
                .geometry()
                .topology()
                .links
                .iter()
                .any(|l| l.vertex == *v)
        })
    {
        out.reason = "source-pole-atlas-owned-disk-link-unproven";
        return Ok(out);
    }
    let Some(regions) = shell.regions() else {
        out.reason = "source-pole-atlas-material-orientation-unproven";
        return Ok(out);
    };
    let mut oriented = BTreeMap::<usize, (usize, bool)>::new();
    for (address, plane) in planes.poles() {
        out.uncertain_pole = Some(*address);
        let vertex = shell.vertices()[address.face][address.wire][address.edge][0];
        let bounds = plane.limit().normal_bounds();
        let axis = if let Some(&(axis, _)) = oriented.get(&vertex) {
            axis
        } else {
            let Some(axis) = bounds.iter().position(|b| b[0] > 0. || b[1] < 0.) else {
                out.reason = "source-pole-atlas-normal-direction-unproven";
                return Ok(out);
            };
            axis
        };
        let Some(positive) = oriented_component(
            bounds[axis],
            regions[address.face].chart_winding(),
            body.reverse_orientation(),
        ) else {
            out.reason = "source-pole-atlas-normal-direction-unproven";
            return Ok(out);
        };
        if oriented
            .get(&vertex)
            .is_some_and(|&(_, sign)| sign != positive)
        {
            out.reason = "source-pole-atlas-incoherent-oriented-normal";
            return Ok(out);
        }
        oriented.insert(vertex, (axis, positive));
    }
    let orientations = oriented
        .into_iter()
        .map(|(vertex, (axis, positive))| PoleOrientation {
            vertex,
            axis,
            positive,
        })
        .collect();
    let mut collars = Vec::new();
    for ((address, pole), (plane_address, plane)) in shell.poles().iter().zip(planes.poles()) {
        out.uncertain_pole = Some(*address);
        if address != plane_address || !std::ptr::eq(pole, plane.pole()) {
            out.reason = "source-pole-atlas-original-ownership-unproven";
            return Ok(out);
        }
        if out.cells == limits.cells {
            out.reason = "source-pole-atlas-cell-limit";
            return Ok(out);
        }
        let collar = certify_polar_collar(
            pole.source().surface(),
            plane.limit().boundary(),
            limits.inward_extent,
            limits.subdivisions,
            limits.cells - out.cells,
        )?;
        out.cells += collar.cells;
        let Some(collar) = collar.certificate else {
            out.reason = collar.reason;
            return Ok(out);
        };
        collars.push((*address, collar));
        out.completed_poles += 1;
    }
    if collars.len() != shell.poles().len() || collars.len() != planes.poles().len() {
        out.reason = "source-pole-atlas-incomplete";
        return Ok(out);
    }
    out.certificate = Some(BodyPoleAtlas {
        planes,
        collars,
        orientations,
    });
    out.uncertain_pole = None;
    out.reason = "source-body-complete-embedded-pole-atlas-qualified";
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oriented_limits_distinguish_a_fold_from_a_reflected_parameter_chart() {
        use nurbs_core::{surface::Surface, surface_join::Boundary};
        let source = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0.; 3]; 2],
                vec![vec![0.5, 0., 0.], vec![0.5, 0.5, 0.]],
                vec![vec![1., 0., 1.], vec![1., 1., 1.]],
            ],
            weights: vec![vec![1., 0.5]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let mut reflected = source.clone();
        reflected.control_points.reverse();
        reflected.weights.reverse();
        let a = normals::prepare_plane_limit(&source, Boundary::UMin, 1000000).unwrap();
        let b = normals::prepare_plane_limit(&reflected, Boundary::UMax, 1000000).unwrap();
        let a = a.certificate.expect(a.reason);
        let b = b.certificate.expect(b.reason);
        assert_eq!(
            normals::compare_plane_limits(&a, &b, 1000000)
                .unwrap()
                .same_plane,
            Some(true)
        );
        let x = a.normal_bounds()[2];
        let y = b.normal_bounds()[2];
        // The actual exact native plane limits agree as unoriented planes.
        // Keeping the same material winding nevertheless reverses the normal.
        assert_ne!(
            oriented_component(x, 1, false),
            oriented_component(y, 1, false)
        );
        assert!(oriented_component(x, 1, false).is_some());
        assert!(oriented_component(y, 1, false).is_some());
        // Reversing the source UV chart AND its material winding is coherent.
        assert_eq!(
            oriented_component(x, 1, false),
            oriented_component(y, -1, false)
        );
        assert_eq!(oriented_component([1., 2.], 1, true), Some(false));
        assert_eq!(oriented_component([-1., 1.], 1, false), None);
        assert_eq!(oriented_component([1., 2.], 0, false), None);
    }
}
