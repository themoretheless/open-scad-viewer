use super::*;

pub struct Support {
    pub(super) face_index: usize,
    pub(super) face_id: TopoId,
    pub(super) loop_index: usize,
    pub(super) loop_slot: usize,
    pub(super) cyclic_index: usize,
    pub(super) face_reversed: bool,
    pub(super) edge_ids: Vec<TopoId>,
    pub(super) face: QualifiedFace,
}
impl Support {
    pub fn face_index(&self) -> usize {
        self.face_index
    }
    pub fn face_id(&self) -> TopoId {
        self.face_id
    }
    pub fn loop_index(&self) -> usize {
        self.loop_index
    }
    pub fn loop_slot(&self) -> usize {
        self.loop_slot
    }
    pub fn cyclic_index(&self) -> usize {
        self.cyclic_index
    }
    pub fn face_reversed(&self) -> bool {
        self.face_reversed
    }
    pub fn edge_ids(&self) -> &[TopoId] {
        &self.edge_ids
    }
    pub fn face(&self) -> &QualifiedFace {
        &self.face
    }
    pub fn selected_pcurve(&self) -> &Curve {
        &self.face.loops()[self.loop_slot].coedges[self.cyclic_index].pcurve
    }
    pub fn selected_reversed(&self) -> bool {
        self.face.loops()[self.loop_slot].coedges[self.cyclic_index].reversed
    }
}
pub struct PreparedEdge {
    pub(super) edge_index: usize,
    pub(super) edge_id: TopoId,
    pub(super) body_id: TopoId,
    pub(super) shell_id: TopoId,
    pub(super) curve: Curve,
    pub(super) supports: [Support; 2],
}
impl PreparedEdge {
    pub fn edge_index(&self) -> usize {
        self.edge_index
    }
    pub fn edge_id(&self) -> TopoId {
        self.edge_id
    }
    pub fn body_id(&self) -> TopoId {
        self.body_id
    }
    pub fn shell_id(&self) -> TopoId {
        self.shell_id
    }
    pub fn curve(&self) -> &Curve {
        &self.curve
    }
    pub fn supports(&self) -> &[Support; 2] {
        &self.supports
    }
}
pub struct Report {
    pub support_audits: Vec<trimmed_face_recipe::Report>,
    pub prepared: Option<PreparedEdge>,
    pub reason: &'static str,
}
#[derive(Clone, Copy)]
struct Address {
    body: usize,
    shell: usize,
    face: usize,
    wire: usize,
    slot: usize,
    cyclic: usize,
    face_reversed: bool,
    edge_reversed: bool,
}
pub fn prepare(
    model: &Model,
    edge_index: usize,
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report> {
    model.validate()?;
    let edge = model
        .edges
        .get(edge_index)
        .ok_or_else(|| Error::new("BREP_OFFSET_EDGE_INDEX", "Select an existing authored edge"))?;
    let mut uses = vec![];
    for (body_index, body) in model.bodies.iter().enumerate() {
        for shell_index in
            std::iter::once(body.outer_shell).chain(body.inner_shells.iter().copied())
        {
            let shell = &model.shells[shell_index];
            for usage in &shell.faces {
                let face = &model.faces[usage.face];
                for (slot, wire) in std::iter::once(face.outer)
                    .chain(face.holes.iter().copied())
                    .enumerate()
                {
                    for (cyclic, coedge) in model.loops[wire].coedges.iter().enumerate() {
                        if coedge.edge == edge_index {
                            uses.push(Address {
                                body: body_index,
                                shell: shell_index,
                                face: usage.face,
                                wire,
                                slot,
                                cyclic,
                                face_reversed: usage.reversed,
                                edge_reversed: coedge.reversed,
                            });
                        }
                    }
                }
            }
        }
    }
    if uses.len() != 2
        || uses[0].body != uses[1].body
        || uses[0].shell != uses[1].shell
        || uses[0].face == uses[1].face
        || !model.shells[uses[0].shell].closed
    {
        return Err(Error::new(
            "BREP_OFFSET_EDGE_OWNERSHIP",
            "Edge preparation needs two distinct faces in one closed material shell",
        ));
    }
    if (uses[0].face_reversed ^ uses[0].edge_reversed)
        == (uses[1].face_reversed ^ uses[1].edge_reversed)
    {
        return Err(Error::new(
            "BREP_OFFSET_EDGE_ORIENTATION",
            "Adjacent material faces must traverse the shared edge oppositely",
        ));
    }
    let context = ToleranceContext::from_brep_tolerance_mm(model.tolerance_mm).map_err(|e| {
        Error::new(
            "BREP_OFFSET_EDGE_TOLERANCE",
            format!("Invalid source tolerance: {e:?}"),
        )
    })?;
    let mut out = Report {
        support_audits: vec![],
        prepared: None,
        reason: "source-face-unqualified",
    };
    let mut identities = vec![];
    for address in &uses {
        let face = &model.faces[address.face];
        let mut ids = vec![];
        let wires = std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .map(|wire| {
                model.loops[wire]
                    .coedges
                    .iter()
                    .map(|c| {
                        ids.push(model.1.edges[c.edge]);
                        Boundary {
                            curve: model.edges[c.edge].curve.clone(),
                            pcurve: c.pcurve.clone(),
                            reversed: c.reversed,
                        }
                    })
                    .collect()
            })
            .collect::<Vec<Vec<Boundary>>>();
        out.support_audits.push(trimmed_face_recipe::assemble(
            &context,
            &face.surface,
            &wires,
            tolerance_uv,
            limits,
        )?);
        identities.push(ids);
    }
    if out.support_audits.iter().any(|r| r.face.is_none()) {
        return Ok(out);
    }
    let supports = std::array::from_fn(|i| {
        let a = uses[i];
        Support {
            face_index: a.face,
            face_id: model.1.faces[a.face],
            loop_index: a.wire,
            loop_slot: a.slot,
            cyclic_index: a.cyclic,
            face_reversed: a.face_reversed,
            edge_ids: identities[i].clone(),
            face: out.support_audits[i].face.as_ref().unwrap().clone(),
        }
    });
    out.prepared = Some(PreparedEdge {
        edge_index,
        edge_id: model.1.edges[edge_index],
        body_id: model.1.bodies[uses[0].body],
        shell_id: model.1.shells[uses[0].shell],
        curve: edge.curve.clone(),
        supports,
    });
    out.reason = "original-edge-supports-qualified";
    Ok(out)
}
/// A world-edge station proposes a section using its original forward/reversed
/// source pcurves. Axis choice is local and numerical, not a whole-edge driver proof.
/// Signed distances are explicit; source ownership alone does not prove material side.
pub fn propose_section(
    prepared: &PreparedEdge,
    fraction: f64,
    distances: [f64; 2],
    numerical_tolerance_mm: f64,
    max_iterations: usize,
    padding_fraction: f64,
    max_spans: usize,
) -> Result<nurbs_core::offset_contact_predictor::Report> {
    if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
        return Err(Error::new(
            "BREP_OFFSET_EDGE_STATION",
            "Choose an edge station fraction in [0,1]",
        ));
    }
    let mut seeds = [[0.; 2]; 2];
    let mut derivative = None;
    for (side, support) in prepared.supports.iter().enumerate() {
        let curve = support.selected_pcurve();
        let d = curve.domain();
        let t = if support.selected_reversed() {
            1. - fraction
        } else {
            fraction
        };
        let value = curve.evaluate(d[0] + (d[1] - d[0]) * t)?;
        seeds[side] = [value.point[0], value.point[1]];
        if side == 0 {
            derivative = value.d1;
        }
    }
    let derivative = derivative.ok_or_else(|| {
        Error::new(
            "BREP_OFFSET_EDGE_DRIVER",
            "Source pcurve tangent is unavailable at this station",
        )
    })?;
    let surface = &prepared.supports[0].face.face().surface;
    let widths = [
        surface.knots_u[surface.control_points.len()] - surface.knots_u[surface.degree_u],
        surface.knots_v[surface.control_points[0].len()] - surface.knots_v[surface.degree_v],
    ];
    let scores = [
        derivative[0].abs() / widths[0],
        derivative[1].abs() / widths[1],
    ];
    if !scores.iter().all(|x| x.is_finite()) || scores.iter().all(|x| *x == 0.) {
        return Err(Error::new(
            "BREP_OFFSET_EDGE_DRIVER",
            "Source pcurve has no regular numerical driving coordinate at this station",
        ));
    }
    let axis = usize::from(scores[1] > scores[0]);
    nurbs_core::offset_contact_predictor::propose(
        [
            &prepared.supports[0].face.face().surface,
            &prepared.supports[1].face.face().surface,
        ],
        distances,
        seeds,
        axis,
        numerical_tolerance_mm,
        max_iterations,
        padding_fraction,
        max_spans,
    )
}
pub struct PathReport {
    pub driver: nurbs_core::curve_axis_driver::Report,
    pub path: Option<nurbs_core::offset_contact_path::Report>,
    pub reason: &'static str,
}
struct DriverSeeds {
    driver: nurbs_core::curve_axis_driver::Report,
    seeds: Option<[[[f64; 2]; 2]; 2]>,
}
fn driver_seeds(
    prepared: &PreparedEdge,
    axis: usize,
    drive: [f64; 2],
    driver_cells: usize,
) -> Result<DriverSeeds> {
    if axis >= 2 || !drive.iter().all(|x| x.is_finite()) || drive[0] >= drive[1] {
        return Err(Error::new(
            "BREP_OFFSET_EDGE_DRIVER",
            "Choose a finite positive source-coordinate interval",
        ));
    }
    let first = prepared.supports[0].selected_pcurve();
    let driver = nurbs_core::curve_axis_driver::certify(first, axis, driver_cells)?;
    if !driver.monotonic_proven {
        return Ok(DriverSeeds {
            driver,
            seeds: None,
        });
    }
    if drive[0] < driver.axis_extent[0] || drive[1] > driver.axis_extent[1] {
        return Err(Error::new(
            "BREP_OFFSET_EDGE_DRIVER",
            "Requested interval exceeds the original edge's source-coordinate extent",
        ));
    }
    let mut ends = [[[0.; 2]; 2]; 2];
    for (end, pair) in ends.iter_mut().enumerate() {
        let first_end = if driver.increasing { end } else { 1 - end };
        let first_point = if first_end == 0 {
            &first.control_points[0]
        } else {
            first.control_points.last().unwrap()
        };
        pair[0] = [first_point[0], first_point[1]];
        let canonical_end = if prepared.supports[0].selected_reversed() {
            1 - first_end
        } else {
            first_end
        };
        let second_end = if prepared.supports[1].selected_reversed() {
            1 - canonical_end
        } else {
            canonical_end
        };
        let second = prepared.supports[1].selected_pcurve();
        let point = second.evaluate(second.domain()[second_end])?.point;
        pair[1] = [point[0], point[1]];
    }
    let seeds = drive.map(|x| {
        let fraction =
            (x - driver.axis_extent[0]) / (driver.axis_extent[1] - driver.axis_extent[0]);
        let mut seed = std::array::from_fn(|side| {
            std::array::from_fn(|k| {
                ends[0][side][k] * (1. - fraction) + ends[1][side][k] * fraction
            })
        });
        seed[0][axis] = x;
        seed
    });
    Ok(DriverSeeds {
        driver,
        seeds: Some(seeds),
    })
}
/// Prove an original source-coordinate driver and a connected offset path.
/// This does not infer material side, trimmed membership or fillet topology.
pub fn propose_path(
    prepared: &PreparedEdge,
    axis: usize,
    drive: [f64; 2],
    distances: [f64; 2],
    driver_cells: usize,
    limits: nurbs_core::offset_contact_path::Limits,
) -> Result<PathReport> {
    let DriverSeeds { driver, seeds } = driver_seeds(prepared, axis, drive, driver_cells)?;
    let Some(seeds) = seeds else {
        return Ok(PathReport {
            driver,
            path: None,
            reason: "original-pcurve-driver-unresolved",
        });
    };
    let path = nurbs_core::offset_contact_path::certify(
        [
            &prepared.supports[0].face.face().surface,
            &prepared.supports[1].face.face().surface,
        ],
        distances,
        axis,
        drive,
        seeds,
        limits,
    )?;
    let reason = path.reason;
    Ok(PathReport {
        driver,
        path: Some(path),
        reason,
    })
}

