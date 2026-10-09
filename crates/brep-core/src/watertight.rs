//! Tolerance-qualified seam closure, independent of solid embedding/outwardness.
use crate::{Model, Result, boundary_agreement};
use nurbs_core::curve_surface_agreement::Status;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Closure {
    WithinTolerance,
    OpenBoundary,
    Mismatch,
    Unresolved,
}
#[derive(Clone, Debug)]
pub struct ShellClosure {
    pub shell: usize,
    pub boundary_edges: Vec<usize>,
    pub incompatible_edges: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub status: Closure,
    pub shells: Vec<ShellClosure>,
    pub agreement: boundary_agreement::Report,
    /// Each admitted face lift lies within this tolerance of its common edge.
    /// Two lifts sharing that edge therefore lie within twice this value.
    pub edge_tolerance_mm: f64,
}
/// Inspect all authored shells, regardless of their `closed` flags. Source is
/// immutable. Structural/endpoint/UV checks remain mandatory. Complete interval
/// agreement replaces sampled admission. Degenerate pole uses are included in
/// agreement but do not require a second face use. This certifies seam closure
/// within tolerance, not exact equality, trim simplicity, embedding or volume.
pub fn inspect(model: &Model, max_cells: usize) -> Result<Report> {
    let agreement = boundary_agreement::verify(model, max_cells)?;
    let mut shells = Vec::new();
    for (shell, s) in model.shells.iter().enumerate() {
        let mut uses = std::collections::BTreeMap::<usize, Vec<bool>>::new();
        for face_use in &s.faces {
            let face = &model.faces[face_use.face];
            for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                for c in &model.loops[wire].coedges {
                    if !model.edges[c.edge].degenerate {
                        uses.entry(c.edge)
                            .or_default()
                            .push(c.reversed ^ face_use.reversed);
                    }
                }
            }
        }
        let mut out = ShellClosure {
            shell,
            boundary_edges: vec![],
            incompatible_edges: vec![],
        };
        for (edge, directions) in uses {
            if directions.len() == 1 {
                out.boundary_edges.push(edge);
            } else if directions.len() != 2 || directions[0] == directions[1] {
                out.incompatible_edges.push(edge);
            }
        }
        shells.push(out);
    }
    let status = if shells.iter().any(|s| !s.boundary_edges.is_empty()) {
        Closure::OpenBoundary
    } else if shells.iter().any(|s| !s.incompatible_edges.is_empty())
        || agreement.uses.iter().any(|u| u.status == Status::Mismatch)
    {
        Closure::Mismatch
    } else if model.shells.is_empty() || !agreement.complete {
        Closure::Unresolved
    } else {
        Closure::WithinTolerance
    };
    Ok(Report {
        status,
        shells,
        agreement,
        edge_tolerance_mm: model.tolerance_mm,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundaryAddress {
    pub model: usize,
    pub face: usize,
    pub wire: usize,
    pub coedge: usize,
}
#[derive(Clone, Debug)]
pub struct BoundaryPair {
    pub boundaries: [usize; 2],
    pub status: Status,
    pub cells: usize,
}
#[derive(Clone, Debug)]
pub struct PatchSetReport {
    pub boundaries: Vec<BoundaryAddress>,
    /// Only mutually unique proven correspondences are admitted.
    pub matches: Vec<[usize; 2]>,
    pub unresolved: Vec<usize>,
    pub pairs: Vec<BoundaryPair>,
    pub untested_pairs: usize,
    pub cells: usize,
    pub all_boundaries_paired: bool,
}
/// Infer opposite boundary correspondences between distinct authored models.
/// Tests every cross-model boundary pair; a budget-limited alternative prevents
/// uniqueness. Correspondence uses normalized affine parameters; nonlinear
/// reparameterization and overlap fragmentation remain unresolved. No topology
/// is mutated or solid certificate granted. Each matched lift is within twice
/// the common model tolerance of the source edge (source admission plus target
/// comparison). Inputs must all use exactly the same tolerance.
pub fn inspect_patch_set(
    models: &[Model],
    max_pairs: usize,
    max_cells: usize,
) -> Result<PatchSetReport> {
    use nurbs_core::{Error, curve_surface_agreement};
    if models.is_empty()
        || models.len() > 256
        || !(1..=100000).contains(&max_pairs)
        || !(1..=1000000).contains(&max_cells)
        || models
            .iter()
            .any(|m| m.tolerance_mm != models[0].tolerance_mm)
    {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Patch audit requires 1..256 models, equal tolerance and bounded positive work",
        ));
    }
    let mut out = PatchSetReport {
        boundaries: vec![],
        matches: vec![],
        unresolved: vec![],
        pairs: vec![],
        untested_pairs: 0,
        cells: 0,
        all_boundaries_paired: false,
    };
    let mut source_complete = true;
    for (id, model) in models.iter().enumerate() {
        model.validate_boundary_diagnostic_inputs()?;
        if out.cells < max_cells {
            let r = boundary_agreement::verify(model, max_cells - out.cells)?;
            out.cells += r.cells;
            source_complete &= r.complete;
        } else {
            source_complete = false;
        }
        let mut count = vec![0usize; model.edges.len()];
        for c in model.loops.iter().flat_map(|l| &l.coedges) {
            count[c.edge] += 1;
        }
        for (face, f) in model.faces.iter().enumerate() {
            for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                for (coedge, c) in model.loops[wire].coedges.iter().enumerate() {
                    if count[c.edge] == 1 && !model.edges[c.edge].degenerate {
                        out.boundaries.push(BoundaryAddress {
                            model: id,
                            face,
                            wire,
                            coedge,
                        });
                    }
                }
            }
        }
    }
    if out.boundaries.len() > 1024 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Patch audit supports at most 1024 free boundaries",
        ));
    }
    let mut possible = vec![Vec::<usize>::new(); out.boundaries.len()];
    let mut proven = vec![Vec::<usize>::new(); out.boundaries.len()];
    for a in 0..out.boundaries.len() {
        for b in a + 1..out.boundaries.len() {
            let [left, right] = [out.boundaries[a], out.boundaries[b]];
            if left.model == right.model {
                continue;
            }
            let mut status = Status::Unresolved;
            let mut cells = 0;
            if out.pairs.len() < max_pairs && out.cells < max_cells {
                let lm = &models[left.model];
                let rm = &models[right.model];
                let lc = &lm.loops[left.wire].coedges[left.coedge];
                let rc = &rm.loops[right.wire].coedges[right.coedge];
                let lf = lm
                    .shells
                    .iter()
                    .flat_map(|s| &s.faces)
                    .find(|f| f.face == left.face)
                    .unwrap();
                let rf = rm
                    .shells
                    .iter()
                    .flat_map(|s| &s.faces)
                    .find(|f| f.face == right.face)
                    .unwrap();
                let reverse = !(lc.reversed ^ lf.reversed) ^ rf.reversed;
                let r = curve_surface_agreement::verify(
                    &lm.edges[lc.edge].curve,
                    &rc.pcurve,
                    &rm.faces[right.face].surface,
                    reverse,
                    lm.tolerance_mm,
                    (max_cells - out.cells).min(100000),
                )?;
                status = r.status;
                cells = r.cells;
                out.cells += cells;
            }
            if status != Status::Mismatch {
                possible[a].push(b);
                possible[b].push(a);
            }
            if status == Status::WithinTolerance {
                proven[a].push(b);
                proven[b].push(a);
            }
            // Preserve unresolved alternatives even after the verification budget.
            if out.pairs.len() < max_pairs {
                out.pairs.push(BoundaryPair {
                    boundaries: [a, b],
                    status,
                    cells,
                });
            } else {
                out.untested_pairs += 1;
            }
        }
    }
    for a in 0..out.boundaries.len() {
        if source_complete && possible[a].len() == 1 && proven[a].len() == 1 {
            let b = proven[a][0];
            if possible[b].len() == 1 && proven[b].len() == 1 {
                if a < b {
                    out.matches.push([a, b]);
                }
                continue;
            }
        }
        out.unresolved.push(a);
    }
    out.all_boundaries_paired =
        source_complete && !out.boundaries.is_empty() && out.unresolved.is_empty();
    Ok(out)
}

