use super::*;

pub(crate) fn unsupported(message: impl Into<String>) -> Error {
    Error::new("BREP_UNSUPPORTED_OPERATION", message)
}
pub(crate) fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(crate) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(crate) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(crate) fn norm(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
pub(crate) fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    norm(sub(a, b))
}
pub(crate) fn point3(p: &[f64]) -> [f64; 3] {
    [p[0], p[1], p[2]]
}

/// Affine inversion of a bilinear affine face `P(u,v) = P00 + u U + v V`.
pub(crate) fn affine_uv_of(surface: &Surface, p: [f64; 3]) -> [f64; 2] {
    let p00 = point3(&surface.control_points[0][0]);
    let u = sub(point3(&surface.control_points[1][0]), p00);
    let v = sub(point3(&surface.control_points[0][1]), p00);
    let rel = sub(p, p00);
    [dot(rel, u) / dot(u, u), dot(rel, v) / dot(v, v)]
}

/// Exact pcurve of a planar arc on an affine face: the affine image of its
/// control points with unchanged weights.
pub(crate) fn affine_arc_pcurve(surface: &Surface, arc: &Curve) -> Curve {
    Curve {
        degree: arc.degree,
        knots: arc.knots.clone(),
        control_points: arc
            .control_points
            .iter()
            .map(|p| affine_uv_of(surface, point3(p)).to_vec())
            .collect(),
        weights: arc.weights.clone(),
        periodic: false,
    }
}

/// Signed-distance contact of the sphere with an oriented plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlaneContact {
    Inside,
    Outside,
    Crossing,
    Tangent,
}

pub(crate) fn plane_contact(
    origin: [f64; 3],
    outward: [f64; 3],
    sphere: &CanonicalSphere,
    band: f64,
) -> (PlaneContact, f64) {
    let d = dot(outward, sub(sphere.center, origin));
    let r = sphere.radius;
    let kind = if d < -(r + band) {
        PlaneContact::Inside
    } else if d > r + band {
        PlaneContact::Outside
    } else if d.abs() < r - band {
        PlaneContact::Crossing
    } else {
        PlaneContact::Tangent
    };
    (kind, d)
}

// ---------------------------------------------------------------------------
// Sphere patch UV helpers (quarter disk: u-axis, quarter arc, v-axis)
// ---------------------------------------------------------------------------

pub(crate) fn piece_of(w: [f64; 2]) -> Result<usize> {
    for corner in [[0., 0.], [1., 0.], [0., 1.]] {
        if (w[0] - corner[0]).hypot(w[1] - corner[1]) <= CORNER {
            return Err(unsupported(
                "Sphere Boolean: section passes through a sphere vertex region",
            ));
        }
    }
    let on_u_axis = w[1].abs() <= BOUNDARY_UV;
    let on_arc = (w[0] * w[0] + w[1] * w[1] - 1.).abs() <= BOUNDARY_UV;
    let on_v_axis = w[0].abs() <= BOUNDARY_UV;
    match (on_u_axis, on_arc, on_v_axis) {
        (true, false, false) => Ok(0),
        (false, true, false) => Ok(1),
        (false, false, true) => Ok(2),
        _ => Err(unsupported(
            "Sphere Boolean: section endpoint is not on exactly one patch boundary piece",
        )),
    }
}
pub(crate) fn snap_to_piece(piece: usize, w: [f64; 2]) -> [f64; 2] {
    match piece {
        0 => [w[0], 0.],
        2 => [0., w[1]],
        _ => {
            let n = w[0].hypot(w[1]);
            [w[0] / n, w[1] / n]
        }
    }
}
/// Piece parameter in loop (CCW) direction: piece 0 is (0,0)->(1,0), piece 1
/// the rational quarter arc (1,0)->(0,1), piece 2 is (0,1)->(0,0).
pub(crate) fn piece_tau(piece: usize, w: [f64; 2], pcurve:&Curve) -> f64 {
    match piece {
        0 => w[0],
        2 => 1. - w[1],
        _ if pcurve.weights == [1.,1.,2.] => w[1]/(1.+w[0]),
        _ => quarter_arc_parameter(w[0], w[1]),
    }
}
/// Parameter of the point at direction (cos, sin) on the canonical
/// quarter arc from angle 0 to 90 degrees (weights 1, cos(pi/4), 1).
pub(crate) fn quarter_arc_parameter(cos: f64, sin: f64) -> f64 {
    let s = sin / (1. + cos);
    let aw = sphere_sphere::ARC_WEIGHT;
    s / (aw + s * (1. - aw))
}
pub(crate) fn strictly_inside_quarter_disk(uv: [f64; 2], margin: f64) -> bool {
    uv[0] > margin && uv[1] > margin && uv[0] * uv[0] + uv[1] * uv[1] < 1. - margin
}

// ---------------------------------------------------------------------------
// Curve network
// ---------------------------------------------------------------------------

/// Section circle on one mate face (a face may carry several).
pub(crate) struct Circle {
    pub(crate) face: usize,
    pub(crate) center: [f64; 3],
    pub(crate) rho: f64,
    pub(crate) e1: [f64; 3],
    pub(crate) e2: [f64; 3],
    /// Edge hits on this circle: (phi, vertex). The mate turns them into
    /// `inside` intervals.
    pub(crate) hits: Vec<(f64, usize)>,
    /// Parts of the circle inside the face, as phi intervals with `hi > lo`;
    /// a full ring is a single `[0, TAU]` interval without hits.
    pub(crate) inside: Vec<[f64; 2]>,
    /// Hit vertex at the start / end of each inside interval (None for a ring).
    pub(crate) ends: Vec<[Option<usize>; 2]>,
    /// Interior events (sphere-seam crossings) per interval.
    pub(crate) events: Vec<Vec<(f64, usize)>>,
    /// The circle is a great circle in a sphere seam plane: it coincides
    /// with four original sphere edges instead of cutting the patches.
    pub(super) seam: bool,
}