pub struct TrimmedPathReport {
    pub driver: nurbs_core::curve_axis_driver::Report,
    pub contact: Option<nurbs_core::offset_path_trims::Report>,
    pub reason: &'static str,
}
/// Fresh driver, connected root and full membership checks on both unchanged
/// original face regions. This does not construct replacement trims or a solid.
pub fn propose_trimmed_path(
    prepared: &PreparedEdge,
    axis: usize,
    drive: [f64; 2],
    distances: [f64; 2],
    driver_cells: usize,
    tolerance_uv: f64,
    limits: nurbs_core::offset_path_trims::Limits,
) -> Result<TrimmedPathReport> {
    let DriverSeeds { driver, seeds } = driver_seeds(prepared, axis, drive, driver_cells)?;
    let Some(seeds) = seeds else {
        return Ok(TrimmedPathReport {
            driver,
            contact: None,
            reason: "original-pcurve-driver-unresolved",
        });
    };
    let loops = original_loops(prepared);
    let contact = nurbs_core::offset_path_trims::certify(
        [
            &prepared.supports[0].face.face().surface,
            &prepared.supports[1].face.face().surface,
        ],
        [&loops[0], &loops[1]],
        distances,
        axis,
        drive,
        seeds,
        tolerance_uv,
        limits,
    )?;
    let reason = contact.reason;
    Ok(TrimmedPathReport {
        driver,
        contact: Some(contact),
        reason,
    })
}

