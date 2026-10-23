//! Native orchestration of retained cap incidence and shared diagnostic budgets.
use crate::{Result, field};
use brep_core::Model;
use std::collections::{BTreeSet, HashMap};
use value_codec::{Value, json};
fn audit(op: &str, mut request: Value) -> Result<Value> {
    request["op"] = json!(op);
    nurbs_core::dispatch(request)
}
fn yes(value: &Value, key: &str) -> bool {
    value[key] == json!(true)
}
fn charge(remaining: &mut u64, report: &Value, key: &str) -> Result<()> {
    let work: u64 = field(report, key)?;
    if work > *remaining {
        return Err(nurbs_core::Error::new(
            "BREP_SWEEP_CAP_CONTACT_INVALID",
            "Invalid native cap work accounting",
        ));
    }
    *remaining -= work;
    Ok(())
}
#[derive(Clone, Copy)]
struct Use {
    face: usize,
    wire: usize,
    coedge: usize,
    reversed: bool,
}
pub fn inspect(
    model: &Model,
    caps: &[usize],
    max_walls: usize,
    tolerance: f64,
    max_products: u64,
    max_cells: u64,
    max_work: u64,
) -> Result<Value> {
    if caps.is_empty()
        || caps.len() > 16
        || model.faces.len() > 1024
        || caps.iter().any(|&i| i >= model.faces.len())
        || caps.iter().collect::<BTreeSet<_>>().len() != caps.len()
        || !tolerance.is_finite()
        || tolerance <= 0.
        || max_products > 100000
        || max_cells > 100000
        || max_work > 1000000
    {
        return Err(nurbs_core::Error::new(
            "BREP_SWEEP_CAP_CONTACT_INVALID",
            "Invalid sweep cap selection or budgets",
        ));
    }
    model.validate()?;
    let wall_faces: Vec<_> = (0..model.faces.len())
        .filter(|id| !caps.contains(id))
        .collect();
    let walls: Vec<_> = wall_faces
        .iter()
        .map(|&id| model.faces[id].surface.clone())
        .collect();
    let face_uses: Vec<Vec<_>> = (0..model.faces.len())
        .map(|face| {
            model
                .shells
                .iter()
                .enumerate()
                .flat_map(|(id, s)| {
                    s.faces
                        .iter()
                        .filter(move |u| u.face == face)
                        .map(move |u| (id, u.reversed))
                })
                .collect()
        })
        .collect();
    let mut edge_uses: HashMap<usize, Vec<Use>> = HashMap::new();
    for (face, f) in model.faces.iter().enumerate() {
        for &wire in std::iter::once(&f.outer).chain(&f.holes) {
            for (coedge, c) in model.loops[wire].coedges.iter().enumerate() {
                edge_uses.entry(c.edge).or_default().push(Use {
                    face,
                    wire,
                    coedge,
                    reversed: c.reversed,
                });
            }
        }
    }
    let mut diagnostic_work = max_work;
    let mut native_work = max_work;
    let mut reports = Vec::new();
    for &cap_face in caps {
        let cap = &model.faces[cap_face];
        let edges: BTreeSet<_> = std::iter::once(&cap.outer)
            .chain(&cap.holes)
            .flat_map(|&w| model.loops[w].coedges.iter().map(|c| c.edge))
            .collect();
        let boundaries: Vec<Option<&str>> = wall_faces
            .iter()
            .map(|&id| {
                let f = &model.faces[id];
                let s = &f.surface;
                let bounds = [
                    s.knots_u[s.degree_u],
                    s.knots_u[s.control_points.len()],
                    s.knots_v[s.degree_v],
                    s.knots_v[s.control_points[0].len()],
                ];
                let mut labels = BTreeSet::new();
                for &wire in std::iter::once(&f.outer).chain(&f.holes) {
                    for c in &model.loops[wire].coedges {
                        if !edges.contains(&c.edge) {
                            continue;
                        }
                        for (axis, value, label) in [
                            (0, bounds[0], "uMin"),
                            (0, bounds[1], "uMax"),
                            (1, bounds[2], "vMin"),
                            (1, bounds[3], "vMax"),
                        ] {
                            if !c.pcurve.control_points.is_empty()
                                && c.pcurve
                                    .control_points
                                    .iter()
                                    .all(|p| p.len() == 2 && p[axis] == value)
                            {
                                labels.insert(label);
                            }
                        }
                    }
                }
                if labels.len() == 1 {
                    labels.into_iter().next()
                } else {
                    None
                }
            })
            .collect();
        let mut products = max_products;
        let mut cells = max_cells;
        let mut work = diagnostic_work;
        let mut coedges = Vec::new();
        let mut all_covered = true;
        let mut all_exact = true;
        let mut all_cap_agreement = true;
        let mut all_paired = true;
        let mut all_wall_agreement = true;
        for &wire in std::iter::once(&cap.outer).chain(&cap.holes) {
            for (index, c) in model.loops[wire].coedges.iter().enumerate() {
                let stored = &model.edges[c.edge].curve;
                let world = if c.reversed {
                    stored.reverse()?
                } else {
                    stored.clone()
                };
                let agreement = audit(
                    "sweep_cap_boundary_audit",
                    json!({"surface":cap.surface,"world":world,"uv":c.pcurve,"tolerance":tolerance,"maxProducts":products}),
                )?;
                charge(&mut products, &agreement, "products")?;
                let exact = audit(
                    "sweep_coedge_exact_audit",
                    json!({"surface":cap.surface,"world":stored,"uv":c.pcurve,"reversed":c.reversed,"maxWork":work}),
                )?;
                charge(&mut work, &exact, "work")?;
                let uses = &edge_uses[&c.edge];
                let wall_uses: Vec<_> = uses
                    .iter()
                    .filter(|u| wall_faces.contains(&u.face))
                    .collect();
                let mut wall_agreements = Vec::new();
                let mut covered = true;
                let mut wall_exact = true;
                let mut wall_agreed = true;
                for u in &wall_uses {
                    let s = &model.faces[u.face].surface;
                    let uv = &model.loops[u.wire].coedges[u.coedge].pcurve;
                    let agreement = audit(
                        "sweep_coedge_agreement_audit",
                        json!({"surface":s,"world":stored,"uv":uv,"reversed":u.reversed,"tolerance":tolerance,"maxCells":cells}),
                    )?;
                    charge(&mut cells, &agreement, "cells")?;
                    let exact = audit(
                        "sweep_coedge_exact_audit",
                        json!({"surface":s,"world":stored,"uv":uv,"reversed":u.reversed,"maxWork":work}),
                    )?;
                    charge(&mut work, &exact, "work")?;
                    let label = boundaries[wall_faces.iter().position(|&f| f == u.face).unwrap()];
                    let coverage = if let Some(label) = label {
                        audit(
                            "sweep_boundary_coverage_audit",
                            json!({"surface":s,"uv":uv,"boundary":label}),
                        )?
                    } else {
                        Value::Null
                    };
                    covered &= yes(&coverage, "wholeBoundaryCovered");
                    wall_exact &= yes(&exact, "exactIdentityCertified");
                    wall_agreed &= yes(&agreement, "withinTolerance");
                    wall_agreements.push(json!({"face":u.face,"wire":u.wire,"coedge":u.coedge,"agreement":agreement,"exact":exact,"coverage":coverage}));
                }
                let cap_shells = &face_uses[cap_face];
                let wall_shells = wall_uses.first().map(|u| &face_uses[u.face]);
                let paired = uses.len() == 2
                    && uses.iter().filter(|u| u.face == cap_face).count() == 1
                    && wall_uses.len() == 1
                    && cap_shells.len() == 1
                    && wall_shells.is_some_and(|s| {
                        s.len() == 1
                            && s[0].0 == cap_shells[0].0
                            && (wall_uses[0].reversed != s[0].1) != (c.reversed != cap_shells[0].1)
                    });
                all_covered &= wall_uses.len() == 1 && covered;
                all_exact &=
                    yes(&exact, "exactIdentityCertified") && wall_uses.len() == 1 && wall_exact;
                all_cap_agreement &= yes(&agreement, "withinBudget");
                all_paired &= paired;
                all_wall_agreement &= wall_uses.len() == 1 && wall_agreed;
                let faces: Vec<_> = wall_uses.iter().map(|u| u.face).collect();
                coedges.push(json!({"edge":c.edge,"wire":wire,"coedge":index,"wallFaces":faces,"pairedOpposite":paired,"agreement":agreement,"exact":exact,"wallAgreements":wall_agreements}));
            }
        }
        diagnostic_work = work;
        let wall_audit = audit(
            "sweep_cap_wall_audit",
            json!({"cap":cap.surface,"walls":walls,"boundaries":boundaries,"maxWalls":max_walls}),
        )?;
        let native = brep_core::sweep_cap_contacts::inspect(
            model,
            cap_face,
            caps,
            brep_core::sweep_cap_contacts::Budgets {
                max_walls,
                max_exact_work: native_work,
                max_chart_cells: 1000,
                max_trim_pairs: 100000,
                max_trim_cells: 100000,
                max_trim_domain_cells: 1000000,
            },
        )?;
        if native.exact_work > native_work {
            return Err(nurbs_core::Error::new(
                "BREP_SWEEP_CAP_CONTACT_INVALID",
                "Invalid native cap work accounting",
            ));
        }
        native_work -= native.exact_work;
        let native = json!({"capCertified":native.cap_certified,"planarControlHullCertified":native.planar_control_hull_certified,"allCapWallContactsCertified":native.all_cap_wall_contacts_certified,"separatedWalls":native.separated_walls,"allowedBoundaries":native.allowed_boundaries,"unresolvedWalls":native.unresolved_walls,"exactWork":native.exact_work,"reason":native.reason,"globalEmbeddingCertified":false});
        let nonempty = !coedges.is_empty();
        reports.push(json!({"capFace":cap_face,"wallFaces":wall_faces,"boundaries":boundaries,"audit":wall_audit,"native":native,"capCoedges":coedges,
            "wallBoundariesCovered":nonempty && all_covered,"capAndWallExactIdentityCertified":nonempty && all_exact,
            "capEdgeAgreementWithinBudget":nonempty && all_cap_agreement,"pairedOppositeCoedges":nonempty && all_paired,"wallEdgeAgreementWithinTolerance":nonempty && all_wall_agreement}));
    }
    Ok(json!(reports))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_caps_share_budgets_and_preserve_model() {
        let sections: Vec<_> = [0., 2.]
            .into_iter()
            .map(|z| {
                vec![vec![
                    nurbs_core::primitives::ellipse_arc(
                        [0., 0., z],
                        [1., 0., 0.],
                        [0., 1., 0.],
                        0.,
                        360.,
                    )
                    .unwrap(),
                ]]
            })
            .collect();
        let model = brep_core::rational_section_loft(&sections).unwrap();
        let before = model.clone();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        let report = inspect(&model, &caps, 1024, 1e-9, 100000, 1000, 1000000).unwrap();
        let reports: Vec<Value> = value_codec::from_value(report).unwrap();
        assert_eq!(reports.len(), 2);
        for r in &reports {
            assert!(yes(r, "wallBoundariesCovered"));
            assert!(yes(r, "capAndWallExactIdentityCertified"));
            assert!(yes(r, "pairedOppositeCoedges"));
            assert!(yes(r, "wallEdgeAgreementWithinTolerance"));
            assert!(yes(&r["native"], "capCertified"));
        }
        let refused: Vec<Value> =
            value_codec::from_value(inspect(&model, &caps, 1024, 1e-9, 0, 0, 0).unwrap()).unwrap();
        for r in &refused {
            assert!(!yes(r, "capAndWallExactIdentityCertified"));
            assert!(!yes(&r["native"], "capCertified"));
        }
        let pair_options = json!({"clearance":0.000001,"distanceTolerance":0.000001,"maxInjectivityCells":100000,"maxPairs":100000,"maxPairCells":100000});
        let pairs = inspect_pairs(&model, &caps, &pair_options).unwrap();
        assert_eq!(pairs["capFaces"], json!(caps));
        assert!(yes(&pairs["audit"], "chartsAndPairsCertified"));
        let limited=inspect_pairs(&model,&caps,&json!({"clearance":0.000001,"distanceTolerance":0.000001,"maxInjectivityCells":0,"maxPairs":0,"maxPairCells":0})).unwrap();
        assert_eq!(limited["unresolvedCapFaces"], json!(caps));
        let mapped: Vec<Value> = field(&limited, "unresolvedPairs").unwrap();
        assert_eq!(mapped[0]["faces"], json!(caps));
        assert_eq!(model, before);
    }
}

