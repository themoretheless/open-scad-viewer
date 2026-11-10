//! Actual source branch cut before every other contact and coordinate critical root.
use super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
use arithmetic::branch_neighbourhood::{self as math, ParameterValue};
use std::sync::Arc;
#[derive(Clone, Copy, Debug)]
pub struct BranchNeighbourhoodLimits {
    pub germ_refinements: usize,
    pub max_steps: usize,
}
impl Default for BranchNeighbourhoodLimits {
    fn default() -> Self {
        Self {
            germ_refinements: 192,
            max_steps: 128,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchNeighbourhoodRecord {
    pub refined_primitive_root: [ExactRatio; 2],
    /// Local cell parameter bounds, not rounded natural-parameter cuts.
    pub local_contact_box: [ExactRatio; 2],
    pub natural_cut: ExactRatio,
    pub cut_position: [ExactRatio; 2],
    pub contact_coordinate_critical: bool,
    pub steps: usize,
}
#[derive(Clone, Debug)]
pub struct OwnedBranchNeighbourhood {
    atlas: Arc<OriginalContactAtlas>,
    source: usize,
    parameter_group: usize,
    direction: i8,
    axis: usize,
    limits: BranchNeighbourhoodLimits,
    branch: OwnedEventBranch,
    record: BranchNeighbourhoodRecord,
}
fn value(
    atlas: &OriginalContactAtlas,
    source: usize,
    group: usize,
) -> Result<(Arc<CurveEventNodes>, EventParameterRef, ParameterValue)> {
    let r = atlas.ordered_parameters()[source][group][0];
    let o = atlas.occurrences()[r.occurrence];
    let owner = atlas.owners()[o.pair].clone();
    let reference = EventParameterRef {
        node: o.node,
        source: r.side,
    };
    let e = &owner.nodes()[o.node].exact;
    let p = ParameterValue {
        primitive: decode_polynomial(&e.primitive)?,
        value: decode_polynomial(&e.parameters[r.side])?,
        interval: [
            decode_ratio(&e.isolating_interval[0])?,
            decode_ratio(&e.isolating_interval[1])?,
        ],
    };
    Ok((owner, reference, p))
}
fn derive(
    atlas: &OriginalContactAtlas,
    source: usize,
    group: usize,
    direction: i8,
    axis: usize,
    l: BranchNeighbourhoodLimits,
) -> Result<(OwnedEventBranch, BranchNeighbourhoodRecord)> {
    if source >= atlas.sources().len()
        || group >= atlas.ordered_parameters()[source].len()
        || !matches!(direction, -1 | 1)
        || axis > 1
        || l.germ_refinements > 512
        || l.max_steps == 0
        || l.max_steps > 512
    {
        return Err(refused(
            "Invalid original branch neighbourhood reference or allowance",
        ));
    }
    atlas.reverify_source()?;
    let (owner, r, contact) = value(atlas, source, group)?;
    let branch = own_event_branch(owner, r, direction, l.germ_refinements)?;
    let axis_sign = branch.ray().signs[axis];
    if axis_sign == 0 {
        return Err(refused(
            "Neighbourhood coordinate needs a nonzero original leading ray",
        ));
    }
    let neighbour = if direction > 0 {
        if group + 1 < atlas.ordered_parameters()[source].len() {
            Some(value(atlas, source, group + 1)?.2)
        } else {
            None
        }
    } else if group > 0 {
        Some(value(atlas, source, group - 1)?.2)
    } else {
        None
    };
    let actual = cells(&atlas.sources()[source])?;
    let cell = &actual[branch.source_cell()];
    let proof = math::prove(
        &cell.net,
        &cell.domain,
        contact,
        neighbour,
        direction,
        axis,
        axis_sign,
        l.max_steps,
    )?;
    let record = BranchNeighbourhoodRecord {
        refined_primitive_root: proof.root.map(|q| ratio(&q)),
        local_contact_box: proof.parameter_box.map(|q| ratio(&q)),
        natural_cut: ratio(&proof.cut),
        cut_position: proof.cut_point.map(|q| ratio(&q)),
        contact_coordinate_critical: proof.contact_critical,
        steps: proof.steps,
    };
    Ok((branch, record))
}
impl OwnedBranchNeighbourhood {
    pub fn atlas(&self) -> &Arc<OriginalContactAtlas> {
        &self.atlas
    }
    pub fn source(&self) -> usize {
        self.source
    }
    pub fn parameter_group(&self) -> usize {
        self.parameter_group
    }
    pub fn direction(&self) -> i8 {
        self.direction
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn branch(&self) -> &OwnedEventBranch {
        &self.branch
    }
    pub fn record(&self) -> &BranchNeighbourhoodRecord {
        &self.record
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (branch, record) = derive(
            &self.atlas,
            self.source,
            self.parameter_group,
            self.direction,
            self.axis,
            self.limits,
        )?;
        if record != self.record
            || branch.reference() != self.branch.reference()
            || branch.source_cell() != self.branch.source_cell()
            || branch.direction() != self.branch.direction()
            || branch.ray() != self.branch.ray()
            || branch.owner().sources() != self.branch.owner().sources()
            || branch.owner().census() != self.branch.owner().census()
        {
            return Err(refused(
                "Original monotone branch neighbourhood source or cut receipt changed",
            ));
        }
        self.branch.reverify_source()
    }
}
/// Own a strictly monotone original coordinate graph from an actual contact
/// to an exact rational cut, before the nearest other original contact and any
/// other coordinate critical point. Full root census is used, not sampled
/// derivatives. Endpoint-stationary germs are admitted without changing the
/// source curve. This does not alone order two tangent branches or publish faces.
pub fn own_branch_neighbourhood(
    atlas: Arc<OriginalContactAtlas>,
    source: usize,
    parameter_group: usize,
    direction: i8,
    axis: usize,
    limits: BranchNeighbourhoodLimits,
) -> Result<OwnedBranchNeighbourhood> {
    let (branch, record) = derive(&atlas, source, parameter_group, direction, axis, limits)?;
    Ok(OwnedBranchNeighbourhood {
        atlas,
        source,
        parameter_group,
        direction,
        axis,
        limits,
        branch,
        record,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_original_graph_replays_cut_position_critical_census_and_allowance() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![-3., 0.], vec![0., 6.], vec![3., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let b = Curve::from_polyline(vec![vec![-4., 1.5], vec![4., 1.5]]).unwrap();
        let atlas = Arc::new(
            assemble_original_contact_atlas(&[a, b], 1e-12, ContactAtlasLimits::default()).unwrap(),
        );
        let owner =
            own_branch_neighbourhood(atlas, 0, 0, 1, 1, BranchNeighbourhoodLimits::default())
                .unwrap();
        owner.reverify_source().unwrap();
        let mut bad = owner.clone();
        bad.record.natural_cut = ["1".into(), "1".into()];
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.record.cut_position[0] = ["99".into(), "1".into()];
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.record.local_contact_box[0] = ["0".into(), "1".into()];
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.record.contact_coordinate_critical = true;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.record.steps += 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.axis = 0;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner;
        bad.limits.max_steps = 1;
        assert!(bad.reverify_source().is_err());
    }
}
