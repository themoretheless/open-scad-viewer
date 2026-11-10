//! Shared closed cap-network sewing. Normal-offset authority belongs to callers.
use super::planar_network::{Boundary, Cut, Limits, prepare, topology};
use crate::feature_ops::region_pattern::{Limits as ProofLimits, Verification, qualification};
use crate::{Error, Model, Result, rounded_patch_network as network};
use nurbs_core::surface::Surface;
fn refused(message: &str) -> Error {
    Error::new("BREP_NETWORK_THICKNESS_REFUSED", message)
}
pub(super) struct Built {
    pub model: Model,
    pub evidence: Verification,
    pub boundaries: Vec<Boundary>,
    pub cuts: Vec<Cut>,
    pub exact_work: u64,
}
pub(super) fn construct(
    sources: &[Surface],
    caps: &[Surface],
    reversed: &[bool],
    signed_thickness: f64,
    tolerance: f64,
    limits: Limits,
    cells: &mut usize,
) -> Result<Built> {
    let source_boundaries =
        prepare::build(sources, None, tolerance, limits.proof.max_cells, cells)?;
    let partitioned =
        !source_boundaries.cuts.is_empty() || topology::has_clock_candidates(&source_boundaries);
    let target_boundaries = if partitioned {
        Some(prepare::build(
            caps,
            Some(&source_boundaries.cuts),
            tolerance,
            limits.proof.max_cells,
            cells,
        )?)
    } else {
        None
    };
    let mut exact = 0;
    let boundaries = if partitioned {
        topology::boundaries_prepared(
            sources,
            reversed,
            limits.exact_work,
            &mut exact,
            &source_boundaries,
            limits.proof.max_cells,
            cells,
            tolerance,
        )?
    } else {
        topology::boundaries(sources, reversed, limits.exact_work, &mut exact)?
    };
    let negative = signed_thickness < 0.;
    let mut patches = sources.to_vec();
    let mut orientations = reversed.iter().map(|r| r ^ !negative).collect::<Vec<_>>();
    patches.extend(caps.iter().cloned());
    orientations.extend(reversed.iter().map(|r| r ^ negative));
    let mut prepared = source_boundaries
        .pieces
        .iter()
        .map(|v| {
            v.iter()
                .map(|p| (p.curve.clone(), p.pcurve.clone()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    prepared.extend(if let Some(target) = &target_boundaries {
        target
            .pieces
            .iter()
            .map(|v| {
                v.iter()
                    .map(|p| (p.curve.clone(), p.pcurve.clone()))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    } else {
        caps.iter()
            .map(|s| (0..4).map(|side| network::boundary(s, side)).collect())
            .collect()
    });
    let mut prepared_mode = partitioned;
    for b in &boundaries {
        let a = &source_boundaries.pieces[b.patch]
            .iter()
            .find(|p| p.address == *b)
            .ok_or_else(|| refused("Exposed source boundary has no prepared carrier"))?
            .curve;
        let target = if let Some(target) = &target_boundaries {
            let has_cut = source_boundaries
                .cuts
                .iter()
                .any(|c| c.patch == b.patch && c.side == b.side);
            if has_cut
                && network::boundary(&sources[b.patch], b.side).0.domain()
                    != network::boundary(&caps[b.patch], b.side).0.domain()
            {
                return Err(refused(
                    "Moving partitioned boundary clocks require an explicit cut map",
                ));
            }
            target.pieces[b.patch]
                .iter()
                .find(|p| p.address.side == b.side && (!has_cut || p.address == *b))
                .ok_or_else(|| refused("Target boundary has no corresponding prepared piece"))?
                .curve
                .clone()
        } else {
            network::boundary(&caps[b.patch], b.side).0
        };
        let same_basis = a.degree == target.degree
            && a.knots == target.knots
            && a.control_points.len() == target.control_points.len()
            && a.weights == target.weights;
        let wall = if same_basis {
            super::side(a, &target)
        } else {
            prepared_mode = true;
            nurbs_core::ruled_surface(a, &target)?
        };
        prepared.push((0..4).map(|side| network::boundary(&wall, side)).collect());
        patches.push(wall);
        orientations.push(reversed[b.patch] ^ negative);
    }
    if exact >= limits.exact_work {
        return Err(refused("Patch network exact sewing allowance exhausted"));
    }
    let assembled = if prepared_mode {
        network::assemble_prepared(
            &patches,
            &orientations,
            tolerance,
            limits.exact_work - exact,
            &prepared,
            limits.proof.max_cells,
            cells,
        )?
    } else {
        network::assemble(
            &patches,
            &orientations,
            tolerance,
            limits.exact_work - exact,
            &[],
        )?
    };
    exact += assembled.correspondence_work;
    let remaining = limits
        .proof
        .max_cells
        .checked_sub(*cells)
        .filter(|n| *n > 0)
        .ok_or_else(|| refused("Patch network shared proof allowance exhausted"))?;
    let mut evidence = qualification::verify_with_linear_charts(
        &assembled.model,
        ProofLimits {
            max_cells: remaining,
            ..limits.proof
        },
    )?;
    evidence.cells += *cells;
    Ok(Built {
        model: assembled.model,
        evidence,
        boundaries,
        cuts: source_boundaries.cuts,
        exact_work: exact,
    })
}
