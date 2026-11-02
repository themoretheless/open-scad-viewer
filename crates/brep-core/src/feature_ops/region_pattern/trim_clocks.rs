//! Original authored scalar clocks remain in the immutable source. Converted
//! nominal cap clocks gain authority only from fresh original-domain proofs.
use super::closure::Cap;
use super::*;
use brep_topology::CoedgeTrim;
use nurbs_core::curve_surface_agreement::{self as agreement, Status};
fn admit(status: Status, cells: usize, work: &mut ProofWork) -> Result<()> {
    work.cells += cells;
    if status != Status::WithinTolerance {
        return Err(error("Original trim clock transfer remains unproven"));
    }
    Ok(())
}
fn original_use(
    source: &Model,
    f: usize,
    w: usize,
    slot: usize,
    seen: &mut BTreeSet<(usize, usize, usize)>,
    limits: Limits,
    work: &mut ProofWork,
) -> Result<()> {
    let c = &source.loops[w].coedges[slot];
    if c.trim().is_none() || seen.contains(&(f, w, slot)) {
        return Ok(());
    }
    let r = crate::boundary_agreement::verify_authored_use(
        source,
        f,
        w,
        slot,
        work.remaining(limits)?.max_cells,
    )?;
    admit(r.0, r.1, work)?;
    let full = agreement::verify(
        &source.edges[c.edge].curve,
        &c.pcurve,
        &source.faces[f].surface,
        c.reversed,
        source.tolerance_mm,
        work.remaining(limits)?.max_cells,
    )?;
    admit(full.status, full.cells, work)?;
    seen.insert((f, w, slot));
    Ok(())
}
fn reflect(domain: [f64; 2], v: f64) -> f64 {
    if v == domain[0] {
        domain[1]
    } else if v == domain[1] {
        domain[0]
    } else {
        domain[0] + (domain[1] - v)
    }
}
fn map(domain: [f64; 2], target: [f64; 2], v: f64, reverse: bool) -> f64 {
    let mut t = if v == domain[0] {
        0.
    } else if v == domain[1] {
        1.
    } else {
        (v - domain[0]) / (domain[1] - domain[0])
    };
    if reverse {
        t = 1. - t;
    }
    if t == 0. {
        target[0]
    } else if t == 1. {
        target[1]
    } else {
        target[0] + (target[1] - target[0]) * t
    }
}
pub(super) fn transfer(
    source: &Model,
    selected: &BTreeSet<usize>,
    incidence: &BTreeMap<usize, Vec<(usize, usize)>>,
    caps: &mut [Cap],
    limits: Limits,
    work: &mut ProofWork,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    for cap in caps {
        for ring in &mut cap.rings {
            for c in ring {
                let (sf, sw) = *incidence[&c.edge]
                    .iter()
                    .find(|(f, _)| selected.contains(f))
                    .unwrap();
                let (pf, pw) = *incidence[&c.edge]
                    .iter()
                    .find(|(f, _)| !selected.contains(f))
                    .unwrap();
                let si = source.loops[sw]
                    .coedges
                    .iter()
                    .position(|e| e.edge == c.edge)
                    .unwrap();
                let pi = source.loops[pw]
                    .coedges
                    .iter()
                    .position(|e| e.edge == c.edge)
                    .unwrap();
                let sc = &source.loops[sw].coedges[si];
                let pc = &source.loops[pw].coedges[pi];
                if sc.trim().is_none() && pc.trim().is_none() {
                    continue;
                }
                original_use(source, sf, sw, si, &mut seen, limits, work)?;
                original_use(source, pf, pw, pi, &mut seen, limits, work)?;
                let from_support = pc.trim().is_some();
                let origin = if from_support { pc } else { sc };
                let trim = origin.trim().unwrap();
                let edge = &source.edges[c.edge].curve;
                let domain = edge.domain();
                let curve = trim.curve_parameter.map(|v| {
                    if c.reversed == origin.reversed {
                        v
                    } else {
                        reflect(domain, v)
                    }
                });
                let pcurve = if from_support && cap.surface == source.faces[pf].surface {
                    trim.pcurve_parameter.map(|v| {
                        map(
                            origin.pcurve.domain(),
                            c.pcurve.domain(),
                            v,
                            c.reversed != origin.reversed,
                        )
                    })
                } else {
                    curve.map(|v| map(domain, c.pcurve.domain(), v, false))
                };
                if curve.iter().chain(&pcurve).any(|v| !v.is_finite()) {
                    return Err(error("Converted cap clock is not representable"));
                }
                let nominal = CoedgeTrim {
                    curve_parameter: curve,
                    pcurve_parameter: pcurve,
                    periodic_lift: [[0; 2]; 2],
                };
                prove_binding(
                    edge,
                    &cap.surface,
                    c,
                    origin,
                    nominal,
                    source.tolerance_mm,
                    limits,
                    work,
                )?;
                work.charge_cells(2, limits)?;
                let lifts =
                    crate::boundary_agreement::periodic_lifts(&cap.surface, &c.pcurve, pcurve)?;
                *c = c.clone().with_trim(CoedgeTrim {
                    curve_parameter: curve,
                    pcurve_parameter: pcurve,
                    periodic_lift: lifts,
                })?;
            }
        }
    }
    Ok(())
}

