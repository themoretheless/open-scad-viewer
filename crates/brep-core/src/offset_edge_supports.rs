//! Immutable original support faces for one authored B-rep edge.
//! Full face audits precede preparation; no fillet or topology edit is admitted.
use crate::{
    trimmed_face_recipe::{self, Boundary, Limits, QualifiedFace},
    Error, Model, Result, TopoId,
};
use cad_predicates::ToleranceContext;
use nurbs_core::curve::Curve;

pub struct Support {
    face_index: usize,
    face_id: TopoId,
    loop_index: usize,
    loop_slot: usize,
    cyclic_index: usize,
    face_reversed: bool,
    edge_ids: Vec<TopoId>,
    face: QualifiedFace,
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
    edge_index: usize,
    edge_id: TopoId,
    body_id: TopoId,
    shell_id: TopoId,
    curve: Curve,
    supports: [Support; 2],
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
    let loops: [Vec<Vec<Curve>>; 2] = std::array::from_fn(|side| {
        prepared.supports[side]
            .face
            .loops()
            .iter()
            .map(|wire| wire.coedges.iter().map(|c| c.pcurve.clone()).collect())
            .collect()
    });
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

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> Limits {
        Limits {
            pairs: 10000,
            region_cells: 100000,
            domain_cells: 100000,
            agreement_cells: 100000,
        }
    }
    #[test]
    fn all_box_edges_bind_two_original_faces_curves_and_ids_without_mutation() {
        let source = crate::cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
        let before = source.clone();
        for edge in 0..source.edges.len() {
            let r = prepare(&source, edge, 1e-8, limits()).unwrap();
            let prepared = r.prepared.expect(r.reason);
            assert_eq!(prepared.edge_id(), source.1.edges[edge]);
            assert_eq!(prepared.curve(), &source.edges[edge].curve);
            assert_ne!(
                prepared.supports()[0].face_index(),
                prepared.supports()[1].face_index()
            );
            for support in prepared.supports() {
                let coedge = &source.loops[support.loop_index()].coedges[support.cyclic_index()];
                assert_eq!(support.selected_pcurve(), &coedge.pcurve);
                assert_eq!(support.selected_reversed(), coedge.reversed);
                assert_eq!(support.face_id(), source.1.faces[support.face_index()]);
                assert_eq!(
                    support.face().face().surface,
                    source.faces[support.face_index()].surface
                );
                assert_eq!(support.edge_ids().len(), support.face().edges().len());
            }
        }
        assert_eq!(source, before);
    }
    #[test]
    fn rational_edge_keeps_original_curved_and_planar_supports() {
        let arc = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: vec![1., 0.5f64.sqrt(), 1.],
            periodic: false,
        };
        let profile = vec![
            arc,
            Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]]).unwrap(),
            Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap(),
        ];
        let source = crate::prism::extrude(&[profile], 0., 2.).unwrap();
        let edge = source
            .edges
            .iter()
            .position(|e| e.curve.degree == 2)
            .unwrap();
        let report = prepare(&source, edge, 1e-8, limits()).unwrap();
        let prepared = report.prepared.expect(report.reason);
        assert_eq!(prepared.curve(), &source.edges[edge].curve);
        assert!(prepared
            .supports()
            .iter()
            .any(|s| s.face().face().surface.degree_u == 2));
        assert!(prepared
            .supports()
            .iter()
            .any(|s| s.face().face().surface.degree_u == 1));
        assert!(report.support_audits.iter().all(|r| {
            r.agreements
                .iter()
                .all(|a| a.status == nurbs_core::curve_surface_agreement::Status::WithinTolerance)
        }));
        let distances = std::array::from_fn(|i| {
            if prepared.supports()[i].face_reversed() {
                0.2
            } else {
                -0.2
            }
        });
        let section = propose_section(&prepared, 0.5, distances, 1e-8, 8, 1e-3, 16).unwrap();
        assert!(section.residual_mm <= 1e-8);
        assert!(
            matches!(
                section.certificate,
                Some(nurbs_core::surface_contact::Verdict::Witness(_))
            ),
            "{}",
            section.reason
        );
        let path = propose_path(
            &prepared,
            section.fixed_axis,
            [0.2, 0.8],
            distances,
            63,
            nurbs_core::offset_contact_path::Limits {
                cells: 255,
                spans: 16,
                iterations: 8,
                numerical_tolerance_mm: 1e-8,
                padding_fraction: 0.01,
            },
        )
        .unwrap();
        assert!(path.driver.monotonic_proven);
        assert!(
            path.path.as_ref().unwrap().continuous_path_proven,
            "{}",
            path.reason
        );
        let trimmed = propose_trimmed_path(
            &prepared,
            section.fixed_axis,
            [0.2, 0.8],
            distances,
            63,
            1e-8,
            nurbs_core::offset_path_trims::Limits {
                path: nurbs_core::offset_contact_path::Limits {
                    cells: 255,
                    spans: 16,
                    iterations: 8,
                    numerical_tolerance_mm: 1e-8,
                    padding_fraction: 0.01,
                },
                membership_cells: 255,
                region_pairs: 10000,
                region_cells: 10000,
                domain_cells: 100000,
            },
        )
        .unwrap();
        assert!(
            trimmed
                .contact
                .as_ref()
                .unwrap()
                .trimmed_continuous_path_proven,
            "{}",
            trimmed.reason
        );
    }
    #[test]
    fn automatic_box_edge_stations_find_certified_centers_after_rotation() {
        let base = crate::cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
        let c = 0.6;
        let t = 0.8;
        let source = crate::transform::affine(
            &base,
            [
                [c, -t, 0., 5.],
                [t, c, 0., -3.],
                [0., 0., 1., 2.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        for edge in 0..source.edges.len() {
            let prepared = prepare(&source, edge, 1e-8, limits())
                .unwrap()
                .prepared
                .unwrap();
            let distances = std::array::from_fn(|i| {
                if prepared.supports()[i].face_reversed() {
                    0.2
                } else {
                    -0.2
                }
            });
            let section = propose_section(&prepared, 0.5, distances, 1e-8, 8, 1e-3, 16).unwrap();
            assert!(section.residual_mm <= 1e-8);
            assert!(
                matches!(
                    section.certificate,
                    Some(nurbs_core::surface_contact::Verdict::Witness(_))
                ),
                "{}",
                section.reason
            );
            assert!(section.iterations <= 8 && section.line_search_evaluations <= 96);
            let path = propose_path(
                &prepared,
                section.fixed_axis,
                [0.2, 0.8],
                distances,
                16,
                nurbs_core::offset_contact_path::Limits {
                    cells: 63,
                    spans: 16,
                    iterations: 8,
                    numerical_tolerance_mm: 1e-8,
                    padding_fraction: 0.01,
                },
            )
            .unwrap();
            assert!(path.driver.monotonic_proven);
            let trimmed = propose_trimmed_path(
                &prepared,
                section.fixed_axis,
                [0.2, 0.8],
                distances,
                16,
                1e-8,
                nurbs_core::offset_path_trims::Limits {
                    path: nurbs_core::offset_contact_path::Limits {
                        cells: 63,
                        spans: 16,
                        iterations: 8,
                        numerical_tolerance_mm: 1e-8,
                        padding_fraction: 0.01,
                    },
                    membership_cells: 127,
                    region_pairs: 10000,
                    region_cells: 10000,
                    domain_cells: 100000,
                },
            )
            .unwrap();
            assert!(
                trimmed
                    .contact
                    .as_ref()
                    .unwrap()
                    .trimmed_continuous_path_proven,
                "edge {}: {}",
                edge,
                trimmed.reason
            );

            assert!(
                path.path.as_ref().unwrap().continuous_path_proven,
                "edge {}: {}",
                edge,
                path.reason
            );
        }
    }
    #[test]
    fn numerical_center_does_not_gain_authority_when_interval_work_stops() {
        // C2 cubic carrier with two incident spans at the driving station.
        let line = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 0.5, 1., 1., 1., 1.],
            control_points: vec![
                vec![0., 0.],
                vec![1. / 3., 0.],
                vec![1., 0.],
                vec![5. / 3., 0.],
                vec![2., 0.],
            ],
            weights: vec![1.; 5],
            periodic: false,
        };
        let points = [vec![2., 0.], vec![2., 3.], vec![0., 3.], vec![0., 0.]];
        let mut profile = vec![line];
        profile.extend(
            points
                .windows(2)
                .map(|p| Curve::from_polyline(p.to_vec()).unwrap()),
        );
        let mut source = crate::prism::extrude(&[profile], 0., 4.).unwrap();
        // Extrusion splits profile spans into separate side faces. Retain a
        // multispan affine cap carrier so the root tube actually crosses a knot.
        for face in &mut source.faces {
            let surface = &mut face.surface;
            let z = surface.control_points[0][0][2];
            if surface.control_points.iter().flatten().all(|p| p[2] == z) {
                surface.degree_u = 3;
                surface.knots_u = vec![0., 0., 0., 0., 0.25, 1., 1., 1., 1.];
                surface.control_points = [0., 1. / 12., 5. / 12., 0.75, 1.]
                    .map(|u| vec![vec![2. * u, 0., z], vec![2. * u, 3., z]])
                    .to_vec();
                surface.weights = vec![vec![1.; 2]; 5];
            }
        }
        let edge = source
            .edges
            .iter()
            .position(|e| e.curve.degree == 3)
            .unwrap();
        let prepared = prepare(&source, edge, 1e-8, limits())
            .unwrap()
            .prepared
            .unwrap();
        let distances = std::array::from_fn(|i| {
            if prepared.supports()[i].face_reversed() {
                0.2
            } else {
                -0.2
            }
        });
        let section = propose_section(&prepared, 0.5, distances, 1e-8, 8, 1e-3, 1).unwrap();
        assert!(section.residual_mm <= 1e-8);
        assert!(
            matches!(
                section.certificate,
                Some(nurbs_core::surface_contact::Verdict::Unresolved)
            ),
            "{}: {:?}",
            section.reason,
            section.certificate
        );
        let sufficient = propose_section(&prepared, 0.5, distances, 1e-8, 8, 1e-3, 16).unwrap();
        assert!(
            matches!(
                sufficient.certificate,
                Some(nurbs_core::surface_contact::Verdict::Witness(_))
            ),
            "{}",
            sufficient.reason
        );
        assert!(propose_section(&prepared, -0.1, distances, 1e-8, 8, 1e-3, 16).is_err());
    }
    #[test]
    fn face_work_stop_and_missing_material_ownership_cannot_prepare_an_edge() {
        let mut source = crate::cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
        let mut work = limits();
        work.pairs = 1;
        work.region_cells = 1;
        work.domain_cells = 1;
        let r = prepare(&source, 0, 1e-8, work).unwrap();
        assert!(r.prepared.is_none());
        assert_eq!(r.support_audits.len(), 2);
        source.bodies.clear();
        source.rebuild_topology_ids();
        assert_eq!(
            prepare(&source, 0, 1e-8, limits()).err().unwrap().code,
            "BREP_OFFSET_EDGE_OWNERSHIP"
        );
    }
}
