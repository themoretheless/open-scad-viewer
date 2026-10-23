//! Root-trimmed G2 strip faces; source grafting and corner closure are separate.
use super::{EdgeRadius, TrihedralSetbacks, plan_trihedral_setbacks};
use crate::{
    Model, TopoId, contact_end, contact_trim, face_crossings, face_domain::FaceDomain,
    fillet_contact, fillet_radius,
};
use nurbs_core::{
    Error, Result,
    curve::Curve,
    interval_eval::{self, Interval as I},
    offset::fillet_surfaces::{FilletOptions, RadiusLaw},
    surface_g2_pcurves,
    surface_join::Decision,
    trim_domain::Location,
};
#[derive(Clone, Copy)]
pub struct StripLimits {
    pub tolerances: [f64; 3],
    pub tolerance_uv: f64,
    pub max_geometry_work: usize,
    pub max_cells: usize,
    pub max_exact: u64,
    pub max_coefficients: usize,
}
pub struct StripPatch {
    pub source_edge: usize,
    pub source_identity: TopoId,
    pub support_faces: [usize; 2],
    pub support_identities: [TopoId; 2],
    pub sides: [f64; 2],
    pub radius_mm: f64,
    pub original_law_domain: [f64; 2],
    pub patch: contact_end::Patch,
    pub contacts: [fillet_contact::Contact; 2],
    pub g2: Vec<surface_g2_pcurves::OwnedInspection>,
    pub radius: fillet_radius::Report,
    pub injectivity: nurbs_core::surface_linear_monotonicity::Report,
    pub cutters: [Model; 2],
    pub cutter_faces: [usize; 2],
    pub end_order: [usize; 2],
}
pub struct StripSeparation {
    pub strips: [usize; 2],
    pub distance: nurbs_core::trimmed_surface_distance::TrimmedDistance,
}
pub struct TrihedralStrips {
    /// Positive whole-face lower bounds; not just distances between mesh samples.
    pub separation: Vec<StripSeparation>,
    pub plan: TrihedralSetbacks,
    pub strips: Vec<StripPatch>,
    pub cells: usize,
    pub exact_work: u64,
    pub coefficient_work: usize,
}
fn refused(s: &str) -> Error {
    Error::new("BREP_TRIHEDRAL_SKIN_UNPROVEN", s)
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn normalize(a: [f64; 3]) -> Result<[f64; 3]> {
    let n = a[0].hypot(a[1]).hypot(a[2]);
    if !n.is_finite() || n <= 0. {
        return Err(refused("Degenerate strip frame"));
    }
    Ok(a.map(|v| v / n))
}
struct Work {
    cells: usize,
    exact: u64,
    coefficients: usize,
    limits: StripLimits,
}
impl Work {
    fn cells(&self) -> Result<usize> {
        let n = self.limits.max_cells.saturating_sub(self.cells).min(100000);
        if n == 0 {
            Err(refused("Shared strip cell work exhausted"))
        } else {
            Ok(n)
        }
    }
    fn exact(&self) -> Result<u64> {
        // One optional exact-identity attempt cannot consume the remaining strips'
        // allowance. Whole-curve interval admission keeps the same position limit.
        let n = self.limits.max_exact.saturating_sub(self.exact).min(20000);
        if n == 0 {
            Err(refused("Shared strip exact work exhausted"))
        } else {
            Ok(n)
        }
    }
    fn charge(&mut self, c: usize, e: u64) -> Result<()> {
        if c > self.limits.max_cells - self.cells || e > self.limits.max_exact - self.exact {
            return Err(refused("Strip stage exceeded shared work"));
        }
        self.cells += c;
        self.exact += e;
        Ok(())
    }
}
fn cutter(
    origin: [f64; 3],
    normal: [f64; 3],
    extent: f64,
    tolerance: f64,
) -> Result<(Model, usize)> {
    let axis = (0..3)
        .min_by(|&a, &b| normal[a].abs().total_cmp(&normal[b].abs()))
        .unwrap();
    let mut basis = [0.; 3];
    basis[axis] = 1.;
    let u = normalize(cross(normal, basis))?;
    let v = normalize(cross(normal, u))?;
    let mut m = crate::cuboid([-extent, -extent, 0.], [extent, extent, extent])?;
    let face = m
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 0.)
        })
        .unwrap();
    let map = |p: [f64; 3]| {
        std::array::from_fn(|k| origin[k] + u[k] * p[0] + v[k] * p[1] + normal[k] * p[2])
    };
    for p in &mut m.vertices {
        p.point = map(p.point);
    }
    for e in &mut m.edges {
        for p in &mut e.curve.control_points {
            *p = map([p[0], p[1], p[2]]).to_vec();
        }
    }
    for f in &mut m.faces {
        for p in f.surface.control_points.iter_mut().flatten() {
            *p = map([p[0], p[1], p[2]]).to_vec();
        }
    }
    m.tolerance_mm = tolerance;
    m.rebuild_topology_ids();
    m.validate()?;
    Ok((m, face))
}
fn membership(source: &Model, face: usize, curve: &Curve, work: &mut Work) -> Result<()> {
    let domain = FaceDomain::new(source, face, work.limits.tolerance_uv)?;
    let mut pending = vec![curve.domain()];
    while let Some(range) = pending.pop() {
        work.cells()?;
        work.charge(1, 0)?;
        let b = interval_eval::evaluate_interval(curve, I::new(range[0], range[1])?)?;
        let c = domain.classify([[b[0].lo, b[0].hi], [b[1].lo, b[1].hi]], work.cells()?)?;
        work.charge(c.cells, 0)?;
        match c.location {
            Location::Inside => {}
            Location::Outside => {
                return Err(refused("Contact leaves its original trimmed support"));
            }
            Location::Unresolved => {
                let mid = range[0] + (range[1] - range[0]) * 0.5;
                if !(range[0] < mid && mid < range[1]) {
                    return Err(refused("Support membership remains unresolved"));
                }
                pending.push([range[0], mid]);
                pending.push([mid, range[1]]);
            }
        }
    }
    Ok(())
}
/// Build three open strip faces with independently rechecked saved contacts,
/// whole-seam G2, rolling-center laws and skin injectivity. span_mm explicitly
/// bounds each strip beyond its common corner setback. All work is shared;
/// no partial strip set escapes. This does not graft source faces or close a
/// corner, and the rolling-center law is not a circular-section curvature law.
pub fn build_trihedral_strips(
    source: &Model,
    shell: usize,
    vertex: usize,
    requested: &[EdgeRadius],
    runout_mm: f64,
    span_mm: f64,
    limits: StripLimits,
) -> Result<TrihedralStrips> {
    if !span_mm.is_finite()
        || span_mm <= 0.
        || limits.tolerances.iter().any(|v| !v.is_finite() || *v <= 0.)
        || limits.tolerances[0] > source.tolerance_mm
        || !(1..=300000).contains(&limits.max_cells)
        || !(1..=1000000).contains(&limits.max_exact)
        || limits.max_coefficients > 2000000
        || !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
    {
        return Err(refused(
            "Need explicit span, unchanged positive tolerances and bounded work",
        ));
    }
    let plan = plan_trihedral_setbacks(
        source,
        shell,
        vertex,
        requested,
        runout_mm,
        limits.max_geometry_work,
    )?;
    let origin = source.vertices[vertex].point;
    let mut inward = [0.; 3];
    for e in &plan.edges {
        let other = *source.edges[e.edge]
            .vertices
            .iter()
            .find(|&&v| v != vertex)
            .unwrap();
        for k in 0..3 {
            inward[k] += source.vertices[other].point[k] - origin[k];
        }
    }
    let extent = source
        .vertices
        .iter()
        .map(|p| {
            let q = std::array::from_fn::<_, 3, _>(|k| p.point[k] - origin[k]);
            q[0].hypot(q[1]).hypot(q[2])
        })
        .fold(0_f64, f64::max)
        * 2.;
    if !extent.is_finite() || extent <= 0. {
        return Err(refused("Cutter extent is not finite"));
    }
    let mut work = Work {
        cells: 0,
        exact: 0,
        coefficients: 0,
        limits,
    };
    let mut strips = Vec::new();
    for edge in &plan.edges {
        let selected = &source.edges[edge.edge];
        let other = *selected.vertices.iter().find(|&&v| v != vertex).unwrap();
        let delta = std::array::from_fn(|k| source.vertices[other].point[k] - origin[k]);
        let ray = normalize(delta)?;
        let length = delta[0].hypot(delta[1]).hypot(delta[2]);
        if edge.distance_mm + span_mm >= length - limits.tolerances[0] {
            return Err(refused("Strip reaches another original corner"));
        }
        let supports = plan
            .ports
            .iter()
            .filter(|p| p.edges.contains(&edge.edge))
            .map(|p| p.face)
            .collect::<Vec<_>>();
        if supports.len() != 2 {
            return Err(refused("Lost original support roles"));
        }
        let supports = [supports[0], supports[1]];
        let surfaces = supports.map(|f| &source.faces[f].surface);
        let mut sides = [0.; 2];
        for k in 0..2 {
            let s = surfaces[k];
            let uv = [
                (s.knots_u[s.degree_u] + s.knots_u[s.control_points.len()]) * 0.5,
                (s.knots_v[s.degree_v] + s.knots_v[s.control_points[0].len()]) * 0.5,
            ];
            let eval = s.evaluate(uv[0], uv[1])?;
            let n = eval
                .unit_normal()
                .ok_or_else(|| refused("No regular original tracing normal"))?;
            let side = dot(n, inward);
            if !side.is_finite() || side == 0. {
                return Err(refused("Inward tracing side is unresolved"));
            }
            sides[k] = side.signum();
        }
        let built = fillet_contact::build_g2(
            surfaces,
            edge.radius_mm,
            &FilletOptions {
                side_a: sides[0],
                side_b: sides[1],
                tolerance: limits.tolerances[0] / 64.,
                max_march_steps: 256,
                max_sections: 5,
            },
            limits.tolerances[0] / 4.,
            work.cells()?,
            work.exact()?,
        )?;
        work.charge(built.cells, built.exact_work)?;
        work.coefficients = work
            .coefficients
            .checked_add(built.skin_edit_work)
            .ok_or_else(|| refused("Coefficient work overflow"))?;
        if work.coefficients > limits.max_coefficients {
            return Err(refused("Shared C2 preparation work exhausted"));
        }
        let contacts = built
            .contacts
            .as_ref()
            .ok_or_else(|| refused(&format!("Original contacts unresolved: {:?}", built.status)))?;
        let skin = built
            .fillet
            .surface
            .as_ref()
            .ok_or_else(|| refused("Missing authored skin"))?;
        let mut ends = Vec::new();
        let mut end_faces = Vec::new();
        for distance in [edge.distance_mm, edge.distance_mm + span_mm] {
            let center = std::array::from_fn(|k| origin[k] + ray[k] * distance);
            let (m, f) = cutter(center, ray, extent, (limits.tolerances[0] / 64.).max(1e-10))?;
            ends.push(m);
            end_faces.push(f);
        }
        let mut bands = Vec::new();
        for k in 0..2 {
            let r = face_crossings::inspect_curve(
                &contacts[0].curve,
                &ends[k],
                end_faces[k],
                limits.tolerance_uv,
                work.cells()?,
            )?;
            work.charge(r.cells, 0)?;
            if !r.complete || r.eligible.len() != 1 {
                return Err(refused("End root is not unique and completely isolated"));
            }
            bands.push(r.roots[r.eligible[0]].root.parameter_box[0]);
        }
        let order = if bands[0][1] < bands[1][0] {
            [0, 1]
        } else if bands[1][1] < bands[0][0] {
            [1, 0]
        } else {
            return Err(refused("End root bands overlap"));
        };
        let cutters = order.map(|k| (&ends[k], end_faces[k], 0));
        let mut restricted = Vec::new();
        for k in 0..2 {
            let r = contact_trim::between_faces(
                &contacts[k],
                [surfaces[k], skin],
                cutters,
                limits.tolerances[0] / 4.,
                limits.tolerance_uv,
                work.cells()?,
                work.exact()?,
            )?;
            work.charge(r.cells, r.exact_work)?;
            membership(source, supports[k], &r.contact.support_pcurve, &mut work)?;
            restricted.push(r);
        }
        let patch = contact_end::patch(
            [&restricted[0], &restricted[1]],
            skin,
            order.map(|k| (&ends[k], end_faces[k])),
            limits.tolerances[0],
            limits.tolerance_uv,
            work.cells()?,
            work.exact()?,
        )?;
        work.charge(patch.cells, patch.exact_work)?;
        let saved = &patch.model.faces[0].surface;
        let mut g2 = Vec::new();
        let mut actual = Vec::new();
        for k in 0..2 {
            let e = patch.edges[if k == 0 { 3 } else { 1 }];
            let curve = &patch.model.edges[e].curve;
            if curve != &restricted[k].contact.curve {
                return Err(refused("Saved contact lost its restricted definition"));
            }
            let coedge = patch.model.loops[patch.model.faces[0].outer]
                .coedges
                .iter()
                .find(|c| c.edge == e)
                .ok_or_else(|| refused("Missing saved contact use"))?;
            let proof = surface_g2_pcurves::inspect_owned(
                curve,
                [&restricted[k].contact.support_pcurve, &coedge.pcurve],
                [surfaces[k], saved],
                [false, coedge.reversed],
                limits.tolerances[0],
                limits.tolerances[1],
                limits.tolerances[2],
                work.cells()?,
                work.exact()?,
            )?;
            work.charge(
                proof.report.geometric.cells,
                proof.report.positions.exact_work,
            )?;
            if !proof.report.positions.proven
                || proof.positions.is_none()
                || proof.report.geometric.decision != Decision::WithinTolerance
                || !proof.report.geometric.regularity_proven
            {
                return Err(refused("Actual saved contact G2 remains unproven"));
            }
            let pcurve = if coedge.reversed {
                coedge.pcurve.reverse()?
            } else {
                coedge.pcurve.clone()
            };
            let positions = surface_g2_pcurves::verify_positions(
                curve,
                [&restricted[k].contact.support_pcurve, &pcurve],
                [surfaces[k], saved],
                [false, false],
                limits.tolerances[0],
                work.cells()?,
                work.exact()?,
            )?;
            work.charge(positions.cells, positions.exact_work)?;
            if !positions.proven {
                return Err(refused("Edge-oriented saved use remains unproven"));
            }
            actual.push(fillet_contact::Contact {
                curve: curve.clone(),
                support_pcurve: restricted[k].contact.support_pcurve.clone(),
                fillet_pcurve: pcurve,
                positions,
            });
            g2.push(proof);
        }
        let actual: [fillet_contact::Contact; 2] = actual
            .try_into()
            .map_err(|_| refused("Missing saved contact pair"))?;
        let radius = fillet_radius::inspect_contacts_in_domain(
            surfaces,
            saved,
            [&actual[0], &actual[1]],
            &RadiusLaw::Constant(edge.radius_mm),
            contacts[0].curve.domain(),
            sides,
            limits.tolerances[0],
            work.cells()?,
            work.exact()?,
        )?;
        work.charge(radius.cells, radius.exact_work)?;
        if radius.decision != fillet_radius::Decision::WithinTolerance {
            return Err(refused(&format!(
                "Saved radius unresolved on edge {}: {:?}, {}, gap {:?}, drift {:?}",
                edge.edge,
                radius.decision,
                radius.reason,
                radius.maximum_center_gap_mm,
                radius.parameter_drift_upper_mm
            )));
        }
        let injectivity =
            nurbs_core::surface_linear_monotonicity::inspect_candidate(saved, work.cells()?)?;
        work.charge(injectivity.cells, 0)?;
        if !injectivity.certified {
            return Err(refused("Skin injectivity remains unproven"));
        }
        let ends: [Model; 2] = ends
            .try_into()
            .map_err(|_| refused("Missing end plane definitions"))?;
        strips.push(StripPatch {
            source_edge: edge.edge,
            source_identity: edge.identity,
            support_faces: supports,
            support_identities: supports.map(|f| source.1.faces[f]),
            sides,
            radius_mm: edge.radius_mm,
            original_law_domain: contacts[0].curve.domain(),
            patch,
            contacts: actual,
            g2,
            radius,
            injectivity,
            cutters: ends,
            cutter_faces: [end_faces[0], end_faces[1]],
            end_order: order,
        });
    }
    let mut separation = Vec::new();
    for a in 0..strips.len() {
        for b in a + 1..strips.len() {
            let remaining = work.cells()?;
            if remaining < 2 {
                return Err(refused("Mutual separation work exhausted"));
            }
            let distance = crate::face_domain::distance_between_faces(
                &strips[a].patch.model, 0,
                &strips[b].patch.model, 0,
                limits.tolerances[0], limits.tolerance_uv,
                (remaining / 2).min(1024).max(1),
                // Initial broad trim queries must leave work for the actual
                // subdivision cover; this remains inside the shared caller limit.
                (remaining - remaining / 2).min(65536),
            )?;
            work.charge(distance.cells + distance.domain_cells, 0)?;
            // Convergence to the minimum is unnecessary for disjointness, but
            // an unresolved or zero lower bound cannot establish separation.
            if !distance.lower_bound_mm.is_finite()
                || distance.lower_bound_mm <= limits.tolerances[0]
            {
                return Err(refused(&format!("Mutual strip separation remains unproven for [{a},{b}]: lower={}, cells={}, domains={}",distance.lower_bound_mm,distance.cells,distance.domain_cells)));
            }
            separation.push(StripSeparation { strips: [a,b], distance });
        }
    }
    Ok(TrihedralStrips {
        separation,
        plan,
        strips,
        cells: work.cells,
        exact_work: work.exact,
        coefficient_work: work.coefficients,
    })
}
