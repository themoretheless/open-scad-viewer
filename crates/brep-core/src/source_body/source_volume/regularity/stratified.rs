//! Whole-face rank away from declared poles, with owned physical C1/C2 pole disks.
//! Ordinary seams and vertices retain their declared classes, including C0.
use crate::source_seam_tangency::{
    Boundary, C1PoleAtlas, C1PoleNeighborhood, C2PoleAtlas, C2PoleNeighborhood, Carrier,
};
mod sealed {
    pub trait Sealed {}
    impl<E: super::Carrier> Sealed for super::C1PoleAtlas<'_, '_, E> {}
    impl<E: super::Carrier> Sealed for super::C2PoleAtlas<'_, '_, E> {}
}
/// Only native complete physical pole atlases can supply regularity evidence.
/// Borrowed neighborhoods remain tied to their original immutable boundary.
pub trait PoleEvidence<'b, 's, E: Carrier>: sealed::Sealed {
    fn boundary(&self) -> &'b Boundary<'s, E>;
    fn c1_neighborhoods(&self) -> &[C1PoleNeighborhood] {
        &[]
    }
    fn c2_neighborhoods(&self) -> &[C2PoleNeighborhood] {
        &[]
    }
}
impl<'b, 's, E: Carrier> PoleEvidence<'b, 's, E> for C1PoleAtlas<'b, 's, E> {
    fn boundary(&self) -> &'b Boundary<'s, E> {
        self.boundary()
    }
    fn c1_neighborhoods(&self) -> &[C1PoleNeighborhood] {
        self.neighborhoods()
    }
}
impl<'b, 's, E: Carrier> PoleEvidence<'b, 's, E> for C2PoleAtlas<'b, 's, E> {
    fn boundary(&self) -> &'b Boundary<'s, E> {
        self.boundary()
    }
    fn c2_neighborhoods(&self) -> &[C2PoleNeighborhood] {
        self.neighborhoods()
    }
}
use nurbs_core::{Error, Result, surface_quotient_injectivity, surface_regularity};
use std::collections::BTreeMap;

pub enum Face {
    Ordinary(surface_regularity::Report),
    Polar {
        vertex: usize,
        axis: usize,
        end: usize,
        proof: surface_quotient_injectivity::PolarReport,
    },
}
/// Private construction retains all source geometry, material, complete face
/// coverage and physical pole-neighborhood evidence through one immutable owner.
pub struct StratifiedBody<
    'p,
    'b,
    's,
    E: Carrier,
    P: PoleEvidence<'b, 's, E> = C2PoleAtlas<'b, 's, E>,
> {
    poles: &'p P,
    boundary: &'b Boundary<'s, E>,
    faces: Vec<Face>,
}
impl<'p, 'b, 's, E: Carrier, P: PoleEvidence<'b, 's, E>> StratifiedBody<'p, 'b, 's, E, P> {
    pub fn poles(&self) -> &'p P {
        self.poles
    }
    pub fn faces(&self) -> &[Face] {
        &self.faces
    }
    pub fn body(&self) -> &crate::source_volume::Body<E> {
        self.boundary.body()
    }
}
pub struct Report<'p, 'b, 's, E: Carrier, P: PoleEvidence<'b, 's, E> = C2PoleAtlas<'b, 's, E>> {
    pub certificate: Option<StratifiedBody<'p, 'b, 's, E, P>>,
    pub cells: usize,
    pub steps: usize,
    pub completed_faces: usize,
    pub uncertain_face: Option<usize>,
    pub reason: &'static str,
}
fn step<'b, 's, E: Carrier, P: PoleEvidence<'b, 's, E>>(
    out: &mut Report<'_, 'b, 's, E, P>,
    max_steps: usize,
) -> bool {
    if out.steps == max_steps {
        out.reason = "source-stratified-incidence-work-limit";
        false
    } else {
        out.steps += 1;
        true
    }
}