fn prove_binding(
    edge: &nurbs_core::curve::Curve,
    surface: &nurbs_core::surface::Surface,
    c: &crate::Coedge,
    origin: &crate::Coedge,
    nominal: CoedgeTrim,
    tolerance: f64,
    limits: Limits,
    work: &mut ProofWork,
) -> Result<()> {
    let original = origin
        .trim()
        .ok_or_else(|| error("Original trim binding is missing"))?;
    let nodes = std::array::from_fn::<_, 2, _>(|i| {
        [original.curve_parameter[i], nominal.curve_parameter[i]]
    });
    let clock = agreement::verify_parameter_clocks(
        edge,
        &nodes,
        [origin.reversed, c.reversed],
        tolerance,
        work.remaining(limits)?.max_cells,
    )?;
    admit(clock.status, clock.cells, work)?;
    let relation = std::array::from_fn::<_, 2, _>(|i| {
        [original.curve_parameter[i], nominal.pcurve_parameter[i]]
    });
    let proof = agreement::verify_parameter_correspondence(
        edge,
        &c.pcurve,
        surface,
        &relation,
        origin.reversed,
        tolerance,
        work.remaining(limits)?.max_cells,
    )?;
    admit(proof.status, proof.cells, work)
}
/// Affine placement can amplify a nominal clock's geometric drift. Re-prove
/// the ORIGINAL authored clock on the actual moved curve/cap before naming.
pub(super) fn verify_occurrence(
    feature: &FeatureRegion,
    moved: &Model,
    limits: Limits,
) -> Result<ProofWork> {
    let mut work = ProofWork::default();
    let source = &feature.source;
    let selected = feature.selected.iter().copied().collect::<BTreeSet<_>>();
    let shell = source
        .shells
        .iter()
        .find(|s| s.faces.iter().any(|u| selected.contains(&u.face)))
        .ok_or_else(|| error("Original clock shell ownership is missing"))?;
    let mut uses = BTreeMap::<usize, Vec<(usize, usize, usize)>>::new();
    for u in &shell.faces {
        let f = &source.faces[u.face];
        for w in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for (slot, c) in source.loops[w].coedges.iter().enumerate() {
                uses.entry(c.edge).or_default().push((u.face, w, slot));
            }
        }
    }
    let source_edges = source
        .1
        .edges
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i))
        .collect::<BTreeMap<_, _>>();
    for &face in &feature.caps {
        let f = &moved.faces[face];
        for w in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for c in &moved.loops[w].coedges {
                let Some(nominal) = c.trim() else {
                    continue;
                };
                let edge = source_edges[&feature.tool.1.edges[c.edge]];
                let selected_use = uses[&edge]
                    .iter()
                    .find(|(f, _, _)| selected.contains(f))
                    .unwrap();
                let support_use = uses[&edge]
                    .iter()
                    .find(|(f, _, _)| !selected.contains(f))
                    .unwrap();
                let sc = &source.loops[selected_use.1].coedges[selected_use.2];
                let pc = &source.loops[support_use.1].coedges[support_use.2];
                let origin = if pc.trim().is_some() { pc } else { sc };
                prove_binding(
                    &moved.edges[c.edge].curve,
                    &f.surface,
                    c,
                    origin,
                    nominal,
                    moved.tolerance_mm,
                    limits,
                    &mut work,
                )?;
            }
        }
    }
    Ok(work)
}
