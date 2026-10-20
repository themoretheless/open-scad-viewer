//! A common end boundary of a fillet skin and an original trimmed end face.
use crate::{Model, contact_trim::Restriction, face_domain::FaceDomain};
use nurbs_core::{
    Error, Result,
    curve::Curve,
    curve_surface_agreement,
    surface::Surface,
    surface_g2_pcurves::{self, Positions},
    trim_domain::Location,
};
pub struct EndCurve {
    pub curve: Curve,
    pub skin_pcurve: Curve,
    pub end_face_pcurve: Curve,
    pub endpoints: [[f64; 3]; 2],
    pub positions: Positions,
    pub cells: usize,
    pub exact_work: u64,
    pub projection_evaluations: usize,
}
fn invalid(s: &str) -> Error {
    Error::new("BREP_CONTACT_END_INPUT", s)
}
fn unproven(s: &str) -> Error {
    Error::new("BREP_CONTACT_END_UNPROVEN", s)
}
/// Connect matching ends of two restricted contacts. Stored root reports only
/// identify end/cutter selections: fresh whole-curve agreement and full material
/// membership are mandatory. No supplied report grants a certificate. Both
/// actual contact endpoint coordinates are retained byte-for-byte.
pub fn connect(
    contacts: [&Restriction; 2],
    end: usize,
    skin: &Surface,
    end_model: &Model,
    face_id: usize,
    tolerance_mm: f64,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<EndCurve> {
    end_model.validate()?;
    skin.validate()?;
    if end > 1
        || !(1..=100000).contains(&max_cells)
        || max_exact > 1000000
        || !tolerance_mm.is_finite()
        || tolerance_mm <= 0.
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(invalid(
            "Need end 0/1, positive tolerances and bounded shared work",
        ));
    }
    let face = end_model
        .faces
        .get(face_id)
        .ok_or_else(|| invalid("End face is missing"))?;
    if !end_model.persistent_naming_complete() {
        return Err(invalid("End face naming is incomplete"));
    }
    let mut endpoints = [[0.; 3]; 2];
    let mut skin_uv = Vec::new();
    for i in 0..2 {
        let contact = &contacts[i].contact;
        contact.curve.validate()?;
        contact.fillet_pcurve.validate()?;
        let c = &contact.curve;
        let d = c.domain();
        if c.periodic
            || c.control_points[0].len() != 3
            || !c.knots[..=c.degree].iter().all(|&t| t == d[0])
            || !c.knots[c.control_points.len()..].iter().all(|&t| t == d[1])
        {
            return Err(invalid(
                "End boundaries need clamped nonperiodic contact curves",
            ));
        }
        let p = &c.control_points[if end == 0 {
            0
        } else {
            c.control_points.len() - 1
        }];
        endpoints[i] = [p[0], p[1], p[2]];
        let pc = &contact.fillet_pcurve;
        let pd = pc.domain();
        if pc.periodic
            || !pc.knots[..=pc.degree].iter().all(|&t| t == pd[0])
            || !pc.knots[pc.control_points.len()..]
                .iter()
                .all(|&t| t == pd[1])
        {
            return Err(invalid("Contact endpoint chart needs clamped ownership"));
        }
        let uv = pc.control_points[if end == 0 {
            0
        } else {
            pc.control_points.len() - 1
        }]
        .clone();
        if uv.len() != 2 {
            return Err(invalid("Skin pcurve must be two-dimensional"));
        }
        skin_uv.push(uv);
        let report = &contacts[i].coverage[end];
        if report.support_face_identity != end_model.1.faces[face_id] {
            return Err(invalid(
                "End-face seed identity differs from the selected actual face",
            ));
        }
        let _root = report
            .eligible
            .iter()
            .map(|&j| &report.roots[j].root)
            .find(|r| {
                r.parameter_box[0][0] <= contacts[i].parameters[end]
                    && contacts[i].parameters[end] <= r.parameter_box[0][1]
            })
            .ok_or_else(|| invalid("No matching original end root seed"))?;
    }
    if endpoints[0] == endpoints[1] {
        return Err(unproven("The end boundary collapses"));
    }
    let skin_pcurve = Curve::from_polyline(skin_uv)?;
    let loops = std::iter::once(face.outer)
        .chain(face.holes.iter().copied())
        .map(|w| {
            end_model.loops[w]
                .coedges
                .iter()
                .map(|c| c.pcurve.clone())
                .collect()
        })
        .collect::<Vec<Vec<_>>>();
    if max_cells < 4 {
        return Err(unproven("No trim admission work remains"));
    }
    let stage = (max_cells / 4).min(100000);
    let trim = nurbs_core::trim_region_audit::inspect(&loops, tolerance_uv, 10000, stage, stage)?;
    let mut cells = trim.cells + trim.domain_cells;
    if trim.valid != Some(true) {
        return Err(unproven("End-face trim region is unproven"));
    }
    for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
        for c in &end_model.loops[wire].coedges {
            if cells == max_cells {
                return Err(unproven("End-face boundary budget exhausted"));
            }
            let r = curve_surface_agreement::verify(
                &end_model.edges[c.edge].curve,
                &c.pcurve,
                &face.surface,
                c.reversed,
                end_model.tolerance_mm,
                max_cells - cells,
            )?;
            cells += r.cells;
            if r.status != curve_surface_agreement::Status::WithinTolerance {
                return Err(unproven("Original end-face boundary lift is unproven"));
            }
        }
    }
    let lift = curve_surface_agreement::lift(
        &skin_pcurve,
        skin,
        Some(endpoints),
        tolerance_mm * 0.5,
        max_cells - cells,
    )?;
    cells += lift.cells;
    let curve = lift
        .curve
        .ok_or_else(|| unproven("End-curve construction/lift remains unresolved"))?;
    if cells == max_cells {
        return Err(unproven("No end-face pullback work remains"));
    }
    let projection = nurbs_core::curve_pullback::project(
        &curve,
        &face.surface,
        nurbs_core::curve_pullback::Limits {
            tolerance: tolerance_mm * 0.5,
            max_segments: 256,
            max_iterations: 32,
            agreement_cells: max_cells - cells,
        },
    )?;
    cells += projection.agreement_cells;
    let projection_evaluations = projection.evaluations;
    let end_face_pcurve = projection
        .pcurve
        .ok_or_else(|| unproven("Actual end-curve pullback remains unresolved"))?;
    let positions = surface_g2_pcurves::verify_positions(
        &curve,
        [&skin_pcurve, &end_face_pcurve],
        [skin, &face.surface],
        [false, false],
        tolerance_mm,
        max_cells - cells,
        max_exact,
    )?;
    cells += positions.cells;
    if !positions.proven {
        return Err(unproven(
            "Actual end curve does not agree with both original surfaces",
        ));
    }
    let domain = FaceDomain::new(end_model, face_id, tolerance_uv)?;
    let mut pending = vec![end_face_pcurve.domain()];
    while let Some(range) = pending.pop() {
        if cells == max_cells {
            return Err(unproven("Whole end-face membership work exhausted"));
        }
        cells += 1;
        let uv = nurbs_core::interval_eval::evaluate_interval(
            &end_face_pcurve,
            nurbs_core::interval_eval::Interval::new(range[0], range[1])?,
        )?;
        let bounds = std::array::from_fn(|k| {
            let lo = end_face_pcurve
                .control_points
                .iter()
                .map(|p| p[k])
                .fold(f64::INFINITY, f64::min);
            let hi = end_face_pcurve
                .control_points
                .iter()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max);
            [uv[k].lo.max(lo), uv[k].hi.min(hi)]
        });
        if cells == max_cells {
            return Err(unproven("No winding classification work remains"));
        }
        let r = domain.classify(bounds, (max_cells - cells).min(4096))?;
        cells += r.cells;
        match r.location {
            Location::Inside => {}
            Location::Outside => {
                return Err(unproven("End curve leaves the actual end-face material"));
            }
            Location::Unresolved => {
                let mid = range[0] * 0.5 + range[1] * 0.5;
                if !(range[0] < mid && mid < range[1]) {
                    return Err(unproven("End-face membership precision exhausted"));
                }
                pending.push([range[0], mid]);
                pending.push([mid, range[1]]);
            }
        }
    }
    let exact_work = positions.exact_work;
    Ok(EndCurve {
        curve,
        skin_pcurve,
        end_face_pcurve,
        endpoints,
        positions,
        cells,
        exact_work,
        projection_evaluations,
    })
}

