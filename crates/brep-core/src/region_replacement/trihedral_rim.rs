//! Six endpoint-owned curves for a central corner rim, not a filled surface.
use super::TrihedralStrips;
use nurbs_core::{
    Error, Result,
    curve::Curve,
    curve_join::{self, Decision, Endpoint, Tolerances},
};

pub struct RimConnector {
    pub source_face: usize,
    /// (strip index, endpoint of that strip's near cross-section).
    pub ports: [(usize, usize); 2],
    pub curve: Curve,
    pub joins: [curve_join::Report; 2],
}
pub struct TrihedralRim {
    pub cross_sections: [Curve; 3],
    pub connectors: Vec<RimConnector>,
    /// One cyclic traversal, oriented through literal source-owned endpoints.
    pub ordered: Vec<Curve>,
}
fn refuse(message: &str) -> Error {
    Error::new("BREP_TRIHEDRAL_RIM_UNPROVEN", message)
}
fn endpoint(c: &Curve, end: usize) -> Result<(Vec<f64>, Vec<f64>)> {
    c.validate()?;
    if c.periodic || c.degree < 2 || c.control_points[0].len() != 3 {
        return Err(refuse("Need clamped regular spatial cross-sections"));
    }
    let d = c.domain();
    if !c.knots[..=c.degree].iter().all(|&t| t == d[0])
        || !c.knots[c.control_points.len()..].iter().all(|&t| t == d[1])
    {
        return Err(refuse("Cross-section endpoint is not clamped"));
    }
    let n = c.control_points.len();
    let (i, j) = if end == 0 { (0, 1) } else { (n - 1, n - 2) };
    let p = c.control_points[i].clone();
    // Outward endpoint tangent: from adjacent pole to the owned endpoint.
    let mut tangent: Vec<f64> = p
        .iter()
        .zip(&c.control_points[j])
        .map(|(a, b)| a - b)
        .collect();
    let length = tangent.iter().map(|v| v * v).sum::<f64>().sqrt();
    if !length.is_finite() || length <= 0. {
        return Err(refuse("Degenerate endpoint tangent"));
    }
    for v in &mut tangent {
        *v /= length;
    }
    Ok((p, tangent))
}
/// Preserve each actual cross-section endpoint literally. Fresh G0/G1/G2
/// inspection is mandatory; this does not certify interior rim simplicity,
/// support-face membership, a corner fill or closed-solid assembly.
pub fn build_trihedral_rim(strips: &TrihedralStrips, tolerances: [f64; 3]) -> Result<TrihedralRim> {
    if strips.strips.len() != 3 || tolerances.iter().any(|v| !v.is_finite() || *v < 0.) {
        return Err(refuse("Need three strips and finite tolerances"));
    }
    let cross_sections: [Curve; 3] = std::array::from_fn(|i| {
        let strip = &strips.strips[i];
        let near = if strip.end_order[0] == 0 { 0 } else { 1 };
        strip.patch.ends[near].curve.clone()
    });
    let mut faces = std::collections::BTreeMap::<usize, Vec<(usize, usize)>>::new();
    for (i, strip) in strips.strips.iter().enumerate() {
        for side in 0..2 {
            faces
                .entry(strip.support_faces[side])
                .or_default()
                .push((i, side));
        }
    }
    if faces.len() != 3
        || faces
            .values()
            .any(|ports| ports.len() != 2 || ports[0].0 == ports[1].0)
    {
        return Err(refuse(
            "Corner rim needs three distinct pairwise support faces",
        ));
    }
    let mut connectors = Vec::new();
    for (face, ports) in faces {
        let [(a, ae), (b, be)] = [ports[0], ports[1]];
        let (p, ta) = endpoint(&cross_sections[a], ae)?;
        let (q, tb) = endpoint(&cross_sections[b], be)?;
        let h = p
            .iter()
            .zip(&q)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt()
            / 5.;
        if !h.is_finite() || h <= tolerances[0] {
            return Err(refuse("Corner connector has no resolved span"));
        }
        let shift = |p: &Vec<f64>, t: &Vec<f64>, scale: f64| {
            p.iter()
                .zip(t)
                .map(|(x, v)| x + scale * v)
                .collect::<Vec<_>>()
        };
        let curve = Curve {
            degree: 5,
            knots: [vec![0.; 6], vec![1.; 6]].concat(),
            control_points: vec![
                p.clone(),
                shift(&p, &ta, h),
                shift(&p, &ta, 2. * h),
                shift(&q, &tb, 2. * h),
                shift(&q, &tb, h),
                q.clone(),
            ],
            weights: vec![1.; 6],
            periodic: false,
        };
        let ep = |e| {
            if e == 0 {
                Endpoint::Start
            } else {
                Endpoint::End
            }
        };
        let tol = || Tolerances {
            position: tolerances[0],
            tangent: tolerances[1],
            curvature: tolerances[2],
        };
        let joins = [
            curve_join::inspect(&cross_sections[a], ep(ae), &curve, Endpoint::Start, tol())?,
            curve_join::inspect(&curve, Endpoint::End, &cross_sections[b], ep(be), tol())?,
        ];
        if joins.iter().any(|r| r.g2 != Decision::WithinTolerance) {
            return Err(refuse("Central rim endpoint G2 is unproven"));
        }
        connectors.push(RimConnector {
            source_face: face,
            ports: [(a, ae), (b, be)],
            curve,
            joins,
        });
    }
    let mut ordered = vec![cross_sections[0].clone()];
    let mut current = (0, 1);
    let mut visited = std::collections::BTreeSet::new();
    for step in 0..3 {
        let found = connectors
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                c.ports
                    .iter()
                    .position(|p| *p == current)
                    .map(|end| (i, end))
            })
            .collect::<Vec<_>>();
        if found.len() != 1 {
            return Err(refuse("Ambiguous rim port incidence"));
        }
        let (i, end) = found[0];
        if !visited.insert(i) {
            return Err(refuse("Rim closes before visiting all connectors"));
        }
        let connector = &connectors[i];
        ordered.push(if end == 0 {
            connector.curve.clone()
        } else {
            connector.curve.reverse()?
        });
        let next = connector.ports[1 - end];
        if step == 2 {
            if next != (0, 0) {
                return Err(refuse("Rim does not close on its original start"));
            }
        } else {
            if next.0 == 0 {
                return Err(refuse("Premature rim closure"));
            }
            ordered.push(if next.1 == 0 {
                cross_sections[next.0].clone()
            } else {
                cross_sections[next.0].reverse()?
            });
            current = (next.0, 1 - next.1);
        }
    }
    for i in 0..ordered.len() {
        if ordered[i].control_points.last()
            != ordered[(i + 1) % ordered.len()].control_points.first()
        {
            return Err(refuse("Oriented rim lost literal endpoint closure"));
        }
    }
    Ok(TrihedralRim {
        cross_sections,
        connectors,
        ordered,
    })
}

