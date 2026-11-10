//! Homothetic bevel walls with explicit fan caps and collapsed pole boundaries.
use super::*;
use nurbs_core::{interval_eval::Interval as I, involute::QualifiedBevelOutline};
fn refuse(s: &str) -> Error {
    Error::new("BREP_BEVEL_BODY_UNPROVEN", s)
}
fn line3(a: [f64; 3], b: [f64; 3]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    }
}
fn point(p: &[f64]) -> [f64; 3] {
    [p[0], p[1], p[2]]
}
fn midpoint(x: [f64; 2]) -> f64 {
    x[0] + (x[1] - x[0]) * 0.5
}
fn ruled(a: &Curve, b: &Curve) -> Surface {
    Surface {
        degree_u: a.degree,
        degree_v: 1,
        knots_u: a.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        periodic_u: false,
        periodic_v: false,
        control_points: a
            .control_points
            .iter()
            .zip(&b.control_points)
            .map(|(a, b)| vec![a.clone(), b.clone()])
            .collect(),
        weights: a.weights.iter().map(|w| vec![*w, *w]).collect(),
    }
}
fn pole_curve(c: &Curve, p: [f64; 3]) -> Curve {
    let mut c = c.clone();
    for q in &mut c.control_points {
        *q = p.to_vec();
    }
    c
}
fn add_edge(edges: &mut Vec<Edge>, vertices: [usize; 2], curve: Curve, degenerate: bool) -> usize {
    let id = edges.len();
    edges.push(Edge {
        vertices,
        curve,
        degenerate,
    });
    id
}
fn add_face(
    faces: &mut Vec<Face>,
    loops: &mut Vec<Loop>,
    uses: &mut Vec<FaceUse>,
    surface: Surface,
    edges: [usize; 4],
    reversed: bool,
) {
    let a = surface.knots_u[surface.degree_u];
    let b = surface.knots_u[surface.control_points.len()];
    let uv = [
        ([a, 0.], [b, 0.]),
        ([b, 0.], [b, 1.]),
        ([b, 1.], [a, 1.]),
        ([a, 1.], [a, 0.]),
    ];
    let coedges = (0..4)
        .map(|i| Coedge::new(
            edges[i],
            i >= 2,
            unit_line(uv[i].0, uv[i].1),
        ))
        .collect();
    let outer = loops.len();
    loops.push(Loop { coedges });
    let face = faces.len();
    faces.push(Face {
        surface,
        outer,
        holes: vec![],
    });
    uses.push(FaceUse { face, reversed });
}
pub struct BevelBodyCandidate {
    pub model: Model,
    /// Original nominal outer profile plus saved radial-scaling error.
    pub world_error_upper: f64,
    pub wall_error_upper: f64,
    pub cap_error_upper: f64,
    pub closure: crate::watertight::QualifiedReport,
    /// Closure and source error do not imply geometric solid admission.
    pub solid_geometry_certified: bool,
}
/// Build a closed topological body from an original qualified spherical profile.
/// Caps are explicit ruled fans to the outer/inner cone-distance axis points;
/// they are not planar faces or spherical caps. Their ideal reference uses the
/// same original profile and exact nominal axis points. This candidate still
/// requires original profile embedding and full solid qualification.
pub fn bevel_body_candidate(
    source: &QualifiedBevelOutline,
    tolerance: f64,
    max_cells: usize,
    max_exact_work: u64,
) -> Result<BevelBodyCandidate> {
    source.reverify()?;
    if !tolerance.is_finite() || tolerance <= 0. {
        return Err(refuse("Bevel body needs finite positive tolerance"));
    }
    let curves = source.curves();
    let n = curves.len();
    if n * 3 > crate::MAX_FACES {
        return Err(refuse("Bevel fan-cap body exceeds the face budget"));
    }
    let ranges = source.source().source().parameters().cone_distances;
    let distances = ranges.map(midpoint);
    let ratio = distances[0] / distances[1];
    if !(0. < ratio && ratio < 1.) {
        return Err(refuse("Bevel radial scale is unrepresentable"));
    }
    let nominal_ratio =
        I::new(ranges[0][0], ranges[0][1])?.div(I::new(ranges[1][0], ranges[1][1])?)?;
    let mut inner = curves.to_vec();
    let mut radial_error = 0_f64;
    for c in &mut inner {
        for p in &mut c.control_points {
            for x in p {
                let original = *x;
                *x *= ratio;
                let ideal = I::point(original).mul(nominal_ratio)?;
                let d = I::point(*x).sub(ideal)?;
                // Coordinate bounds combined below by the conservative L1 norm.
                radial_error = radial_error.max(d.lo.abs().max(d.hi.abs()));
            }
        }
    }
    let radial_error = I::point(radial_error).mul(I::point(3.))?.hi;
    let wall_error = I::point(source.certificate().world_error_upper)
        .add(I::point(radial_error))?
        .hi;
    let mut center_error = 0_f64;
    for i in 0..2 {
        let d = I::point(distances[i]).sub(I::new(ranges[i][0], ranges[i][1])?)?;
        center_error = center_error.max(d.lo.abs().max(d.hi.abs()));
    }
    let cap_error = wall_error.max(center_error);
    let error = wall_error.max(cap_error);
    if error > tolerance {
        return Err(refuse(
            "Bevel radial/cap nominal error exceeds requested tolerance",
        ));
    }
    let axes = [[0., 0., distances[0]], [0., 0., distances[1]]];
    let mut vertices = Vec::with_capacity(2 * n + 2);
    for profile in [inner.as_slice(), curves] {
        for c in profile {
            vertices.push(Vertex {
                point: point(&c.control_points[0]),
            });
        }
    }
    vertices.extend(axes.map(|point| Vertex { point }));
    let mut edges = Vec::new();
    let mut level = [Vec::new(), Vec::new()];
    for (l, profile) in [inner.as_slice(), curves].into_iter().enumerate() {
        for (j, c) in profile.iter().enumerate() {
            level[l].push(add_edge(
                &mut edges,
                [l * n + j, l * n + (j + 1) % n],
                c.clone(),
                false,
            ));
        }
    }
    let mut radial = Vec::new();
    let mut spokes = [Vec::new(), Vec::new()];
    for j in 0..n {
        radial.push(add_edge(
            &mut edges,
            [j, n + j],
            line3(vertices[j].point, vertices[n + j].point),
            false,
        ));
    }
    for l in 0..2 {
        for j in 0..n {
            spokes[l].push(add_edge(
                &mut edges,
                [2 * n + l, l * n + j],
                line3(axes[l], vertices[l * n + j].point),
                false,
            ));
        }
    }
    let pole = [0, 1].map(|l| {
        add_edge(
            &mut edges,
            [2 * n + l, 2 * n + l],
            line3(axes[l], axes[l]),
            true,
        )
    });
    let mut faces = Vec::new();
    let mut loops = Vec::new();
    let mut uses = Vec::new();
    for j in 0..n {
        let next = (j + 1) % n;
        add_face(
            &mut faces,
            &mut loops,
            &mut uses,
            ruled(&inner[j], &curves[j]),
            [level[0][j], radial[next], level[1][j], radial[j]],
            false,
        );
        for l in 0..2 {
            let c = if l == 0 { &inner[j] } else { &curves[j] };
            add_face(
                &mut faces,
                &mut loops,
                &mut uses,
                ruled(&pole_curve(c, axes[l]), c),
                [pole[l], spokes[l][next], level[l][j], spokes[l][j]],
                l == 1,
            );
        }
    }
    let mut model = Model(
        brep_topology::Model {
            vertices,
            edges,
            loops,
            faces,
            shells: vec![Shell {
                faces: uses,
                closed: true,
            }],
            bodies: vec![Body {
                outer_shell: 0,
                inner_shells: vec![],
            }],
            tolerance_mm: tolerance,
        },
        TopologyIds::default(),
    );
    model.rebuild_topology_ids();
    model.validate()?;
    let closure = crate::watertight::inspect_with_exact(&model, max_cells, max_exact_work)?;
    if closure.closure.status != crate::watertight::Closure::WithinTolerance {
        return Err(Error::new(
            "BREP_BEVEL_BODY_UNPROVEN",
            format!(
                "Bevel closure {:?}; cells {}; exact work {}; first unresolved uses {:?}",
                closure.closure.status,
                closure.closure.agreement.cells,
                closure.exact.work,
                closure
                    .closure
                    .agreement
                    .uses
                    .iter()
                    .filter(|u| u.status
                        != nurbs_core::curve_surface_agreement::Status::WithinTolerance)
                    .take(3)
                    .collect::<Vec<_>>()
            ),
        ));
    }
    Ok(BevelBodyCandidate {
        model,
        world_error_upper: error,
        wall_error_upper: wall_error,
        cap_error_upper: cap_error,
        closure,
        solid_geometry_certified: false,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct BevelBuildLimits {
    pub profile: ProfileEmbeddingLimits,
    pub max_patches_per_flank: usize,
    pub max_surfaces: usize,
    pub max_curves: usize,
    pub closure_cells: usize,
    pub exact_work: u64,
}
impl Default for BevelBuildLimits {
    fn default() -> Self {
        Self {
            profile: ProfileEmbeddingLimits::default(),
            max_patches_per_flank: 16,
            max_surfaces: 8192,
            max_curves: 8192,
            closure_cells: 1000000,
            exact_work: 1000000,
        }
    }
}
/// Native straight-bevel body candidate with an original embedded 3D contour,
/// nominal whole-wall/fan-cap error and continuous shared-edge closure.
/// Global cap/wall embedding and whole-solid admission remain separate.
pub struct ProfileVerifiedBevelGear {
    profile: VerifiedBevelProfile,
    candidate: BevelBodyCandidate,
}
impl ProfileVerifiedBevelGear {
    pub fn model(&self) -> &Model {
        &self.candidate.model
    }
    pub fn profile(&self) -> &VerifiedBevelProfile {
        &self.profile
    }
    pub fn world_error_upper(&self) -> f64 {
        self.candidate.world_error_upper
    }
    pub fn closure(&self) -> &crate::watertight::QualifiedReport {
        &self.candidate.closure
    }
    pub fn solid_geometry_certified(&self) -> bool {
        false
    }
    pub fn into_model(self) -> Model {
        self.candidate.model
    }
}
pub fn bevel_with_verified_profile(
    spec: nurbs_core::involute::BevelGearSpec,
    tolerance: f64,
    limits: BevelBuildLimits,
) -> Result<ProfileVerifiedBevelGear> {
    // Cheap source/body feasibility precedes expensive exact embedding search.
    let source = nurbs_core::involute::qualified_bevel_outline(
        spec,
        tolerance,
        limits.max_patches_per_flank,
        limits.max_surfaces,
        limits.max_curves,
    )?;
    let candidate =
        bevel_body_candidate(&source, tolerance, limits.closure_cells, limits.exact_work)?;
    let profile = certify_bevel_profile(
        spec,
        tolerance,
        limits.max_patches_per_flank,
        limits.max_surfaces,
        limits.max_curves,
        limits.profile,
    )?;
    let original = profile.source().curves();
    let n = original.len();
    if candidate.model.edges.len() < 2 * n
        || candidate.model.edges[n..2 * n]
            .iter()
            .zip(original)
            .any(|(e, c)| e.curve != *c)
    {
        return Err(refuse(
            "Bevel body outer edges differ from the embedded original profile",
        ));
    }
    let validation = candidate.model.validate()?;
    if validation.boundary_edge_count != 0
        || candidate.model.bodies.len() != 1
        || candidate.model.shells.len() != 1
        || !candidate.model.shells[0].closed
        || candidate.world_error_upper > tolerance
    {
        return Err(refuse(
            "Bevel native candidate lacks closed ownership or nominal error admission",
        ));
    }
    Ok(ProfileVerifiedBevelGear { profile, candidate })
}