impl Circle {
    pub(crate) fn new(face: usize, center: [f64; 3], rho: f64, e1: [f64; 3], e2: [f64; 3]) -> Self {
        Self {
            face,
            center,
            rho,
            e1,
            e2,
            hits: Vec::new(),
            inside: Vec::new(),
            ends: Vec::new(),
            events: Vec::new(),
            seam: false,
        }
    }
    pub(crate) fn phi_of(&self, point: [f64; 3]) -> f64 {
        let d = sub(point, self.center);
        dot(d, self.e2).atan2(dot(d, self.e1)).rem_euclid(TAU)
    }
    pub(crate) fn point_at(&self, phi: f64) -> [f64; 3] {
        let (sin, cos) = phi.sin_cos();
        std::array::from_fn(|k| self.center[k] + self.rho * (cos * self.e1[k] + sin * self.e2[k]))
    }
    pub(crate) fn push_ring(&mut self) {
        self.inside.push([0., TAU]);
        self.ends.push([None, None]);
        self.events.push(Vec::new());
    }
    pub(crate) fn push_interval(&mut self, lo: f64, hi: f64, from: usize, to: usize) {
        self.inside.push([lo, hi]);
        self.ends.push([Some(from), Some(to)]);
        self.events.push(Vec::new());
    }
    /// Sort the hits and refuse coincident ones. Returns the sorted hits.
    pub(crate) fn sorted_hits(&self) -> Result<Vec<(f64, usize)>> {
        let mut hits = self.hits.clone();
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        let n = hits.len();
        for k in 0..n {
            let gap = if k + 1 < n {
                hits[k + 1].0 - hits[k].0
            } else {
                hits[0].0 + TAU - hits[k].0
            };
            if n > 1 && gap < EVENT_GAP {
                return Err(unsupported(
                    "Sphere Boolean: two edge hits coincide on one section circle",
                ));
            }
        }
        Ok(hits)
    }
}

/// One arc piece of the intersection network.
pub(crate) struct Arc {
    pub(crate) face: usize,
    /// Owning sphere patch; `None` when the piece runs along a sphere seam
    /// and is an original sphere edge (or a piece of one).
    pub(crate) patch: Option<usize>,
    /// The 3D edge carrying this piece; its +phi curve and ends live in the
    /// assembler's network entry of the same index.
    pub(super) key: EKey,
    /// The +phi orientation runs against the 3D edge `key`.
    pub(super) reversed: bool,
}

/// The faceted analytic operand paired with the sphere.
pub(crate) trait Mate: Sized {
    /// Strict solid classification with the shared error band; ambiguous
    /// samples are refusals.
    fn inside(&self, point: [f64; 3], band: f64) -> Result<bool>;
    /// Chart coordinates of a boundary point known to lie on `face`.
    fn uv_of(&self, face: usize, point: [f64; 3]) -> Result<[f64; 2]>;
    /// Exact pcurve of a network arc on `face`; the engine snaps the ends to
    /// the chart vertices and verifies the curve against the surface.
    fn arc_pcurve(&self, face: usize, arc: &Curve) -> Result<Curve>;
    /// Author the section network into `imprint.circles`, registering hits
    /// through `Imprint::hit`. `Some` resolves the pair by empty algebra
    /// before any arc is needed.
    fn network(&self, imprint: &mut Imprint<'_, Self>) -> Result<Option<SpatialRelation>>;
}

pub(crate) struct Imprint<'m, M: Mate> {
    pub(crate) asm: Assembler<'m>,
    pub(crate) mate_index: usize,
    pub(crate) mate: &'m M,
    pub(crate) sphere: CanonicalSphere,
    pub(crate) band: f64,
    pub(crate) circles: Vec<Circle>,
    /// Sphere: per face, coedge index of each UV boundary piece.
    pub(super) sphere_pieces: [[usize; 3]; 8],
    /// UV of a special vertex in a sphere patch chart.
    pub(super) patch_uv: BTreeMap<(usize, usize), [f64; 2]>,
    /// Parallel to `asm.net`.
    pub(super) arcs: Vec<Arc>,
}

/// Run the engine for `a op b`, where operand `mate_index` is the mate and
/// the other operand the recognized sphere.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run<M: Mate>(
    a: &Model,
    b: &Model,
    mate_index: usize,
    mate: &M,
    sphere: CanonicalSphere,
    band: f64,
    operation: &str,
) -> Result<Model> {
    let tolerance = a.tolerance_mm.max(b.tolerance_mm);
    let mut imprint = Imprint::new(a, b, mate_index, mate, sphere, band, operation)?;
    if let Some(relation) = mate.network(&mut imprint)? {
        return imprint_pipeline::regularized_empty_algebra(a, b, operation, relation);
    }
    if let Some(relation) = imprint.no_arcs_relation()? {
        return imprint_pipeline::regularized_empty_algebra(a, b, operation, relation);
    }
    imprint.sphere_crossings()?;
    imprint.refine_and_build_arcs()?;
    imprint.emit_mate()?;
    imprint.emit_sphere()?;
    imprint.asm.finish(tolerance)
}