#[derive(Clone, Debug)]
pub struct BoundaryPortion {
    /// Rounded authored curves, independently qualified against both surfaces.
    pub curve: nurbs_core::curve::Curve,
    pub source_pcurve: nurbs_core::curve::Curve,
    pub target_pcurve: nurbs_core::curve::Curve,
    pub agreements: Vec<nurbs_core::curve_surface_agreement::Report>,
    pub cells: usize,
    pub within_tolerance: bool,
}
/// Check explicit normalized subintervals of two independent free boundaries.
/// Fractions follow each coedge's pcurve traversal. Positive ordered subranges
/// are mandatory. Trimming uses binary64 knot insertion; continuous checks
/// qualify the resulting authored curve against both original surfaces rather
/// than claiming exact identity to the unrounded mathematical restriction.
/// This is correspondence evidence, without editing or splitting topology.
pub fn inspect_boundary_portions(
    models: &[Model],
    boundaries: [BoundaryAddress; 2],
    fractions: [[f64; 2]; 2],
    max_cells: usize,
) -> Result<BoundaryPortion> {
    use nurbs_core::{Error, curve::Curve, curve_surface_agreement};
    let invalid = || {
        Error::new(
            "BREP_INVALID_INPUT",
            "Portion audit requires distinct free boundaries, ordered unit fractions and bounded work",
        )
    };
    if !(1..=1000000).contains(&max_cells)
        || boundaries[0].model == boundaries[1].model
        || fractions
            .iter()
            .any(|r| !r.iter().all(|v| v.is_finite()) || r[0] < 0. || r[1] > 1. || r[0] >= r[1])
    {
        return Err(invalid());
    }
    let mut selected = Vec::new();
    for address in boundaries {
        let model = models.get(address.model).ok_or_else(invalid)?;
        model.validate_boundary_diagnostic_inputs()?;
        let face = model.faces.get(address.face).ok_or_else(invalid)?;
        if face.outer != address.wire && !face.holes.contains(&address.wire) {
            return Err(invalid());
        }
        let c = model
            .loops
            .get(address.wire)
            .and_then(|l| l.coedges.get(address.coedge))
            .ok_or_else(invalid)?;
        if model.edges[c.edge].degenerate
            || model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|other| other.edge == c.edge)
                .count()
                != 1
        {
            return Err(invalid());
        }
        let face_use = model
            .shells
            .iter()
            .flat_map(|s| &s.faces)
            .find(|f| f.face == address.face)
            .ok_or_else(invalid)?;
        selected.push((model, face, c, face_use.reversed));
    }
    if selected[0].0.tolerance_mm != selected[1].0.tolerance_mm {
        return Err(invalid());
    }
    fn portion(curve: &Curve, range: [f64; 2]) -> Result<Curve> {
        if range == [0., 1.] {
            return Ok(curve.clone());
        }
        let [a, b] = curve.domain();
        let map = |t: f64| {
            if t == 0. {
                a
            } else if t == 1. {
                b
            } else {
                a + (b - a) * t
            }
        };
        curve.trim(map(range[0]), map(range[1]))
    }
    let (source, surface, sc, sf) = selected[0];
    let (_, target, tc, tf) = selected[1];
    let range = if sc.reversed {
        [1. - fractions[0][1], 1. - fractions[0][0]]
    } else {
        fractions[0]
    };
    let curve = portion(&source.edges[sc.edge].curve, range)?;
    let source_pcurve = portion(&sc.pcurve, fractions[0])?;
    let target_pcurve = portion(&tc.pcurve, fractions[1])?;
    let mut agreements = Vec::new();
    let mut cells = 0;
    for (pcurve, surface, reverse) in [
        (&source_pcurve, &surface.surface, sc.reversed),
        (&target_pcurve, &target.surface, !(sc.reversed ^ sf) ^ tf),
    ] {
        if cells == max_cells {
            break;
        }
        let proof = curve_surface_agreement::verify(
            &curve,
            pcurve,
            surface,
            reverse,
            source.tolerance_mm,
            (max_cells - cells).min(100000),
        )?;
        cells += proof.cells;
        agreements.push(proof);
    }
    let within_tolerance = agreements.len() == 2
        && agreements
            .iter()
            .all(|r| r.status == Status::WithinTolerance);
    Ok(BoundaryPortion {
        curve,
        source_pcurve,
        target_pcurve,
        agreements,
        cells,
        within_tolerance,
    })
}