fn original_loops(prepared: &PreparedEdge) -> [Vec<Vec<Curve>>; 2] {
    std::array::from_fn(|side| {
        prepared.supports[side]
            .face
            .loops()
            .iter()
            .map(|wire| wire.coedges.iter().map(|c| c.pcurve.clone()).collect())
            .collect()
    })
}
pub struct PcurveReport {
    pub driver: nurbs_core::curve_axis_driver::Report,
    pub pcurves: Option<nurbs_core::offset_path_pcurves::Report>,
    pub reason: &'static str,
}
/// Generate shared UV contact pieces against fresh immutable original supports.
/// Full UV branch and trim checks qualify only an approximation in source
/// parameter units, not world coedge identity, radius, G1 or fillet topology.
pub fn propose_source_pcurves(
    prepared: &PreparedEdge,
    axis: usize,
    drive: [f64; 2],
    distances: [f64; 2],
    driver_cells: usize,
    tolerance_uv: f64,
    limits: nurbs_core::offset_path_pcurves::Limits,
) -> Result<PcurveReport> {
    let DriverSeeds { driver, seeds } = driver_seeds(prepared, axis, drive, driver_cells)?;
    let Some(seeds) = seeds else {
        return Ok(PcurveReport {
            driver,
            pcurves: None,
            reason: "original-pcurve-driver-unresolved",
        });
    };
    let loops = original_loops(prepared);
    let pcurves = nurbs_core::offset_path_pcurves::propose(
        [
            &prepared.supports[0].face.face().surface,
            &prepared.supports[1].face.face().surface,
        ],
        [&loops[0], &loops[1]],
        distances,
        axis,
        drive,
        seeds,
        tolerance_uv,
        limits,
    )?;
    let reason = pcurves.reason;
    Ok(PcurveReport {
        driver,
        pcurves: Some(pcurves),
        reason,
    })
}

pub struct WorldContactReport {
    pub pcurves: PcurveReport,
    pub lifts: Vec<nurbs_core::curve_surface_lift::Report>,
    pub source_scale_upper: [Option<f64>; 2],
    pub contact_error_upper_mm: [Option<f64>; 2],
    pub world_contacts_proven: bool,
    pub reason: &'static str,
}
