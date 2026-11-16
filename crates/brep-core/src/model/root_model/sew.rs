//! Topological sewing of oriented patches along shared original SS edges.
use super::*;

impl QualifiedRootShellPositiveFlux {
    /// Assemble an already selected boundary. The caller supplies every
    /// contribution from both operands. An all-discarded selection has no
    /// shell. Any retained faces must form one certified connected shell.
    pub fn assemble_boolean_selection(
        contributions: &[RootPeriodicBooleanContribution],
        tolerance_mm: f64,
        max_chart_spans: usize,
        censuses: Vec<([usize; 2], ss::QualifiedQuotientCoverage)>,
        max_separation_cells: usize,
        flux: RootShellFluxLimits,
    ) -> Result<Option<Self>> {
        if contributions.is_empty() {
            return Err(unproved("Boolean assembly requires classified contributions"));
        }
        if !tolerance_mm.is_finite() || tolerance_mm <= 0. {
            return Err(unproved("Boolean assembly requires positive finite tolerance"));
        }
        if contributions.iter().all(|face| face.patch().is_none()) {
            if !censuses.is_empty() {
                return Err(unproved("Empty Boolean selection has no faces for contact censuses"));
            }
            return Ok(None);
        }
        Self::from_boolean_contributions(
            contributions,
            tolerance_mm,
            max_chart_spans,
            censuses,
            max_separation_cells,
            flux,
        )
        .map(Some)
    }

    /// Sew the retained, oriented Boolean contributions into one certified
    /// connected shell. Census indices count retained faces only, in input
    /// order. Empty and disconnected results require separate assembly.
    pub fn from_boolean_contributions(
        contributions: &[RootPeriodicBooleanContribution],
        tolerance_mm: f64,
        max_chart_spans: usize,
        censuses: Vec<([usize; 2], ss::QualifiedQuotientCoverage)>,
        max_separation_cells: usize,
        flux: RootShellFluxLimits,
    ) -> Result<Self> {
        let patches: Vec<_> = contributions.iter().filter_map(|face| face.patch().cloned()).collect();
        if patches.is_empty() {
            return Err(unproved("Boolean contributions contain no retained shell faces"));
        }
        Self::from_patches(&patches, tolerance_mm, max_chart_spans, censuses, max_separation_cells, flux)
    }

    /// Sew oriented patches and prove the resulting shell's contacts and
    /// positive signed volume. Census face IDs follow patch, shell and face-use
    /// order, with each pair ordered by the census's original supports.
    pub fn from_patches(
        patches: &[Arc<RootBrep>],
        tolerance_mm: f64,
        max_chart_spans: usize,
        censuses: Vec<([usize; 2], ss::QualifiedQuotientCoverage)>,
        max_separation_cells: usize,
        flux: RootShellFluxLimits,
    ) -> Result<Self> {
        let shell = Arc::new(RootBrep::sew_root_shell(patches, tolerance_mm)?);
        let charts =
            QualifiedRootShellCharts::qualify(shell, max_chart_spans, flux.winding.geometry)?;
        let contacts =
            QualifiedRootShellContacts::qualify(charts, censuses, max_separation_cells)?;
        Self::qualify(contacts, flux)
    }
}

impl RootBrep {
    /// Sew a connected closed shell from oriented root patches. Shared edges
    /// must have the same graph, endpoint selectors, phases and clock bounds.
    /// The shell still requires embedding and material qualification.
    pub fn sew_root_shell(patches: &[Arc<Self>], tolerance_mm: f64) -> Result<Self> {
        let sewn = Self::sew_root_shells(patches, tolerance_mm)?;
        if sewn.topology().shells.len() != 1 {
            return Err(unproved("Root shell sewing needs one connected component"));
        }
        Ok(sewn)
    }