/// A positive determinant of the complete polar (F,G) blow-up implies rank two
/// of the original surface for u>0: F and G are smooth functions of its world
/// point there, with a freshly bounded positive denominator. At u=0, the SAME
/// body's owned C1/C2 graph disk supplies a regular physical surface neighborhood.
/// This does not turn the singular original parameters into a regular chart.
/// Full original faces share one cell budget; all incidence shares one step
/// budget. No partial face cover or caller-provided pole report authorizes this.
pub fn qualify<'p, 'b, 's, E: Carrier, P: PoleEvidence<'b, 's, E>>(
    poles: &'p P,
    max_cells: usize,
    max_steps: usize,
) -> Result<Report<'p, 'b, 's, E, P>> {
    if max_cells > 100000 || max_steps > 1000000 {
        return Err(Error::new(
            "BREP_SOURCE_STRATIFIED_REGULARITY",
            "Choose bounded face and incidence work",
        ));
    }
    let body = poles.boundary().body();
    let shell = body.geometry().shell();
    for wires in shell.faces() {
        wires[0].edges()[0].surface().validate()?;
    }
    let mut out = Report {
        certificate: None,
        cells: 0,
        steps: 0,
        completed_faces: 0,
        uncertain_face: None,
        reason: "source-stratified-regularity-unproven",
    };
    let mut neighborhoods = BTreeMap::new();
    for (vertex, point, faces) in poles
        .c1_neighborhoods()
        .iter()
        .map(|n| (n.vertex(), n.point(), n.faces()))
        .chain(
            poles
                .c2_neighborhoods()
                .iter()
                .map(|n| (n.vertex(), n.point(), n.faces())),
        )
    {
        if !step(&mut out, max_steps) {
            return Ok(out);
        }
        for &face in faces {
            if !step(&mut out, max_steps) {
                return Ok(out);
            }
            if neighborhoods.insert((vertex, face), point).is_some() {
                return Ok(out);
            }
        }
    }
    let mut declarations = BTreeMap::new();
    for (a, p) in shell.poles() {
        if !step(&mut out, max_steps) {
            return Ok(out);
        }
        let vertex = shell.vertices()[a.face][a.wire][a.edge][0];
        let Some(n) = neighborhoods.get(&(vertex, a.face)) else {
            return Ok(out);
        };
        if *n != p.point() || declarations.insert(a.face, (a, p, vertex)).is_some() {
            // Multiple collapsed sides need their own complete chart cover.
            return Ok(out);
        }
    }
    let mut faces = Vec::new();
    for (index, wires) in shell.faces().iter().enumerate() {
        out.uncertain_face = Some(index);
        if !step(&mut out, max_steps) {
            return Ok(out);
        }
        let s = wires[0].edges()[0].surface();
        let remaining = max_cells - out.cells;
        if remaining == 0 {
            out.reason = "source-stratified-cell-work-limit";
            return Ok(out);
        }
        if let Some((_, pole, vertex)) = declarations.get(&index) {
            let source = pole.source();
            let curve = source.curve();
            let domain = curve.domain();
            let bounds = source.parameter_bounds();
            let full = bounds.iter().any(|b| *b == [domain[0]; 2])
                && bounds.iter().any(|b| *b == [domain[1]; 2]);
            if !full || curve.degree != 1 || curve.control_points.len() != 2 {
                return Ok(out);
            }
            let ranges = [
                [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
            ];
            let a = &curve.control_points[0];
            let b = &curve.control_points[1];
            let side = (0..2).find_map(|axis| {
                let other = 1 - axis;
                if a[axis] != b[axis]
                    || !((a[other] == ranges[other][0] && b[other] == ranges[other][1])
                        || (a[other] == ranges[other][1] && b[other] == ranges[other][0]))
                {
                    return None;
                }
                ranges[axis]
                    .iter()
                    .position(|x| *x == a[axis])
                    .map(|end| (axis, end))
            });
            let Some((axis, end)) = side else {
                return Ok(out);
            };
            if remaining < 64 {
                out.reason = "source-stratified-cell-work-limit";
                return Ok(out);
            }
            // Pure parameter exchange preserves every original binary leaf.
            let canonical = crate::face_injectivity::canonical_pole_surface(s, axis);
            let proof = match surface_quotient_injectivity::certify_polar_source_frame(
                &canonical, end, 8, remaining,
            ) {
                Ok(proof) => proof,
                Err(_) => {
                    out.cells = max_cells;
                    return Ok(out);
                }
            };
            out.cells += proof.cells;
            if !proof.proven {
                return Ok(out);
            }
            faces.push(Face::Polar {
                vertex: *vertex,
                axis,
                end,
                proof,
            });
        } else {
            let proof = match surface_regularity::inspect(s, remaining) {
                Ok(proof) => proof,
                Err(_) => {
                    out.cells = max_cells;
                    return Ok(out);
                }
            };
            out.cells += proof.cells;
            if !proof.spanwise_regular {
                return Ok(out);
            }
            faces.push(Face::Ordinary(proof));
        }
        out.completed_faces += 1;
    }
    if !faces.is_empty() {
        out.certificate = Some(StratifiedBody {
            poles,
            boundary: poles.boundary(),
            faces,
        });
        out.uncertain_face = None;
        out.reason = "source-whole-face-ranks-and-owned-physical-poles-qualified";
    }
    Ok(out)
}
