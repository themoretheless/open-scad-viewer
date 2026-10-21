//! Exact boundary incidence on the actual original support quotient.
//! Multiple material fans remain explicit; no manifold face is inferred here.
use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSurfaceEndpointUse {
    pub trace: usize,
    pub end: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSurfaceBoundaryFan {
    pub component: usize,
    pub walk: usize,
    pub incoming: usize,
    pub outgoing: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSurfaceIncidenceVertex {
    /// Original physical quotient vertex, not a rounded world-space bucket.
    pub source_vertex: usize,
    pub endpoints: Vec<OriginalPeriodicSurfaceEndpointUse>,
    /// Actual source-sector successor pairs. Distinct fans are never merged.
    pub fans: Vec<OriginalPeriodicSurfaceBoundaryFan>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSurfaceIncidenceEdge {
    pub trace: usize,
    pub vertices: [usize; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSurfaceIncidenceLoop {
    pub component: usize,
    pub walk: usize,
    pub traces: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Data {
    vertices: Vec<OriginalPeriodicSurfaceIncidenceVertex>,
    edges: Vec<OriginalPeriodicSurfaceIncidenceEdge>,
    loops: Vec<OriginalPeriodicSurfaceIncidenceLoop>,
    entities: usize,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSupportIncidence {
    owner: Arc<OwnedPeriodicSupportTraces>,
    max_entities: usize,
    data: Data,
}
fn derive(o: &OwnedPeriodicSupportIncidence) -> Result<Data> {
    let _replay = super::periodic_replay::scope();
    if !(1..=1000000).contains(&o.max_entities) {
        return Err(refused("Invalid original boundary incidence allowance"));
    }
    super::periodic_replay::verify_arc(&o.owner, |owner| owner.reverify_source())?;
    let lifts = o.owner.owner().owner().owner();
    let cycles = lifts.owner();
    let ports = cycles.rotation().owner();
    let traces = o.owner.traces();
    // Edges + two endpoint uses + one successor fan per trace, plus loops.
    // Check this census BEFORE allocating the derived records and lookup maps.
    let mut entities = traces
        .len()
        .checked_mul(4)
        .and_then(|n| n.checked_add(lifts.walks().len()))
        .filter(|n| *n <= o.max_entities)
        .ok_or_else(|| refused("Complete original boundary incidence exceeds allowance"))?;
    let mut lookup = BTreeMap::new();
    let mut vertex_map = BTreeMap::new();
    let mut vertices: Vec<OriginalPeriodicSurfaceIncidenceVertex> = Vec::new();
    let mut edges = Vec::with_capacity(traces.len());
    for (trace, t) in traces.iter().enumerate() {
        if lookup.insert((t.walk, t.part), trace).is_some() {
            return Err(refused("Boundary incidence repeats an original trace"));
        }
        let p = ports
            .ports()
            .get(t.original.dart)
            .ok_or_else(|| refused("Boundary incidence lost original port"))?;
        if p.end > 1
            || p.edge != t.original.edge
            || p.source != t.original.source
            || p.cell != t.original.cell
            || p.direction != t.original.direction
        {
            return Err(refused(
                "Boundary incidence rebound an original native carrier",
            ));
        }
        let twin = p
            .edge
            .checked_mul(2)
            .and_then(|n| n.checked_add(1 - p.end))
            .ok_or_else(|| refused("Boundary incidence twin overflow"))?;
        let q = ports
            .ports()
            .get(twin)
            .ok_or_else(|| refused("Boundary incidence lost actual opposite port"))?;
        if q.edge != p.edge || q.end != 1 - p.end {
            return Err(refused("Boundary incidence has unrelated opposite port"));
        }
        let mut ends = [0, 0];
        for (end, source_vertex) in [p.vertex, q.vertex].into_iter().enumerate() {
            let vertex = if let Some(&i) = vertex_map.get(&source_vertex) {
                i
            } else {
                entities = entities
                    .checked_add(1)
                    .filter(|n| *n <= o.max_entities)
                    .ok_or_else(|| refused("Original boundary vertex census exceeds allowance"))?;
                let i = vertices.len();
                vertex_map.insert(source_vertex, i);
                vertices.push(OriginalPeriodicSurfaceIncidenceVertex {
                    source_vertex,
                    endpoints: Vec::new(),
                    fans: Vec::new(),
                });
                i
            };
            ends[end] = vertex;
            vertices[vertex]
                .endpoints
                .push(OriginalPeriodicSurfaceEndpointUse { trace, end });
        }
        edges.push(OriginalPeriodicSurfaceIncidenceEdge {
            trace,
            vertices: ends,
        });
    }
    let mut loops = Vec::with_capacity(lifts.walks().len());
    let mut used = vec![false; traces.len()];
    for (walk, w) in lifts.walks().iter().enumerate() {
        if w.parts.is_empty() {
            return Err(refused(
                "Original boundary incidence has an empty selected walk",
            ));
        }
        let indices = (0..w.parts.len())
            .map(|part| {
                lookup
                    .get(&(walk, part))
                    .copied()
                    .ok_or_else(|| refused("Original boundary incidence omits a selected part"))
            })
            .collect::<Result<Vec<_>>>()?;
        let component = traces[indices[0]].component;
        for (i, &incoming) in indices.iter().enumerate() {
            let outgoing = indices[(i + 1) % indices.len()];
            if used[incoming]
                || traces[incoming].component != component
                || traces[outgoing].component != component
            {
                return Err(refused(
                    "Original incidence walk repeats a trace or changes material component",
                ));
            }
            used[incoming] = true;
            if cycles.successor().get(traces[incoming].original.dart)
                != Some(&Some(traces[outgoing].original.dart))
                || edges[incoming].vertices[1] != edges[outgoing].vertices[0]
            {
                return Err(refused(
                    "Boundary incidence lost its actual source-sector successor or physical vertex",
                ));
            }
            vertices[edges[incoming].vertices[1]]
                .fans
                .push(OriginalPeriodicSurfaceBoundaryFan {
                    component,
                    walk,
                    incoming,
                    outgoing,
                });
        }
        loops.push(OriginalPeriodicSurfaceIncidenceLoop {
            component,
            walk,
            traces: indices,
        });
    }
    if used.iter().any(|v| !*v) {
        return Err(refused(
            "Original incidence omitted selected trace coverage",
        ));
    }
    for v in &vertices {
        if v.endpoints.len() != v.fans.len() * 2 {
            return Err(refused(
                "Original incidence has incomplete material endpoint fans",
            ));
        }
        let mut remaining = v
            .endpoints
            .iter()
            .map(|u| (u.trace, u.end))
            .collect::<BTreeSet<_>>();
        if remaining.len() != v.endpoints.len() {
            return Err(refused("Original incidence repeats an endpoint use"));
        }
        for f in &v.fans {
            for u in [
                OriginalPeriodicSurfaceEndpointUse {
                    trace: f.incoming,
                    end: 1,
                },
                OriginalPeriodicSurfaceEndpointUse {
                    trace: f.outgoing,
                    end: 0,
                },
            ] {
                if !remaining.remove(&(u.trace, u.end)) {
                    return Err(refused(
                        "Original incidence duplicates or rebinds a material fan endpoint",
                    ));
                }
            }
        }
        if !remaining.is_empty() {
            return Err(refused("Original incidence lost endpoint fan coverage"));
        }
    }
    Ok(Data {
        vertices,
        edges,
        loops,
        entities,
    })
}
impl OwnedPeriodicSupportIncidence {
    pub fn owner(&self) -> &Arc<OwnedPeriodicSupportTraces> {
        &self.owner
    }
    pub fn vertices(&self) -> &[OriginalPeriodicSurfaceIncidenceVertex] {
        &self.data.vertices
    }
    pub fn edges(&self) -> &[OriginalPeriodicSurfaceIncidenceEdge] {
        &self.data.edges
    }
    pub fn loops(&self) -> &[OriginalPeriodicSurfaceIncidenceLoop] {
        &self.data.loops
    }
    pub fn entities(&self) -> usize {
        self.data.entities
    }
    /// Regularity ONLY of this selected one-dimensional boundary. An empty
    /// boundary says nothing about chart cuts needed to publish a torus face.
    pub fn has_regular_boundary(&self) -> bool {
        self.data
            .vertices
            .iter()
            .all(|v| v.endpoints.len() == 2 && v.fans.len() == 1)
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(self)? != self.data {
            return Err(refused(
                "Original source vertex, boundary fan or trace incidence changed",
            ));
        }
        Ok(())
    }
    /// An actual incident endpoint supplies the vertex's exact world expression.
    /// Other endpoint root fields remain distinct, tied through the original
    /// quotient chart transports and actual supporting-surface seam proofs.
    pub fn vertex_endpoint(
        &self,
        vertex: usize,
        max_work: usize,
    ) -> Result<OriginalPeriodicSurfaceTraceEndpoint> {
        self.reverify_source()?;
        let u = self
            .data
            .vertices
            .get(vertex)
            .and_then(|v| v.endpoints.first())
            .ok_or_else(|| refused("Unknown original incidence vertex"))?;
        self.owner.endpoint(u.trace, u.end, max_work)
    }
}
pub fn own_periodic_support_incidence(
    owner: Arc<OwnedPeriodicSupportTraces>,
    max_entities: usize,
) -> Result<OwnedPeriodicSupportIncidence> {
    let mut o = OwnedPeriodicSupportIncidence {
        owner,
        max_entities,
        data: Data {
            vertices: Vec::new(),
            edges: Vec::new(),
            loops: Vec::new(),
            entities: 0,
        },
    };
    o.data = derive(&o)?;
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::super::periodic_support_regions::tests as fixtures;
    use super::*;
    use crate::profile_certificate::algebraic_intersections::{decode_polynomial, decode_ratio};
    fn traces(selection: Arc<OwnedPeriodicSupportSelection>) -> Arc<OwnedPeriodicSupportTraces> {
        let signatures = fixtures::signatures(selection);
        let regions = Arc::new(own_periodic_support_regions(signatures, &[], 100000).unwrap());
        Arc::new(
            own_periodic_support_traces(regions, ParameterOrderLimits::default(), 1000).unwrap(),
        )
    }
    #[test]
    fn original_surface_incidence_keeps_real_vertex_cycles_and_rejects_cached_rebinding() {
        let source = traces(super::super::periodic_support_material::tests::full_plane(
            true,
        ));
        assert!(own_periodic_support_incidence(source.clone(), 1).is_err());
        let o = own_periodic_support_incidence(source, 1000).unwrap();
        assert_eq!(
            (
                o.vertices().len(),
                o.edges().len(),
                o.loops().len(),
                o.entities()
            ),
            (4, 4, 1, 21)
        );
        assert!(o.has_regular_boundary());
        assert!(
            o.vertices()
                .iter()
                .all(|v| v.endpoints.len() == 2 && v.fans.len() == 1)
        );
        let anchor = o.vertices()[0].endpoints[0];
        assert_eq!(
            o.vertex_endpoint(0, 100000).unwrap(),
            o.owner()
                .endpoint(anchor.trace, anchor.end, 100000)
                .unwrap()
        );
        assert!(o.vertex_endpoint(4, 100000).is_err());
        let mut changed = o.clone();
        changed.data.vertices[0].source_vertex += 100;
        assert!(changed.reverify_source().is_err());
        let mut changed = o.clone();
        changed.data.loops[0].traces.swap(0, 1);
        assert!(changed.reverify_source().is_err());
        let mut changed = o.clone();
        changed.data.vertices[0].fans[0].outgoing = usize::MAX;
        assert!(changed.reverify_source().is_err());
        let mut changed = o.clone();
        changed.data.edges[0].vertices.swap(0, 1);
        assert!(changed.reverify_source().is_err());
    }
    #[test]
    fn original_surface_incidence_retains_two_material_fans_at_an_actual_touching_vertex() {
        let source = traces(fixtures::discs(true));
        let o = own_periodic_support_incidence(source, 1000).unwrap();
        assert_eq!(
            (
                o.vertices().len(),
                o.edges().len(),
                o.loops().len(),
                o.entities()
            ),
            (7, 8, 2, 41)
        );
        assert!(!o.has_regular_boundary());
        let singular = o
            .vertices()
            .iter()
            .enumerate()
            .filter(|(_, v)| v.fans.len() != 1)
            .collect::<Vec<_>>();
        assert_eq!(singular.len(), 1);
        let (i, v) = singular[0];
        assert_eq!(v.endpoints.len(), 4);
        assert_eq!(v.fans.len(), 2);
        assert_ne!(v.fans[0].component, v.fans[1].component);
        let point = o.vertex_endpoint(i, 100000).unwrap();
        let f = decode_polynomial(&point.parameter.primitive).unwrap();
        let root = [
            decode_ratio(&point.parameter.interval[0]).unwrap(),
            decode_ratio(&point.parameter.interval[1]).unwrap(),
        ];
        for axis in 0..3 {
            let n = decode_polynomial(&point.world[axis][0]).unwrap();
            let d = decode_polynomial(&point.world[axis][1]).unwrap();
            let expected = if axis == 2 {
                Q::zero()
            } else {
                Q::new(1.into(), 2.into())
            };
            assert_eq!(
                arithmetic::parameter_order::compare_rational(
                    &f,
                    [&n, &d],
                    &root,
                    &vec![Q::zero(), Q::one()],
                    [&vec![expected], &vec![Q::one()]],
                    &[Q::zero(), Q::zero()],
                    128,
                    128
                )
                .unwrap(),
                std::cmp::Ordering::Equal
            );
        }
    }
    #[test]
    fn original_surface_incidence_does_not_invent_vertices_for_empty_material() {
        let source = traces(super::super::periodic_support_material::tests::full_plane(
            false,
        ));
        let o = own_periodic_support_incidence(source, 1).unwrap();
        assert!(o.owner().owner().components().is_empty());
        assert!(o.vertices().is_empty() && o.edges().is_empty() && o.loops().is_empty());
        assert_eq!(o.entities(), 0);
        assert!(o.has_regular_boundary());
        assert!(o.vertex_endpoint(0, 100000).is_err());
    }
}

#[cfg(test)]
mod periodic_tests {
    use super::super::periodic_support_regions::tests as fixtures;
    use super::*;
    use crate::profile_certificate::algebraic_intersections::{decode_polynomial, decode_ratio};
    fn context(p: &OriginalPeriodicSurfaceTraceEndpoint) -> (Vec<Q>, [Q; 2]) {
        (
            decode_polynomial(&p.parameter.primitive).unwrap(),
            [
                decode_ratio(&p.parameter.interval[0]).unwrap(),
                decode_ratio(&p.parameter.interval[1]).unwrap(),
            ],
        )
    }
    #[test]
    fn original_surface_incidence_periodic_self_loops_keep_exact_world_seam_vertices() {
        let signatures = Arc::new(
            own_periodic_support_signatures(
                fixtures::cylinder_bands(),
                ParameterOrderLimits::default(),
                10000,
            )
            .unwrap(),
        );
        let search = construct_periodic_support_regions(
            signatures,
            ParameterOrderLimits::default(),
            PeriodicRegionSearchLimits::default(),
        )
        .unwrap();
        assert_eq!(search.stop(), None);
        assert_eq!(search.paths().len(), 2);
        let regions = Arc::new(search.regions().unwrap().clone());
        assert_eq!(regions.components().len(), 2);
        let traces = Arc::new(
            own_periodic_support_traces(regions, ParameterOrderLimits::default(), 1000).unwrap(),
        );
        let o = own_periodic_support_incidence(traces, 1000).unwrap();
        assert_eq!(
            (
                o.vertices().len(),
                o.edges().len(),
                o.loops().len(),
                o.entities()
            ),
            (4, 4, 4, 24)
        );
        assert!(o.has_regular_boundary());
        assert!(o.edges().iter().all(|e| e.vertices[0] == e.vertices[1]));
        assert!(o.loops().iter().all(|w| w.traces.len() == 1));
        let levels = [
            Q::new(1.into(), 4.into()),
            Q::new(3.into(), 8.into()),
            Q::new(5.into(), 8.into()),
            Q::new(3.into(), 4.into()),
        ];
        // The first component has oppositely oriented essential rims. Query
        // both actual shores of each, retaining their distinct root fields.
        for edge in o.edges().iter().take(2) {
            let t = &o.owner().traces()[edge.trace];
            assert!(t.original.source < levels.len());
            let a = o.owner().endpoint(edge.trace, 0, 100000).unwrap();
            let b = o.owner().endpoint(edge.trace, 1, 100000).unwrap();
            for p in [&a, &b] {
                let (f, root) = context(p);
                for axis in 0..3 {
                    let n = decode_polynomial(&p.world[axis][0]).unwrap();
                    let d = decode_polynomial(&p.world[axis][1]).unwrap();
                    let expected = if axis == 2 {
                        levels[t.original.source].clone()
                    } else {
                        Q::zero()
                    };
                    assert_eq!(
                        arithmetic::parameter_order::compare_rational(
                            &f,
                            [&n, &d],
                            &root,
                            &vec![Q::zero(), Q::one()],
                            [&vec![expected], &vec![Q::one()]],
                            &[Q::zero(), Q::zero()],
                            128,
                            128
                        )
                        .unwrap(),
                        std::cmp::Ordering::Equal
                    );
                }
            }
            let (f, ar) = context(&a);
            let (g, br) = context(&b);
            let an = decode_polynomial(&a.uv[0][0]).unwrap();
            let ad = decode_polynomial(&a.uv[0][1]).unwrap();
            let bn = decode_polynomial(&b.uv[0][0]).unwrap();
            let bd = decode_polynomial(&b.uv[0][1]).unwrap();
            assert_ne!(
                arithmetic::parameter_order::compare_rational(
                    &f,
                    [&an, &ad],
                    &ar,
                    &g,
                    [&bn, &bd],
                    &br,
                    128,
                    128
                )
                .unwrap(),
                std::cmp::Ordering::Equal
            );
        }
    }
}
