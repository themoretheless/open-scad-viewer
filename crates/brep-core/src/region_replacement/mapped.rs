//! Mapped original-support corner construction followed by owned B-rep assembly.
use super::*;
use crate::corner_close::BoundaryUse;
use nurbs_core::{
    corner_blend::{self, mapped},
    curve::Curve,
};
pub struct MappedReplacement {
    pub replacement: Replacement,
    pub corner: mapped::Report,
    pub cells: usize,
    pub exact_work: u64,
}
/// Four complete owned rim edges in patch order UMin, UMax, VMin, VMax.
/// Directions enter retained support charts. `reverse_along` refers to the
/// saved face-local pcurve, not an inferred isoparametric coordinate.
/// Saved 3D edges and root vertices remain authoritative. No fitting/welding
/// or radius law is supplied by this operation.
pub fn replace_g2_mapped(
    source: &Model,
    selected: &[usize],
    uses: [BoundaryUse; 4],
    directions: [[f64; 2]; 4],
    options: corner_blend::Options,
    tolerance_uv: f64,
    tolerances: [f64; 3],
    max_cells: usize,
    max_coefficients: usize,
    max_exact: u64,
) -> Result<MappedReplacement> {
    source.validate()?;
    let chosen: BTreeSet<_> = selected.iter().copied().collect();
    if chosen.len() != selected.len()
        || selected.is_empty()
        || selected.len() > 256
        || selected.iter().any(|&f| f >= source.faces.len())
        || tolerances[0] > source.tolerance_mm
        || !source.persistent_naming_complete()
        || uses.iter().any(|u| {
            u.face >= source.faces.len()
                || u.edge >= source.edges.len()
                || selected.contains(&u.face)
        })
    {
        return Err(refused(
            "Mapped corner needs retained owned supports and unchanged tolerance",
        ));
    }
    let mut saved = Vec::new();
    let mut seen = BTreeSet::new();
    let mut reversed = None;
    for (side, u) in uses.iter().enumerate() {
        if !seen.insert(u.edge) {
            return Err(refused("Mapped corner repeats a rim edge"));
        }
        let incidence = source
            .faces
            .iter()
            .enumerate()
            .flat_map(|(f, face)| {
                std::iter::once(face.outer)
                    .chain(face.holes.iter().copied())
                    .flat_map(move |w| {
                        source.loops[w]
                            .coedges
                            .iter()
                            .filter(move |c| c.edge == u.edge)
                            .map(move |_| f)
                    })
            })
            .collect::<Vec<_>>();
        if incidence.len() != 2
            || incidence.iter().filter(|f| chosen.contains(f)).count() != 1
            || !incidence.contains(&u.face)
        {
            return Err(refused(
                "Mapped rim must separate selected material from one retained support",
            ));
        }
        let face = &source.faces[u.face];
        let hits = std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .flat_map(|w| &source.loops[w].coedges)
            .filter(|c| c.edge == u.edge)
            .collect::<Vec<_>>();
        let owners = source
            .shells
            .iter()
            .flat_map(|s| &s.faces)
            .filter(|f| f.face == u.face)
            .collect::<Vec<_>>();
        if hits.len() != 1 || owners.len() != 1 {
            return Err(refused("Mapped rim support occurrence is not unique"));
        }
        let old = hits[0];
        let [a, b] = old.pcurve.domain();
        let start = if options.reverse_along[side] { 1. } else { 0. };
        let end = 1. - start;
        let point = |t| {
            if side < 2 {
                vec![side as f64, t]
            } else {
                vec![t, (side - 2) as f64]
            }
        };
        let forward = side == 1 || side == 2;
        let reverse = (start < end) != forward;
        let pc = Curve {
            degree: 1,
            knots: vec![a, a, b, b],
            control_points: if reverse {
                vec![point(end), point(start)]
            } else {
                vec![point(start), point(end)]
            },
            weights: vec![1.; 2],
            periodic: false,
        };
        let orientation = !(reverse ^ owners[0].reversed);
        if reversed.is_some_and(|r| r != orientation) {
            return Err(refused("Mapped rim shell orientation is inconsistent"));
        }
        reversed = Some(orientation);
        saved.push((
            old,
            Coedge {
                edge: u.edge,
                reversed: old.reversed ^ reverse,
                pcurve: pc,
            },
        ));
    }
    let supports = std::array::from_fn(|i| mapped::Support {
        surface: &source.faces[uses[i].face].surface,
        pcurve: &saved[i].0.pcurve,
        direction: directions[i],
    });
    let corner = mapped::g2(
        supports,
        options,
        tolerances,
        max_cells,
        max_coefficients,
        max_exact,
    )?;
    if !corner.admitted || !corner.direct_admitted {
        return Err(refused(
            "Mapped corner original-support G2 or interior remains unproven",
        ));
    }
    let mut cells = corner.cells;
    for i in 0..4 {
        for (pc, surface, reverse) in [
            (
                &saved[i].0.pcurve,
                &source.faces[uses[i].face].surface,
                saved[i].0.reversed,
            ),
            (
                &saved[i].1.pcurve,
                &corner.corner.surface,
                saved[i].1.reversed,
            ),
        ] {
            if cells >= max_cells {
                return Err(refused("Mapped saved-rim proof exhausted shared cells"));
            }
            let proof = nurbs_core::curve_surface_agreement::verify(
                &source.edges[uses[i].edge].curve,
                pc,
                surface,
                reverse,
                source.tolerance_mm,
                max_cells - cells,
            )?;
            cells += proof.cells;
            if proof.status != nurbs_core::curve_surface_agreement::Status::WithinTolerance {
                return Err(refused(
                    "Mapped rim does not agree with actual saved 3D edge",
                ));
            }
        }
    }
    if max_cells.saturating_sub(cells) < 3 {
        return Err(refused("No shared cells remain for mapped B-rep assembly"));
    }
    let rim = [2, 1, 3, 0].map(|i| saved[i].1.clone()).to_vec();
    let replacement = replace(
        source,
        selected,
        &corner.corner.surface,
        &[rim],
        reversed.unwrap(),
        tolerance_uv,
        max_cells - cells,
        max_exact - corner.corner.exact_work,
    )?;
    cells += replacement.cells;
    let exact_work = corner.corner.exact_work + replacement.exact_work;
    Ok(MappedReplacement {
        replacement,
        corner,
        cells,
        exact_work,
    })
}