    /// Sew the edge-connected components independently. Each must be a
    /// closed oriented manifold. Cross-shell geometry and nesting require
    /// separate qualification before interpreting these shells as bodies.
    pub fn sew_root_shells(patches: &[Arc<Self>], tolerance_mm: f64) -> Result<Self> {
        let mut topology = RootTopology {
            vertices: Vec::new(),
            edges: Vec::new(),
            loops: Vec::new(),
            faces: Vec::new(),
            shells: Vec::new(),
            bodies: Vec::new(),
            tolerance_mm,
        };
        let mut faces = Vec::new();
        let mut joints = 0usize;
        for patch in patches {
            let source = patch.topology();
            for shell in &source.shells {
                for face_use in &shell.faces {
                    let face = &source.faces[face_use.face];
                    let mut loops = Vec::new();
                    for id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                        let mut coedges = Vec::new();
                        for c in &source.loops[id].coedges {
                            joints += 1;
                            if joints > 4096 {
                                return Err(unproved("Root shell sewing coedge budget exhausted"));
                            }
                            let (edge, swapped) =
                                insert_edge(&mut topology, &source.edges[c.edge].curve)?;
                            let reversed = c.reversed ^ swapped;
                            coedges.push(brep_topology::Coedge::new(
                                edge,
                                reversed,
                                RootPcurve {
                                    world_edge: edge,
                                    support: c.pcurve.support,
                                    reversed,
                                },
                            ));
                        }
                        loops.push(topology.loops.len());
                        topology.loops.push(brep_topology::Loop { coedges });
                    }
                    faces.push(brep_topology::FaceUse {
                        face: topology.faces.len(),
                        reversed: face_use.reversed,
                    });
                    topology.faces.push(brep_topology::Face {
                        surface: face.surface.clone(),
                        outer: loops[0],
                        holes: loops[1..].to_vec(),
                    });
                }
            }
        }
        if faces.is_empty() {
            return Err(unproved("Root shell sewing needs at least one face"));
        }
        let mut edge_faces = vec![Vec::new(); topology.edges.len()];
        for (id, face) in topology.faces.iter().enumerate() {
            for contour in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                for coedge in &topology.loops[contour].coedges {
                    edge_faces[coedge.edge].push(id);
                }
            }
        }
        let mut neighbors = vec![Vec::new(); faces.len()];
        for uses in edge_faces {
            let [a, b] = uses.as_slice() else {
                return Err(unproved("Root shell sewing requires two uses per edge"));
            };
            neighbors[*a].push(*b);
            neighbors[*b].push(*a);
        }
        let mut visited = vec![false; faces.len()];
        for start in 0..faces.len() {
            if std::mem::replace(&mut visited[start], true) {
                continue;
            }
            let mut pending = vec![start];
            let mut component = Vec::new();
            while let Some(face) = pending.pop() {
                component.push(face);
                for &neighbor in &neighbors[face] {
                    if !std::mem::replace(&mut visited[neighbor], true) {
                        pending.push(neighbor);
                    }
                }
            }
            component.sort_unstable();
            topology.shells.push(brep_topology::Shell {
                faces: component.into_iter().map(|id| faces[id].clone()).collect(),
                closed: true,
            });
        }
        let sewn = Arc::new(Self::from_topology(topology)?);
        QualifiedRootPeriodicUvClosure::qualify(sewn.clone(), joints)?;
        Ok(Arc::unwrap_or_clone(sewn))
    }
}

fn same_port(a: &ss::QualifiedOriginalSsPort, b: &ss::QualifiedOriginalSsPort) -> bool {
    let same_witness = match (a.third_surface_binding(), b.third_surface_binding()) {
        (None, None) => true,
        (Some((a, ar)), Some((b, br))) => Arc::ptr_eq(a, b) && ar == br,
        _ => false,
    };
    same_witness
        && a.kind() == b.kind()
        && a.point().same_selector(b.point())
        && a.phase() == b.phase()
        && a.parameter_bounds() == b.parameter_bounds()
}

fn insert_edge(
    topology: &mut RootTopology,
    edge: &ss::OriginalSsRootEdge,
) -> Result<(usize, bool)> {
    for (id, old) in topology.edges.iter().enumerate() {
        let old = &old.curve;
        let same = same_port(old.start(), edge.start()) && same_port(old.end(), edge.end());
        let swapped = same_port(old.start(), edge.end()) && same_port(old.end(), edge.start());
        if (same || swapped) && old.curve().same_definition(edge.curve())? {
            return Ok((id, !same));
        }
    }
    let vertices = [edge.start(), edge.end()].map(|port| {
        if let Some(id) = topology
            .vertices
            .iter()
            .position(|v| v.point.selector.same_selector(port.point()))
        {
            id
        } else {
            let id = topology.vertices.len();
            topology.vertices.push(brep_topology::Vertex {
                point: RootVertex {
                    selector: port.point().clone(),
                },
            });
            id
        }
    });
    let id = topology.edges.len();
    topology.edges.push(brep_topology::Edge {
        vertices,
        degenerate: false,
        curve: edge.clone(),
    });
    Ok((id, false))
}
