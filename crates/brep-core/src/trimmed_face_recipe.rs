//! Assemble an immutable face fragment from audited UV loops and canonical
//! world edges. No welding, curve reversal, fitting or solid admission occurs.
use crate::{Coedge, Edge, Face, Loop, Vertex};
use cad_predicates::{ToleranceContext, ToleranceSpecIdentity};
use nurbs_core::{
    Error, Result, curve::Curve, curve_surface_agreement as agreement, surface::Surface,
    trim_region_audit,
};

pub struct Boundary {
    pub curve: Curve,
    /// Forward UV traversal of the face loop, regardless of edge reversal.
    pub pcurve: Curve,
    pub reversed: bool,
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub pairs: usize,
    pub region_cells: usize,
    pub domain_cells: usize,
    pub agreement_cells: usize,
}
pub struct QualifiedFace {
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    loops: Vec<Loop>,
    face: Face,
    context: ToleranceSpecIdentity,
}
impl QualifiedFace {
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
    pub fn loops(&self) -> &[Loop] {
        &self.loops
    }
    pub fn face(&self) -> &Face {
        &self.face
    }
    pub fn context(&self) -> &ToleranceSpecIdentity {
        &self.context
    }
}
pub struct Report {
    pub region: trim_region_audit::Report,
    pub agreements: Vec<agreement::Report>,
    pub agreement_cells: usize,
    pub face: Option<QualifiedFace>,
    pub reason: &'static str,
}
fn endpoints(c: &Curve) -> Option<[[f64; 3]; 2]> {
    let n = c.control_points.len();
    let d = c.domain();
    if c.periodic
        || !c.knots[..=c.degree].iter().all(|k| *k == d[0])
        || !c.knots[n..].iter().all(|k| *k == d[1])
    {
        return None;
    }
    let p = |i| {
        let q: &Vec<f64> = &c.control_points[i];
        [q[0], q[1], q[2]]
    };
    Some([p(0), p(n - 1)])
}
fn oriented(edge: &Boundary) -> Option<[[f64; 3]; 2]> {
    endpoints(&edge.curve).map(|p| if edge.reversed { [p[1], p[0]] } else { p })
}
pub fn assemble(
    context: &ToleranceContext,
    surface: &Surface,
    wires: &[Vec<Boundary>],
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report> {
    surface.validate()?;
    if !(1..=100000).contains(&limits.agreement_cells) {
        return Err(Error::new(
            "BREP_FACE_RECIPE_BUDGET",
            "Face agreement needs 1..100000 shared cells",
        ));
    }
    // Every input is checked before budget or region refusal can hide it.
    for edge in wires.iter().flatten() {
        edge.curve.validate()?;
        edge.pcurve.validate()?;
        if edge.curve.control_points[0].len() != 3 || edge.pcurve.control_points[0].len() != 2 {
            return Err(Error::new(
                "BREP_FACE_RECIPE_DIMENSION",
                "Face recipes need 3D world edges and 2D pcurves",
            ));
        }
    }
    let uv = wires
        .iter()
        .map(|wire| wire.iter().map(|e| e.pcurve.clone()).collect())
        .collect::<Vec<Vec<_>>>();
    let region = trim_region_audit::inspect(
        &uv,
        tolerance_uv,
        limits.pairs,
        limits.region_cells,
        limits.domain_cells,
    )?;
    let mut out = Report {
        region,
        agreements: vec![],
        agreement_cells: 0,
        face: None,
        reason: "face-region-unqualified",
    };
    if out.region.valid != Some(true) {
        return Ok(out);
    }
    if out.region.winding[0] != Some(1) || out.region.winding[1..].iter().any(|w| *w != Some(-1)) {
        out.reason = "face-region-orientation-invalid";
        return Ok(out);
    }
    for wire in wires {
        for i in 0..wire.len() {
            let (Some(a), Some(b)) = (oriented(&wire[i]), oriented(&wire[(i + 1) % wire.len()]))
            else {
                out.reason = "world-endpoint-identity-unproven";
                return Ok(out);
            };
            if a[1].map(f64::to_bits) != b[0].map(f64::to_bits) {
                out.reason = "world-joins-not-identical";
                return Ok(out);
            }
        }
    }
    for edge in wires.iter().flatten() {
        if out.agreement_cells == limits.agreement_cells {
            out.reason = "face-agreement-work-limit";
            return Ok(out);
        }
        let r = agreement::verify(
            &edge.curve,
            &edge.pcurve,
            surface,
            edge.reversed,
            context.spatial_bounds().on_mm,
            limits.agreement_cells - out.agreement_cells,
        )?;
        out.agreement_cells += r.cells;
        let status = r.status;
        out.agreements.push(r);
        if status != agreement::Status::WithinTolerance {
            out.reason = "face-source-agreement-unqualified";
            return Ok(out);
        }
    }
    let mut vertices = vec![];
    let mut edges = vec![];
    let mut loops = vec![];
    for wire in wires {
        let start = vertices.len();
        for edge in wire {
            vertices.push(Vertex {
                point: oriented(edge).unwrap()[0],
            });
        }
        let mut coedges = vec![];
        for (i, edge) in wire.iter().enumerate() {
            let ends = [start + i, start + (i + 1) % wire.len()];
            coedges.push(Coedge {
                edge: edges.len(),
                reversed: edge.reversed,
                pcurve: edge.pcurve.clone(),
            });
            edges.push(Edge {
                degenerate: false,
                vertices: if edge.reversed {
                    [ends[1], ends[0]]
                } else {
                    ends
                },
                curve: edge.curve.clone(),
            });
        }
        loops.push(Loop { coedges });
    }
    let face = Face {
        surface: surface.clone(),
        outer: 0,
        holes: (1..loops.len()).collect(),
    };
    out.face = Some(QualifiedFace {
        vertices,
        edges,
        loops,
        face,
        context: context.spec_identity(),
    });
    out.reason = "face-fragment-qualified";
    Ok(out)
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
    fn fixture() -> (Surface, Vec<Vec<Boundary>>) {
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 1.], vec![0., 1., 1.]],
                vec![vec![1., 0., 1.], vec![1., 1., 1.]],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let arc = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: vec![1., 0.5f64.sqrt(), 1.],
            periodic: false,
        };
        let uv = vec![
            arc,
            Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]]).unwrap(),
            Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap(),
        ];
        let wire = uv
            .into_iter()
            .map(|pcurve| {
                let mut curve = pcurve.clone();
                for p in &mut curve.control_points {
                    p.push(1.);
                }
                Boundary {
                    curve,
                    pcurve,
                    reversed: false,
                }
            })
            .collect();
        (surface, vec![wire])
    }
    #[test]
    fn canonical_rational_face_preserves_definitions_and_closed_world_incidence() {
        let (surface, mut wires) = fixture();
        // A reversed canonical edge retains a forward loop pcurve.
        wires[0][1].curve.control_points.reverse();
        wires[0][1].curve.weights.reverse();
        wires[0][1].reversed = true;
        let report = assemble(
            &ToleranceContext::default_valid(),
            &surface,
            &wires,
            1e-8,
            limits(),
        )
        .unwrap();
        let face = report.face.as_ref().expect(report.reason);
        assert_eq!(face.vertices().len(), 3);
        assert_eq!(face.edges().len(), 3);
        assert_eq!(face.face().surface, surface);
        for (i, e) in face.edges().iter().enumerate() {
            assert_eq!(e.curve, wires[0][i].curve);
        }
        assert!(face.loops()[0].coedges[1].reversed);
        assert_eq!(face.edges()[1].vertices, [2, 1]);
        assert!(
            report
                .agreements
                .iter()
                .all(|r| r.status == agreement::Status::WithinTolerance)
        );
    }
    #[test]
    fn world_gap_and_wrong_source_lift_never_produce_a_face() {
        let (surface, mut wires) = fixture();
        wires[0][0].curve.control_points[0][0] += 1e-9;
        let r = assemble(
            &ToleranceContext::default_valid(),
            &surface,
            &wires,
            1e-8,
            limits(),
        )
        .unwrap();
        assert!(r.face.is_none());
        assert_eq!(r.reason, "world-joins-not-identical");
        let (surface, mut wires) = fixture();
        wires[0][0].curve.control_points[1][2] += 0.1;
        let r = assemble(
            &ToleranceContext::default_valid(),
            &surface,
            &wires,
            1e-8,
            limits(),
        )
        .unwrap();
        assert!(r.face.is_none());
        assert_eq!(r.reason, "face-source-agreement-unqualified");
    }
    #[test]
    fn exhausted_region_budget_cannot_hide_malformed_world_geometry() {
        let (surface, mut wires) = fixture();
        wires[0][2].curve.weights[1] = -1.;
        let work = Limits {
            pairs: 1,
            region_cells: 1,
            domain_cells: 1,
            agreement_cells: 1,
        };
        assert!(
            assemble(
                &ToleranceContext::default_valid(),
                &surface,
                &wires,
                1e-8,
                work
            )
            .is_err()
        );
    }
}