/// Full retained cap charts conservatively include their trimmed regions.
pub fn inspect_pairs(model: &Model, caps: &[usize], options: &Value) -> Result<Value> {
    if caps.is_empty()
        || caps.len() > 16
        || caps.iter().any(|&i| i >= model.faces.len())
        || caps.iter().collect::<BTreeSet<_>>().len() != caps.len()
    {
        return Err(nurbs_core::Error::new(
            "BREP_SWEEP_CAP_CONTACT_INVALID",
            "Invalid cap pair face selection",
        ));
    }
    let walls: Vec<_> = caps
        .iter()
        .map(|&i| model.faces[i].surface.clone())
        .collect();
    let mut request = options.clone();
    request["walls"] = json!(walls);
    request["sharedBoundaries"] = json!(Vec::<[usize; 2]>::new());
    let report = audit("sweep_wall_audit", request)?;
    let unresolved: Vec<usize> = field(&report, "unresolvedCharts")?;
    let unresolved_faces: Vec<_> = unresolved.iter().map(|&i| caps[i]).collect();
    let pairs: Vec<Value> = field(&report["pairs"], "unresolved")?;
    let mapped: Vec<_> = pairs
        .iter()
        .map(|p| {
            let indices: [usize; 2] = field(p, "patches")?;
            let reason: String = field(p, "reason")?;
            Ok(json!({"faces":[caps[indices[0]],caps[indices[1]]],"reason":reason}))
        })
        .collect::<Result<_>>()?;
    Ok(
        json!({"capFaces":caps,"audit":report,"unresolvedCapFaces":unresolved_faces,"unresolvedPairs":mapped,"globalEmbeddingCertified":false}),
    )
}
