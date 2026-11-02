//! Selection-owned coplanar chains for the independent extrusion Full Round.
use super::{build, endpoint, owns_segment};
use crate::{
    Model, Result,
    feature_ops::{dot, error, line, plane, sub},
    solid_audit::SolidAuditCertificate,
};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct FullRoundGroupsReport {
    pub model: Model,
    /// Original source identities, never temporary canonical-model indices.
    pub replaced_faces: Vec<usize>,
    pub support_faces: [Vec<usize>; 2],
    pub radius_mm: f64,
    pub audit: SolidAuditCertificate,
    pub tangency: std::sync::Arc<crate::feature_ops::full_round_seams::Qualification>,
}

/// Full Round for left/end/right groups on a single proven XY extrusion.
/// Each group must own one continuous chain of straight, coplanar outer-profile
/// segments. Selected seams are dissolved; all unselected profile carriers
/// and holes are retained by the extrusion builder. No inferred extra faces
/// are added to a user's selection.
pub fn full_round_groups(source: &Model, groups: [&[usize]; 3]) -> Result<FullRoundGroupsReport> {
    source.validate()?;
    if source.bodies.len() != 1 || source.shells.len() != 1 || groups.iter().any(|g| g.is_empty()) {
        return Err(error(
            "Full Round groups require one body and three nonempty groups",
        ));
    }
    let mut selected = BTreeSet::new();
    if groups
        .iter()
        .flat_map(|g| g.iter())
        .any(|&f| !selected.insert(f))
    {
        return Err(error("Full Round groups must be unique and disjoint"));
    }
    let mut profile = crate::prism::recognize(source)?
        .ok_or_else(|| error("Full Round groups require a proven XY extrusion"))?;
    let wire = &profile.loops[0];
    let n = wire.len();
    let mut labels = vec![None; n];
    let mut planes = Vec::new();
    for (g, faces) in groups.iter().enumerate() {
        let base = plane(source, faces[0])?;
        if base.1[2].abs() > 1e-10 {
            return Err(error("Full Round groups must select vertical side faces"));
        }
        for &f in *faces {
            let (p, normal) = plane(source, f)?;
            if dot(normal, base.1) < 1. - 1e-10
                || dot(base.1, sub(p, base.0)).abs() > source.tolerance_mm
                || source.faces[f]
                    .surface
                    .control_points
                    .iter()
                    .flatten()
                    .any(|q| {
                        dot(base.1, sub([q[0], q[1], q[2]], base.0)).abs()
                            > source.tolerance_mm / 4.
                    })
            {
                return Err(error(
                    "Full Round group faces must be consistently oriented and coplanar",
                ));
            }
            let mut owned = Vec::new();
            for (i, c) in wire.iter().enumerate() {
                if owns_segment(source, f, c, profile.z_max)? {
                    owned.push(i);
                }
            }
            if owned.len() != 1 {
                return Err(error(
                    "Selected face must own exactly one complete outer profile segment",
                ));
            }
            let i = owned[0];
            let c = &wire[i];
            if c.degree != 1
                || c.control_points.len() != 2
                || c.weights.iter().any(|w| *w <= 0.)
                || labels[i].replace(g).is_some()
            {
                return Err(error(
                    "Full Round group segments must be straight and uniquely owned",
                ));
            }
        }
        planes.push(base);
    }
    let mut starts = [0; 3];
    let mut ends = [0; 3];
    for g in 0..3 {
        let entries = (0..n)
            .filter(|&i| labels[i] == Some(g) && labels[(i + n - 1) % n] != Some(g))
            .collect::<Vec<_>>();
        if entries.len() != 1 {
            return Err(error(
                "Full Round group must be one continuous outer-profile chain",
            ));
        }
        starts[g] = entries[0];
        let mut end = starts[g];
        while labels[(end + 1) % n] == Some(g) {
            end = (end + 1) % n;
        }
        ends[g] = end;
    }
    let merged = (0..3)
        .map(|g| {
            line(
                endpoint(&wire[starts[g]], false)?,
                endpoint(&wire[ends[g]], true)?,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    // Start at a group entry so no collapsed chain straddles the output seam.
    let mut collapsed = Vec::new();
    for k in 0..n {
        let i = (starts[1] + k) % n;
        match labels[i] {
            Some(g) if i == starts[g] => collapsed.push(merged[g].clone()),
            Some(_) => {}
            None => collapsed.push(wire[i].clone()),
        }
    }
    profile.loops[0] = collapsed;
    let mut canonical = crate::prism::extrude(&profile.loops, profile.z_min, profile.z_max)?;
    canonical.tolerance_mm = source.tolerance_mm;
    canonical.inherit_topology_ids(&[source]);
    let mut selected_canonical = [0; 3];
    for g in 0..3 {
        let mut owners = Vec::new();
        for f in 0..canonical.faces.len() {
            if let Ok((_, normal)) = plane(&canonical, f) {
                if dot(normal, planes[g].1) > 1. - 1e-10
                    && owns_segment(&canonical, f, &merged[g], profile.z_max)?
                {
                    owners.push(f);
                }
            }
        }
        if owners.len() != 1 {
            return Err(error(
                "Cannot bind the collapsed group to one authored side",
            ));
        }
        selected_canonical[g] = owners[0];
    }
    let rounded = build(&canonical, selected_canonical)?;
    rounded.model.validate()?;
    Ok(FullRoundGroupsReport {
        model: rounded.model,
        replaced_faces: groups[1].to_vec(),
        support_faces: [groups[0].to_vec(), groups[2].to_vec()],
        radius_mm: rounded.radius_mm,
        audit: rounded.audit,
        tangency: rounded.tangency,
    })
}