/// One open fillet face with four genuinely shared corner vertex identities.
/// Retained support/end faces still need integration; this is not a solid.
pub struct Patch {
    pub model: Model,
    /// Start end, second contact, final end, first contact.
    pub edges: [usize; 4],
    pub ends: [EndCurve; 2],
    pub cells: usize,
    pub exact_work: u64,
}
/// Assemble the actual original skin plus all four qualified boundary uses.
/// Shared cells/exact work cover BOTH end constructions, trim orientation and
/// the four actual saved pcurves, including their rounded reversed definitions.
pub fn patch(
    contacts: [&Restriction; 2],
    skin: &Surface,
    ends: [(&Model, usize); 2],
    tolerance_mm: f64,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Patch> {
    if !(1..=100000).contains(&max_cells) || max_exact > 1000000 {
        return Err(invalid("Patch admission requires bounded shared work"));
    }
    let first = connect(
        contacts,
        0,
        skin,
        ends[0].0,
        ends[0].1,
        tolerance_mm,
        tolerance_uv,
        max_cells,
        max_exact,
    )?;
    let mut cells = first.cells;
    let mut exact_work = first.exact_work;
    if cells == max_cells {
        return Err(unproven("No final end-boundary work remains"));
    }
    let last = connect(
        contacts,
        1,
        skin,
        ends[1].0,
        ends[1].1,
        tolerance_mm,
        tolerance_uv,
        max_cells - cells,
        max_exact - exact_work,
    )?;
    cells += last.cells;
    exact_work += last.exact_work;
    let mut model = Model::empty(tolerance_mm)?;
    model.vertices = [
        first.endpoints[0],
        first.endpoints[1],
        last.endpoints[1],
        last.endpoints[0],
    ]
    .into_iter()
    .map(|point| crate::Vertex { point })
    .collect();
    let definitions = [
        first.curve.clone(),
        contacts[1].contact.curve.clone(),
        last.curve.clone(),
        contacts[0].contact.curve.clone(),
    ];
    let indices = [[0, 1], [1, 2], [3, 2], [0, 3]];
    model.edges = definitions
        .into_iter()
        .zip(indices)
        .map(|(curve, vertices)| crate::Edge {
            curve,
            vertices,
            degenerate: false,
        })
        .collect();
    model.loops.push(crate::Loop {
        coedges: vec![
            crate::Coedge {
                edge: 0,
                reversed: false,
                pcurve: first.skin_pcurve.clone(),
            },
            crate::Coedge {
                edge: 1,
                reversed: false,
                pcurve: contacts[1].contact.fillet_pcurve.clone(),
            },
            crate::Coedge {
                edge: 2,
                reversed: true,
                pcurve: last.skin_pcurve.reverse()?,
            },
            crate::Coedge {
                edge: 3,
                reversed: true,
                pcurve: contacts[0].contact.fillet_pcurve.reverse()?,
            },
        ],
    });
    model.faces.push(crate::Face {
        surface: skin.clone(),
        outer: 0,
        holes: vec![],
    });
    model.shells.push(crate::Shell {
        faces: vec![crate::FaceUse {
            face: 0,
            reversed: false,
        }],
        closed: false,
    });
    model.rebuild_topology_ids();
    if max_cells - cells < 2 {
        return Err(unproven("Patch trim budget exhausted"));
    }
    let stage = ((max_cells - cells) / 2).min(100000);
    let curves = model.loops[0]
        .coedges
        .iter()
        .map(|c| c.pcurve.clone())
        .collect::<Vec<_>>();
    let trim =
        nurbs_core::trim_region_audit::inspect(&[curves], tolerance_uv, 10000, stage, stage)?;
    cells += trim.cells + trim.domain_cells;
    if trim.valid != Some(true) || !matches!(trim.winding[0],Some(1)|Some(-1)) {
        return Err(unproven("Actual skin trim loop is invalid or unresolved"));
    }
    if trim.winding[0] == Some(-1) {
        model.loops[0].coedges.reverse();
        for c in &mut model.loops[0].coedges {
            c.reversed = !c.reversed;
            c.pcurve = c.pcurve.reverse()?;
        }
        if max_cells - cells < 2 {
            return Err(unproven("Reoriented patch trim budget exhausted"));
        }
        let stage=(max_cells-cells)/2;
        let curves=model.loops[0].coedges.iter().map(|c|c.pcurve.clone()).collect::<Vec<_>>();
        let fresh=nurbs_core::trim_region_audit::inspect(&[curves],tolerance_uv,10000,stage,stage)?;
        cells+=fresh.cells+fresh.domain_cells;
        if fresh.valid!=Some(true)||fresh.winding[0]!=Some(1) {
            return Err(unproven("Reoriented actual skin trim loop remains unproven"));
        }
    }
    model.validate()?;
    for c in &model.loops[0].coedges {
        let mut exact = false;
        if exact_work < max_exact {
            if let Some(r) = curve_surface_agreement::verify_exact(
                &model.edges[c.edge].curve,
                &c.pcurve,
                skin,
                c.reversed,
                max_exact - exact_work,
            )? {
                exact_work += r.work_used;
                exact = r.outcome == cad_predicates::BezierIdentity::Equal;
            }
        }
        if !exact {
            if cells == max_cells {
                return Err(unproven("Patch boundary agreement budget exhausted"));
            }
            let r = curve_surface_agreement::verify(
                &model.edges[c.edge].curve,
                &c.pcurve,
                skin,
                c.reversed,
                tolerance_mm,
                max_cells - cells,
            )?;
            cells += r.cells;
            if r.status != curve_surface_agreement::Status::WithinTolerance {
                return Err(unproven("Actual saved skin boundary lift is unproven"));
            }
        }
    }
    Ok(Patch {
        model,
        edges: [0, 1, 2, 3],
        ends: [first, last],
        cells,
        exact_work,
    })
}