pub struct RimSupports {
    pub pcurves: Vec<Curve>,
    pub cells: usize,
    pub projection_evaluations: usize,
}
/// Fresh full-interval lift agreement and whole-curve membership in each
/// ORIGINAL trimmed support. Numerical projection alone never admits a rim.
pub fn inspect_rim_supports(
    source: &crate::Model,
    rim: &TrihedralRim,
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<RimSupports> {
    source.validate()?;
    if rim.connectors.len() != 3
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || !(6..=100000).contains(&max_cells)
    {
        return Err(refuse(
            "Need three connectors and bounded support membership work",
        ));
    }
    let mut result = RimSupports {
        pcurves: Vec::new(),
        cells: 0,
        projection_evaluations: 0,
    };
    for connector in &rim.connectors {
        let face = source
            .faces
            .get(connector.source_face)
            .ok_or_else(|| refuse("Missing original rim support"))?;
        let remaining = max_cells.saturating_sub(result.cells);
        if remaining < 2 {
            return Err(refuse("Rim support work exhausted"));
        }
        let projection = nurbs_core::curve_pullback::project(
            &connector.curve,
            &face.surface,
            nurbs_core::curve_pullback::Limits {
                tolerance: source.tolerance_mm / 2.,
                max_segments: 256,
                max_iterations: 32,
                agreement_cells: remaining / 2,
            },
        )?;
        result.cells += projection.agreement_cells;
        result.projection_evaluations += projection.evaluations;
        if !projection.within_tolerance {
            return Err(refuse("Original support lift agreement is unproven"));
        }
        let pc = projection
            .pcurve
            .ok_or_else(|| refuse("Missing certified support pcurve"))?;
        let domain =
            crate::face_domain::FaceDomain::new(source, connector.source_face, tolerance_uv)?;
        let mut pending = vec![pc.domain()];
        while let Some(range) = pending.pop() {
            if result.cells >= max_cells {
                return Err(refuse("Rim membership work exhausted"));
            }
            result.cells += 1;
            let bounds = nurbs_core::interval_eval::evaluate_interval(
                &pc,
                nurbs_core::interval_eval::Interval::new(range[0], range[1])?,
            )?;
            if result.cells >= max_cells {
                return Err(refuse("No rim classification work remains"));
            }
            let classified = domain.classify(
                [[bounds[0].lo, bounds[0].hi], [bounds[1].lo, bounds[1].hi]],
                (max_cells - result.cells).min(4096),
            )?;
            result.cells += classified.cells;
            match classified.location {
                nurbs_core::trim_domain::Location::Inside => {}
                nurbs_core::trim_domain::Location::Outside => {
                    return Err(refuse("Connector leaves original support material"));
                }
                nurbs_core::trim_domain::Location::Unresolved => {
                    let middle = range[0] * 0.5 + range[1] * 0.5;
                    if !(range[0] < middle && middle < range[1]) {
                        return Err(refuse("Rim membership precision exhausted"));
                    }
                    pending.push([range[0], middle]);
                    pending.push([middle, range[1]]);
                }
            }
        }
        result.pcurves.push(pc);
    }
    Ok(result)
}
