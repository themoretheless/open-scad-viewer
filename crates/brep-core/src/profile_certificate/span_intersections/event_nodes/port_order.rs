//! Original-source cyclic branch order, with exact intrinsic tangent slices.
use super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
mod intrinsic;
use std::{cmp::Ordering, sync::Arc};
#[derive(Clone, Copy, Debug)]
pub struct ContactPortOrderLimits {
    pub algebraic: ParameterOrderLimits,
    pub max_ports: usize,
    pub max_comparisons: usize,
    pub max_neighbourhood_steps: usize,
}
impl Default for ContactPortOrderLimits {
    fn default() -> Self {
        Self {
            algebraic: ParameterOrderLimits::default(),
            max_ports: 256,
            max_comparisons: 4096,
            max_neighbourhood_steps: 128,
        }
    }
}
#[derive(Clone, Debug)]
pub struct OriginalContactPort {
    pub source: usize,
    pub parameter_group: usize,
    pub direction: i8,
    branch: OwnedEventBranch,
}
impl OriginalContactPort {
    pub fn branch(&self) -> &OwnedEventBranch {
        &self.branch
    }
}
#[derive(Clone, Debug)]
pub struct OwnedContactPortOrder {
    atlas: Arc<OriginalContactAtlas>,
    group: usize,
    limits: ContactPortOrderLimits,
    ports: Vec<OriginalContactPort>,
    comparisons: usize,
}
fn quadrant(ray: &LeadingBranchRay) -> usize {
    let [x, y] = ray.signs;
    if x > 0 && y >= 0 {
        0
    } else if y > 0 && x <= 0 {
        1
    } else if x < 0 && y <= 0 {
        2
    } else {
        3
    }
}
pub(in crate::profile_certificate) fn compare(
    a: &LeadingBranchRay,
    b: &LeadingBranchRay,
    l: ParameterOrderLimits,
) -> Result<Ordering> {
    let q = quadrant(a).cmp(&quadrant(b));
    if q != Ordering::Equal {
        return Ok(q);
    }
    if a.signs[0] == 0 || b.signs[0] == 0 {
        return Ok(match (a.signs[0] == 0, b.signs[0] == 0) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Less,
            _ => Ordering::Greater,
        });
    }
    let interval = |p: &[ExactRatio; 2]| -> Result<[Q; 2]> {
        Ok([decode_ratio(&p[0])?, decode_ratio(&p[1])?])
    };
    let ax = decode_polynomial(&a.numerator[0])?;
    let ay = decode_polynomial(&a.numerator[1])?;
    let bx = decode_polynomial(&b.numerator[0])?;
    let by = decode_polynomial(&b.numerator[1])?;
    arithmetic::parameter_order::compare_rational(
        &decode_polynomial(&a.primitive)?,
        [&ay, &ax],
        &interval(&a.isolating_interval)?,
        &decode_polynomial(&b.primitive)?,
        [&by, &bx],
        &interval(&b.isolating_interval)?,
        l.max_refinements,
        l.max_resultant_size,
    )
}
fn build(
    atlas: &OriginalContactAtlas,
    group: usize,
    l: ContactPortOrderLimits,
) -> Result<(Vec<OriginalContactPort>, usize)> {
    if group >= atlas.groups().len()
        || l.max_ports == 0
        || l.max_ports > 4096
        || l.max_comparisons > 65536
        || l.max_neighbourhood_steps == 0
        || l.max_neighbourhood_steps > 512
        || l.algebraic.max_refinements > 512
        || !(1..=128).contains(&l.algebraic.max_resultant_size)
    {
        return Err(refused("Invalid original contact port group or allowance"));
    }
    atlas.reverify_source()?;
    let members = atlas.groups()[group]
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut ports = Vec::<OriginalContactPort>::new();
    let mut comparisons = 0;
    for (source, parameters) in atlas.ordered_parameters().iter().enumerate() {
        for (parameter_group, refs) in parameters.iter().enumerate() {
            let r = refs[0];
            if !members.contains(&r.occurrence) {
                continue;
            }
            let occurrence = atlas.occurrences()[r.occurrence];
            for direction in [-1, 1] {
                let Some(branch) = try_own_event_branch(
                    atlas.owners()[occurrence.pair].clone(),
                    EventParameterRef {
                        node: occurrence.node,
                        source: r.side,
                    },
                    direction,
                    l.algebraic.max_refinements,
                )?
                else {
                    continue;
                };
                if ports.len() == l.max_ports {
                    return Err(refused("Original contact port allowance exhausted"));
                }
                let candidate = OriginalContactPort {
                    source,
                    parameter_group,
                    direction,
                    branch,
                };
                let (mut lo, mut hi) = (0, ports.len());
                while lo < hi {
                    if comparisons == l.max_comparisons {
                        return Err(refused(
                            "Original cyclic port comparison allowance exhausted",
                        ));
                    }
                    comparisons += 1;
                    let mid = lo + (hi - lo) / 2;
                    match compare(candidate.branch.ray(), ports[mid].branch.ray(), l.algebraic)? {
                        Ordering::Less => hi = mid,
                        Ordering::Greater => lo = mid + 1,
                        Ordering::Equal => {
                            match intrinsic::compare(atlas, &candidate, &ports[mid], l)? {
                                Ordering::Less => hi = mid,
                                Ordering::Greater => lo = mid + 1,
                                Ordering::Equal => {
                                    return Err(refused(
                                        "Intrinsic original tangent branches are not separated",
                                    ));
                                }
                            }
                        }
                    }
                }
                ports.insert(lo, candidate);
            }
        }
    }
    if ports.is_empty() {
        return Err(refused("Original contact has no owned incident branch"));
    }
    // Leading +X rays can approach the cut from below (angle 2pi-ε).
    // Move that exact below-axis prefix to the end for canonical CCW order.
    let mut below = 0;
    for port in &ports {
        if port.branch.ray().signs != [1, 0]
            || intrinsic::transverse_sign(port, l.algebraic.max_refinements)? >= 0
        {
            break;
        }
        below += 1;
    }
    ports.rotate_left(below);
    Ok((ports, comparisons))
}
impl OwnedContactPortOrder {
    pub fn atlas(&self) -> &Arc<OriginalContactAtlas> {
        &self.atlas
    }
    pub fn group(&self) -> usize {
        self.group
    }
    /// Counterclockwise from the positive X ray, retaining original traversal
    /// parameter groups and both actual source-domain directions.
    pub fn ports(&self) -> &[OriginalContactPort] {
        &self.ports
    }
    pub fn comparisons_used(&self) -> usize {
        self.comparisons
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (fresh, comparisons) = build(&self.atlas, self.group, self.limits)?;
        let same = fresh.len() == self.ports.len()
            && fresh.iter().zip(&self.ports).all(|(a, b)| {
                a.source == b.source
                    && a.parameter_group == b.parameter_group
                    && a.direction == b.direction
                    && a.branch.reference() == b.branch.reference()
                    && a.branch.direction() == b.branch.direction()
                    && a.branch.source_cell() == b.branch.source_cell()
                    && a.branch.ray() == b.branch.ray()
                    && a.branch.owner().sources() == b.branch.owner().sources()
                    && a.branch.owner().census() == b.branch.owner().census()
            });
        if !same || comparisons != self.comparisons {
            return Err(refused(
                "Original cyclic port source/germ order receipt changed",
            ));
        }
        for port in &self.ports {
            port.branch.reverify_source()?;
        }
        Ok(())
    }
}
/// Own cyclic contact branch order from complete original event/parameter
/// ownership and exact first-nonzero algebraic germs. Rational slopes compare
/// by checked selected-root eliminants; no sampled angles. Same leading rays
/// use contact-free original monotone neighbourhoods and a common exact slice.
/// Original carrier cuts, filled faces and supporting-surface sewing remain
/// separate requirements.
pub fn own_original_contact_port_order(
    atlas: Arc<OriginalContactAtlas>,
    group: usize,
    limits: ContactPortOrderLimits,
) -> Result<OwnedContactPortOrder> {
    let (ports, comparisons) = build(&atlas, group, limits)?;
    Ok(OwnedContactPortOrder {
        atlas,
        group,
        limits,
        ports,
        comparisons,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cyclic_port_receipt_rejects_missing_permuted_and_detached_source_branches() {
        let a = Curve::from_polyline(vec![vec![-1., -1.], vec![1., 1.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![-1., 1.], vec![1., -1.]]).unwrap();
        let atlas = Arc::new(
            assemble_original_contact_atlas(&[a, b], 1e-12, ContactAtlasLimits::default()).unwrap(),
        );
        let owner =
            own_original_contact_port_order(atlas, 0, ContactPortOrderLimits::default()).unwrap();
        let mut bad = owner.clone();
        bad.ports.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.ports.swap(0, 1);
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.ports[0].source = 99;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.comparisons += 1;
        assert!(bad.reverify_source().is_err());
        // Translation preserves these rays and primitive parameter intervals;
        // source binding must still reject inheriting a different contact.
        let a = Curve::from_polyline(vec![vec![9., 9.], vec![11., 11.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![9., 11.], vec![11., 9.]]).unwrap();
        let other = Arc::new(
            assemble_original_contact_atlas(&[a, b], 1e-12, ContactAtlasLimits::default()).unwrap(),
        );
        let other =
            own_original_contact_port_order(other, 0, ContactPortOrderLimits::default()).unwrap();
        assert_eq!(owner.ports[0].branch.ray(), other.ports[0].branch.ray());
        let mut bad = owner;
        bad.ports[0].branch = other.ports[0].branch.clone();
        assert!(bad.reverify_source().is_err());
    }
}
