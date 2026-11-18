//! Actual central patch incidence with explicit shared root ownership.
//! This is an OPEN shell, never a Solid, volume or curvature certificate.
use super::*;
/// An original support binding PROPOSAL; attaching this support still requires
/// fresh complete edge/lift and trim checks on the final assembled model.
pub struct NetworkSupportBinding {
    pub boundary: usize,
    pub patch: usize,
    pub edge: usize,
    pub parameter: [f64; 2],
    pub support: Surface,
    pub pcurve: Curve,
}
pub struct OpenCornerNetwork {
    model: Model,
    shared_edges: Vec<usize>,
    pole_edges: [usize; 2],
    bindings: Vec<NetworkSupportBinding>,
    support_agreements: Vec<nurbs_core::curve_surface_agreement::Report>,
    pub cells: usize,
}
impl OpenCornerNetwork {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn shared_edges(&self) -> &[usize] {
        &self.shared_edges
    }
    pub fn pole_edges(&self) -> [usize; 2] {
        self.pole_edges
    }
    pub fn support_binding_proposals(&self) -> &[NetworkSupportBinding] {
        &self.bindings
    }
    /// Whole-edge G0 on the immutable support-binding proposals above. Final
    /// stock/strip trimming and closed-body admission remain separate.
    pub fn support_agreements(&self) -> &[nurbs_core::curve_surface_agreement::Report] {
        &self.support_agreements
    }
}
impl CentralLoftCandidate<'_, '_, '_> {
    /// Assemble actual patch boundaries, with each internal 3D root stored ONCE
    /// and used with opposite traversal by its two owning faces. Outer support
    /// trims are not part of this open-shell construction.
    pub fn assemble_open_network(
        &self,
        patches: &[Surface],
        max_cells: usize,
        max_exact: u64,
    ) -> Result<OpenCornerNetwork> {
        require(
            (2..=32).contains(&patches.len())
                && (100..=100000).contains(&max_cells)
                && max_exact <= 1000000,
            "Bound open central network assembly",
        )?;
        self.reverify()?;
        let pieces = self.network_exterior_pieces(patches, max_cells)?;
        require(
            pieces.len() == 2 * patches.len(),
            "Each central cell needs two owned exterior intervals",
        )?;
        for s in patches {
            s.validate()?;
            require(
                s.control_points.iter().flatten().all(|p| {
                    p.len() == 3
                        && (0..3).all(|k| self.stock[k][0] <= p[k] && p[k] <= self.stock[k][1])
                }),
                "Central network control hull leaves source stock",
            )?;
            for col in [0, s.degree_u] {
                require(
                    s.weights[col].iter().all(|&w| w == s.weights[col][0]),
                    "Shared root lost its literal Cartesian V clock",
                )?;
            }

            require(
                crate::rounded_patch_network::clamped_single(
                    &s.knots_u,
                    s.degree_u,
                    s.control_points.len(),
                ) && crate::rounded_patch_network::clamped_single(
                    &s.knots_v,
                    s.degree_v,
                    s.control_points[0].len(),
                ),
                "Central network cells must retain full clamped charts",
            )?;
        }
        for pair in patches.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            require(
                a.knots_u[a.degree_u + 1] == b.knots_u[b.degree_u]
                    && a.knots_v == b.knots_v
                    && a.control_points.last() == b.control_points.first(),
                "Central shared root lost its literal ownership",
            )?;
        }
        let exterior =
            self.inspect_network_exterior_support_jets(patches, max_cells / 2, max_exact)?;
        let mut cells = 0;
        for entry in exterior {
            let r = entry.report?;
            cells += r.cells;
            require(
                r.positions.proven
                    && r.regularity_proven
                    && r.pending == 0
                    && r.decision == nurbs_core::surface_join::Decision::WithinTolerance,
                "Actual network exterior G2 remains unproven",
            )?;
        }
        for (i, pair) in patches.windows(2).enumerate() {
            require(
                cells < max_cells,
                "Central network G2 exhausted its cell allowance",
            )?;
            let r = nurbs_core::surface_g2_pcurves::inspect_original_patch_pair(
                &pair[0],
                &pair[1],
                self.options.tolerances[1],
                self.options.tolerances[2],
                (max_cells - cells) / (patches.len() - 1 - i),
            )?;
            cells += r.cells;
            require(
                r.regularity_proven
                    && r.pending == 0
                    && r.decision == nurbs_core::surface_join::Decision::WithinTolerance,
                "Actual network internal G2 remains unproven",
            )?;
        }
        let mut model = Model(
            brep_topology::Model {
                vertices: vec![],
                edges: vec![],
                loops: vec![],
                faces: vec![],
                shells: vec![],
                bodies: vec![],
                tolerance_mm: self.body.source().tolerance_mm,
            },
            crate::TopologyIds::default(),
        );
        let mut roots = Vec::new();
        let mut root_edges = Vec::new();
        for i in 0..=patches.len() {
            let (s, col) = if i == 0 {
                (&patches[0], 0)
            } else {
                (&patches[i - 1], patches[i - 1].degree_u)
            };
            let points = &s.control_points[col];
            let weights = &s.weights[col];
            require(
                weights.iter().all(|&w| w == weights[0]),
                "Central root must keep its Cartesian V clock",
            )?;
            let low = topology::vertex(
                &mut model,
                points[0]
                    .as_slice()
                    .try_into()
                    .map_err(|_| refuse("Central root is not 3D"))?,
            );
            let collapsed = points.iter().all(|p| p == &points[0]);
            require(
                collapsed == (i == 0 || i == patches.len()),
                "Unexpected collapsed internal root or missing end pole",
            )?;
            let high = if collapsed {
                low
            } else {
                topology::vertex(
                    &mut model,
                    points
                        .last()
                        .unwrap()
                        .as_slice()
                        .try_into()
                        .map_err(|_| refuse("Central root is not 3D"))?,
                )
            };
            let curve = if collapsed {
                Curve::from_polyline(vec![points[0].clone(); 2])?
            } else {
                Curve {
                    degree: s.degree_v,
                    knots: s.knots_v.clone(),
                    control_points: points.clone(),
                    weights: vec![1.; points.len()],
                    periodic: false,
                }
            };
            let edge = topology::edge(&mut model, curve, [low, high], collapsed);
            roots.push([low, high]);
            root_edges.push(edge);
        }
        let mut bindings = Vec::new();
        for (i, s) in patches.iter().enumerate() {
            let mut wire = crate::Loop {
                coedges: Vec::new(),
            };
            for side in 0..4 {
                let (curve, pc) = crate::rounded_patch_network::boundary(s, side);
                if side == 1 || side == 3 {
                    let root = if side == 1 { i + 1 } else { i };
                    wire.coedges
                        .push(crate::Coedge::new(root_edges[root], side == 3, pc));
                } else {
                    let v = if side == 0 {
                        s.knots_v[s.degree_v]
                    } else {
                        s.knots_v[s.degree_v + 1]
                    };
                    let candidates = pieces
                        .iter()
                        .filter(|(boundary, patch, _)| {
                            *patch == i && self.pcurves[*boundary].control_points[0][1] == v
                        })
                        .collect::<Vec<_>>();
                    require(
                        candidates.len() == 1,
                        "Central side lost its original support owner",
                    )?;
                    let (boundary, _, parameter) = *candidates[0];
                    let (lo, hi) = if side == 0 {
                        (roots[i][0], roots[i + 1][0])
                    } else {
                        (roots[i + 1][1], roots[i][1])
                    };
                    let edge = topology::edge(&mut model, curve, [lo, hi], false);
                    let mut source_pc = if boundary % 2 == 1 {
                        // Keep the actual shared 3D edge's rational clock on
                        // its original affine support; no linear refit.
                        planar::project(&model.edges[edge].curve, &self.originals[boundary].0)?
                    } else {
                        self.originals[boundary]
                            .1
                            .trim(parameter[0], parameter[1])?
                    };
                    let forward = self.pcurves[boundary].control_points[0][0]
                        < self.pcurves[boundary].control_points[1][0];
                    if boundary % 2 == 0 && forward != (side == 0) {
                        // Both full affine poles are reversed; preserve their
                        // original knot endpoints, avoiding rounded reflection.
                        source_pc.control_points.reverse();
                        source_pc.weights.reverse();
                    }
                    bindings.push(NetworkSupportBinding {
                        boundary,
                        patch: i,
                        edge,
                        parameter,
                        support: self.originals[boundary].0.clone(),
                        pcurve: source_pc,
                    });
                    wire.coedges.push(crate::Coedge::new(edge, false, pc));
                }
            }
            let outer = model.loops.len();
            model.loops.push(wire);
            model.faces.push(crate::Face {
                surface: s.clone(),
                outer,
                holes: vec![],
            });
        }
        model.shells.push(crate::Shell {
            faces: (0..patches.len())
                .map(|face| crate::FaceUse {
                    face,
                    reversed: false,
                })
                .collect(),
            closed: false,
        });
        let mut support_agreements = Vec::new();
        for (i, binding) in bindings.iter().enumerate() {
            let allowance = (max_cells - cells) / (bindings.len() - i);
            require(
                allowance > 0,
                "No actual network/source binding work remains",
            )?;
            let proof = nurbs_core::curve_surface_agreement::verify(
                &model.edges[binding.edge].curve,
                &binding.pcurve,
                &binding.support,
                false,
                self.options.tolerances[0],
                allowance,
            )?;
            cells += proof.cells;
            require(
                proof.status == nurbs_core::curve_surface_agreement::Status::WithinTolerance,
                "Actual shared edge/source-surface agreement remains unproven",
            )?;
            support_agreements.push(proof);
        }
        model.inherit_topology_ids(&[self.body.source()]);
        model.validate()?;
        let pole_edges = [root_edges[0], *root_edges.last().unwrap()];
        Ok(OpenCornerNetwork {
            model,
            support_agreements,
            shared_edges: root_edges[1..root_edges.len() - 1].to_vec(),
            pole_edges,
            bindings,
            cells,
        })
    }
}
