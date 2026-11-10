//! source profile body operations: face.
use super::*;
impl<'i, 'f, 'a, 's, 'p, 'b> TransitionPatchIncidence<'i, 'f, 'a, 's, 'p, 'b> {
pub fn face(&self) -> &TransitionFace<'_, '_, '_, '_> {
        self.rails.face()
    }
/// Fresh admission of the four outer rail portions against their neighbor
    /// contour fragments. Selected geometry retains exact preserved rails.
    pub fn qualify_outer_edges<'o>(
        &'o self,
        faces: [&RetainedProfileFace<'_, '_, '_, 's, 'p, 'b>; 2],
        max_work: u64,
        max_drivers: usize,
    ) -> Result<OuterEdgeReport<'o, 'i, 'f, 'a, 's, 'p, 'b>> {
        use crate::source_boundary_fragment::{Endpoint, Role};
        let mut out = OuterEdgeReport {
            assembly: None,
            work: 0,
            driver_cells: 0,
            reason: "profile-outer-edge-face-owner-unproven",
        };
        if faces[0].exterior.section.high == faces[1].exterior.section.high
            || faces
                .iter()
                .any(|face| !std::ptr::eq(face.exterior.section.boundary, self.rails.face.boundary))
        {
            return Ok(out);
        }
        let faces = if faces[0].exterior.section.high {
            [faces[1], faces[0]]
        } else {
            faces
        };
        for end in 0..2 {
            if faces[end].exterior.kept_loop().len() != self.retained[end].len()
                || !faces[end]
                    .exterior
                    .kept_loop()
                    .iter()
                    .zip(&self.retained[end])
                    .all(|(a, b)| a.same_definition(b))
            {
                return Ok(out);
            }
        }
        let mut edges = Vec::new();
        for rail in 0..2 {
            let root = self.rails.face.boundary.rooted[rail];
            let original = root.points[0].edge();
            let selected = root.points[0].source_side();
            let neighbor = 1 - selected;
            let contour = &self.rails.rails[rail].neighbor_contour;
            let [wire, index] = contour.middle;
            for end in 0..2 {
                let point = &root.points[end];
                let candidates=self.retained[end].iter().filter(|fragment|fragment.surface()==original.uses()[selected].surface() && fragment.curve()==original.uses()[selected].curve()
                    && fragment.endpoints().iter().any(|e|matches!(e,Endpoint::Crossing{point:p,role:Role::Boundary} if p.same_definition(point.root())))).collect::<Vec<_>>();
                if candidates.len() != 1 {
                    out.reason = "profile-outer-edge-selected-use-unproven";
                    return Ok(out);
                }
                let neighbors=[&contour.loops[wire][index-1],&contour.loops[wire][index+1]].into_iter().filter(|fragment|
                    fragment.endpoints().iter().any(|e|matches!(e,Endpoint::Carrier{point:p,side} if *side==neighbor && p.same_definition(point)))).collect::<Vec<_>>();
                if neighbors.len() != 1 {
                    out.reason = "profile-outer-edge-neighbor-use-unproven";
                    return Ok(out);
                }
                let ranges = crate::source_seam_tangency::ranges(original);
                let checked = crate::source_mapped_edge::qualify(
                    original.world(),
                    [candidates[0], neighbors[0]],
                    [ranges[selected], ranges[neighbor]],
                    [None, None],
                    max_work - out.work,
                    max_drivers - out.driver_cells,
                )?;
                out.work += checked.work_used;
                out.driver_cells += checked.driver_cells;
                out.reason = checked.reason;
                let Some(edge) = checked.edge else {
                    return Ok(out);
                };
                edges.push(edge);
            }
        }
        out.assembly = Some(OuterEdgeAssembly {
            patch: self,
            edges: edges.try_into().unwrap_or_else(|_| unreachable!()),
            retained_factored: faces.map(|face| face.factored_boundary),
        });
        out.reason = "profile-four-outer-edges-qualified";
        Ok(out)
    }
pub fn rails(&self) -> &TransitionRailIncidence<'_, '_, '_, '_, '_> {
        self.rails
    }
pub fn retained(&self) -> &[Vec<crate::source_boundary_fragment::Fragment>; 2] {
        &self.retained
    }
}
