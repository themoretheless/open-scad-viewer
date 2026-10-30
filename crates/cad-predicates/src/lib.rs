//! Candidate finite predicates over explicit authored source coordinates.
//!
//! This crate has no B-rep dependency and issues no solid/operation certificate.
//! Authored admission is a trusted boundary: callers must not relabel rounded
//! construction results as authored input. Opaque references prevent accidental
//! cross-source reuse; they do not authenticate arbitrary caller assertions.
mod arithmetic;
mod fibered_affine;
pub use fibered_affine::*;
mod bezier_quadric_identity;
pub use bezier_quadric_identity::*;
mod bezier_composition_identity;
pub use bezier_composition_identity::*;
mod line_chart_identity;
pub use line_chart_identity::*;
mod line_crossing_parameter;
pub use line_crossing_parameter::*;
mod bspline_restriction_identity;
pub use bspline_restriction_identity::*;
mod bspline_projective_correspondence;
pub use bspline_projective_correspondence::*;
mod bezier_projective_correspondence;
pub use bezier_projective_correspondence::*;
mod bezier_identity;
pub use bezier_identity::*;
mod projective_strip_jets;
pub use projective_strip_jets::*;
mod curve_geometric_jets;
pub use curve_geometric_jets::*;
mod bezier_strip_jets;
pub use bezier_strip_jets::*;
mod bspline_internal_jets;
pub use bspline_internal_jets::*;
mod bspline_knot_jets;
pub use bspline_knot_jets::*;
mod bspline_endpoint_jets;
pub use bspline_endpoint_jets::*;
mod construction;
mod construction3;
mod context;
mod recipe_codec3;
pub use recipe_codec3::*;

use arithmetic::{Approx, Expansion, Interval};
pub use construction::*;
pub use construction3::*;
pub use context::*;

const BINOMIAL_TABLE: [[u64; 33]; 33] = {
    let mut table = [[0u64; 33]; 33];
    let mut n = 0;
    while n < 33 {
        table[n][0] = 1;
        let mut k = 1;
        while k <= n {
            table[n][k] = table[n - 1][k - 1] + table[n - 1][k];
            k += 1;
        }
        n += 1;
    }
    table
};

#[inline(always)]
pub(crate) fn binomial_u64(n: usize, k: usize) -> u64 {
    if k > n {
        0
    } else if n < 33 {
        BINOMIAL_TABLE[n][k]
    } else {
        (0..k).fold(1, |v, i| v * (n - i) as u64 / (i + 1) as u64)
    }
}

pub const IMPLEMENTATION_VERSION: &str = "cad-predicates-candidate-15";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sign {
    Negative,
    Zero,
    Positive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    PrecisionExhausted,
    ResourceLimit,
    Cancelled,
    DeadlineExceeded,
    MissingProof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Admission,
    Binary64,
    Interval,
    ExactExpansion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Sign(Sign),
    Indeterminate(Reason),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    pub outcome: Outcome,
    pub stage: Stage,
    /// Cumulative charged work in the immutable source/tolerance context.
    pub work_used: u64,
    pub context: ContextIdentity,
}

#[derive(Clone, Copy)]
enum Predicate {
    Orient2,
    Orient3,
    InCircle2,
    InSphere3,
    DirectionDot3,
    Distance(usize),
}

trait Algebra: Sized + Clone {
    fn add(&self, other: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason>;
    fn sub(&self, other: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason>;
    fn mul(&self, other: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason>;
}







pub fn orient2d(
    ctx: &mut PredicateContext<'_>,
    a: [LeafRef; 2],
    b: [LeafRef; 2],
    c: [LeafRef; 2],
) -> Result<Decision, InputError> {
    evaluate(
        ctx,
        Predicate::Orient2,
        &[a[0], a[1], b[0], b[1], c[0], c[1]],
    )
}

/// Sign of (b-a) dot (d-c), evaluated from original coordinates without
/// rounding either direction first. Zero certifies perpendicular directions;
/// callers must establish nondegeneracy separately.
pub fn direction_dot3d(
    ctx: &mut PredicateContext<'_>,
    a: [LeafRef; 3], b: [LeafRef; 3], c: [LeafRef; 3], d: [LeafRef; 3],
) -> Result<Decision, InputError> {
    evaluate(ctx, Predicate::DirectionDot3,
        &[a[0],a[1],a[2],b[0],b[1],b[2],c[0],c[1],c[2],d[0],d[1],d[2]])
}

pub fn orient3d(
    ctx: &mut PredicateContext<'_>,
    a: [LeafRef; 3],
    b: [LeafRef; 3],
    c: [LeafRef; 3],
    d: [LeafRef; 3],
) -> Result<Decision, InputError> {
    evaluate(
        ctx,
        Predicate::Orient3,
        &[
            a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], d[0], d[1], d[2],
        ],
    )
}

/// Authored 2D incircle predicate: positive when `d` lies strictly inside the
/// counter-clockwise circle through `(a, b, c)`, negative when strictly outside,
/// and zero when cocircular.
pub fn incircle2d_authored(
    ctx: &mut PredicateContext<'_>,
    a: [LeafRef; 2],
    b: [LeafRef; 2],
    c: [LeafRef; 2],
    d: [LeafRef; 2],
) -> Result<Decision, InputError> {
    evaluate(
        ctx,
        Predicate::InCircle2,
        &[a[0], a[1], b[0], b[1], c[0], c[1], d[0], d[1]],
    )
}

/// Adaptive exact 2D incircle predicate over finite binary64 coordinates.
pub fn incircle2d(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> Sign {
    let coords = [a[0], a[1], b[0], b[1], c[0], c[1], d[0], d[1]];
    if coords.iter().any(|v| !v.is_finite()) {
        return Sign::Zero;
    }
    let scalars = coords
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("incircle2d-ephemeral", 0, scalars) else {
        return Sign::Zero;
    };
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
    let pair = |offset: usize| [source.leaf(offset).unwrap(), source.leaf(offset + 1).unwrap()];
    match incircle2d_authored(&mut ctx, pair(0), pair(2), pair(4), pair(6)) {
        Ok(Decision {
            outcome: Outcome::Sign(sign),
            ..
        }) => sign,
        _ => Sign::Zero,
    }
}





fn distance_references(
    ctx: &PredicateContext<'_>,
    p: &[LeafRef],
    q: &[LeafRef],
) -> Result<Vec<LeafRef>, InputError> {
    if p.len() != q.len() || !(2..=3).contains(&p.len()) {
        return Err(InputError::InvalidInput(
            "Expected matching 2D or 3D points",
        ));
    }
    let references: Vec<_> = p.iter().chain(q).copied().collect();
    for reference in &references {
        ctx.resolve(*reference)?;
    }
    Ok(references)
}

pub fn compare_squared_distance(
    ctx: &mut PredicateContext<'_>,
    p: &[LeafRef],
    q: &[LeafRef],
    radius: LeafRef,
) -> Result<Decision, InputError> {
    let mut references = distance_references(ctx, p, q)?;
    let nonnegative = match *ctx.resolve(radius)? {
        AuthoredScalar::Binary64Bits(bits) => f64::from_bits(bits) >= 0.,
        AuthoredScalar::RationalConstant { numerator, .. } => numerator >= 0,
    };
    if !nonnegative {
        return Err(InputError::InvalidInput("Radius must be nonnegative"));
    }
    references.push(radius);
    evaluate(ctx, Predicate::Distance(p.len()), &references)
}

/// An enclosure issued only by this crate's distance recipe, bound to its
/// source snapshot and tolerance context. No public arbitrary-bound constructor.
#[derive(Debug)]
pub struct ProvenResidual {
    squared: Interval,
    context: ContextIdentity,
}

#[derive(Debug)]
pub enum ResidualOutcome {
    Proven(ProvenResidual),
    Indeterminate(Reason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelClass {
    Coincident,
    Separate,
    Indeterminate(Reason),
}



pub fn classify_residual(
    ctx: &mut PredicateContext<'_>,
    residual: Option<&ProvenResidual>,
) -> Result<ModelClass, InputError> {
    if residual.is_some_and(|r| r.context != ctx.identity()) {
        return Err(InputError::ContextMismatch);
    }
    let computed = (|| {
        ctx.charge(1)?;
        let residual = residual.ok_or(Reason::MissingProof)?;
        let on = Interval::point(ctx.tolerance.spec.on_tol);
        let clear = Interval::point(ctx.tolerance.spec.clear_tol);
        let on_squared = on.mul(&on, ctx)?;
        let clear_squared = clear.mul(&clear, ctx)?;
        ctx.charge(0)?;
        if residual.squared.hi < on_squared.lo {
            Ok(ModelClass::Coincident)
        } else if residual.squared.lo > clear_squared.hi {
            Ok(ModelClass::Separate)
        } else {
            Ok(ModelClass::Indeterminate(Reason::PrecisionExhausted))
        }
    })();
    Ok(computed.unwrap_or_else(ModelClass::Indeterminate))
}

#[cfg(test)]
mod tests;

mod bezier_point_identity;
pub use bezier_point_identity::*;

mod quadratic_shear_identity;
pub use quadratic_shear_identity::*;

mod projected_surface_jacobian;
pub use projected_surface_jacobian::Decision as SurfaceDerivativeDecision;
pub use projected_surface_jacobian::*;

mod quadratic_curve_separator;
pub use quadratic_curve_separator::*;
mod bilinear_graph_side;
pub use bilinear_graph_side::rational_bilinear_graph_side;

mod planar_bezier_roots;
pub use planar_bezier_roots::*;


/// Authored 3D insphere predicate: positive when `e` lies strictly inside the
/// sphere through `(a, b, c, d)` when `orient3d(a, b, c, d)` is positive,
/// negative when strictly outside, and zero when cospherical.
pub fn insphere3d_authored(
    ctx: &mut PredicateContext<'_>,
    a: [LeafRef; 3],
    b: [LeafRef; 3],
    c: [LeafRef; 3],
    d: [LeafRef; 3],
    e: [LeafRef; 3],
) -> Result<Decision, InputError> {
    evaluate(
        ctx,
        Predicate::InSphere3,
        &[
            a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], d[0], d[1], d[2], e[0], e[1],
            e[2],
        ],
    )
}



/// Fast 2D axis-aligned bounding-box rejection filter: returns `true` when closed
/// boxes `[min_a, max_a]` and `[min_b, max_b]` do not overlap.
#[inline]
pub fn aabb2_disjoint(min_a: [f64; 2], max_a: [f64; 2], min_b: [f64; 2], max_b: [f64; 2]) -> bool {
    max_a[0] < min_b[0] || max_b[0] < min_a[0] || max_a[1] < min_b[1] || max_b[1] < min_a[1]
}

/// Fast 3D axis-aligned bounding-box rejection filter: returns `true` when closed
/// boxes `[min_a, max_a]` and `[min_b, max_b]` do not overlap.
#[inline]
pub fn aabb3_disjoint(min_a: [f64; 3], max_a: [f64; 3], min_b: [f64; 3], max_b: [f64; 3]) -> bool {
    max_a[0] < min_b[0]
        || max_b[0] < min_a[0]
        || max_a[1] < min_b[1]
        || max_b[1] < min_a[1]
        || max_a[2] < min_b[2]
        || max_b[2] < min_a[2]
}





/// Exact 3D point-in-tetrahedron classification using adaptive `orient3d` predicates.
/// Returns `Sign::Positive` when `p` is strictly inside tetrahedron `(a, b, c, d)`,
/// `Sign::Zero` when `p` lies on a boundary face, edge, or vertex, and `Sign::Negative`
/// when `p` is outside (or if `(a, b, c, d)` is coplanar/degenerate), independent of
/// the orientation of `(a, b, c, d)`.
pub fn point_in_tetrahedron_3d(
    p: [f64; 3],
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    d: [f64; 3],
) -> Sign {
    let coords = [
        p[0], p[1], p[2], a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], d[0], d[1], d[2],
    ];
    if coords.iter().any(|v| !v.is_finite()) {
        return Sign::Negative;
    }
    let scalars = coords
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("pt-in-tet-3d-ephemeral", 0, scalars) else {
        return Sign::Negative;
    };
    let tolerance = ToleranceContext::default_valid();
    let pt = |offset: usize| {
        [
            source.leaf(offset).unwrap(),
            source.leaf(offset + 1).unwrap(),
            source.leaf(offset + 2).unwrap(),
        ]
    };
    let orient = |q0: usize, q1: usize, q2: usize, q3: usize| -> Option<Sign> {
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        match orient3d(&mut ctx, pt(q0), pt(q1), pt(q2), pt(q3)) {
            Ok(Decision {
                outcome: Outcome::Sign(s),
                ..
            }) => Some(s),
            _ => None,
        }
    };
    let Some(tet_orient) = orient(3, 6, 9, 12) else {
        return Sign::Negative;
    };
    if tet_orient == Sign::Zero {
        return Sign::Negative;
    }
    let (Some(s0), Some(s1), Some(s2), Some(s3)) = (
        orient(0, 6, 9, 12),
        orient(3, 0, 9, 12),
        orient(3, 6, 0, 12),
        orient(3, 6, 9, 0),
    ) else {
        return Sign::Negative;
    };
    if [s0, s1, s2, s3].iter().all(|&s| s == tet_orient) {
        return Sign::Positive;
    }
    if [s0, s1, s2, s3]
        .iter()
        .all(|&s| s == tet_orient || s == Sign::Zero)
    {
        return Sign::Zero;
    }
    Sign::Negative
}

/// Exact 3D segment–triangle intersection classification using adaptive `orient3d` predicates
/// with a fast [`aabb3_disjoint`] pre-rejection filter.
///
/// Returns:
/// - `Sign::Positive` when the open segment `(seg_a, seg_b)` pierces the strict interior of
///   triangle `(tri_a, tri_b, tri_c)`,
/// - `Sign::Zero` when the closed segment `[seg_a, seg_b]` touches the triangle boundary
///   (edge or vertex) or when a segment endpoint lies on the closed triangle,
/// - `Sign::Negative` when the segment and triangle are disjoint (or if the triangle is degenerate).
pub fn segment_triangle_intersect_3d_exact(
    seg_a: [f64; 3],
    seg_b: [f64; 3],
    tri_a: [f64; 3],
    tri_b: [f64; 3],
    tri_c: [f64; 3],
) -> Sign {
    let coords = [
        seg_a[0], seg_a[1], seg_a[2], seg_b[0], seg_b[1], seg_b[2], tri_a[0], tri_a[1], tri_a[2],
        tri_b[0], tri_b[1], tri_b[2], tri_c[0], tri_c[1], tri_c[2],
    ];
    if coords.iter().any(|v| !v.is_finite()) {
        return Sign::Negative;
    }
    let min_seg = [
        seg_a[0].min(seg_b[0]),
        seg_a[1].min(seg_b[1]),
        seg_a[2].min(seg_b[2]),
    ];
    let max_seg = [
        seg_a[0].max(seg_b[0]),
        seg_a[1].max(seg_b[1]),
        seg_a[2].max(seg_b[2]),
    ];
    let min_tri = [
        tri_a[0].min(tri_b[0]).min(tri_c[0]),
        tri_a[1].min(tri_b[1]).min(tri_c[1]),
        tri_a[2].min(tri_b[2]).min(tri_c[2]),
    ];
    let max_tri = [
        tri_a[0].max(tri_b[0]).max(tri_c[0]),
        tri_a[1].max(tri_b[1]).max(tri_c[1]),
        tri_a[2].max(tri_b[2]).max(tri_c[2]),
    ];
    if aabb3_disjoint(min_seg, max_seg, min_tri, max_tri) {
        return Sign::Negative;
    }
    let scalars = coords
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("seg-tri-3d-ephemeral", 0, scalars) else {
        return Sign::Negative;
    };
    let tolerance = ToleranceContext::default_valid();
    let pt = |offset: usize| {
        [
            source.leaf(offset).unwrap(),
            source.leaf(offset + 1).unwrap(),
            source.leaf(offset + 2).unwrap(),
        ]
    };
    let orient = |q0: usize, q1: usize, q2: usize, q3: usize| -> Option<Sign> {
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        match orient3d(&mut ctx, pt(q0), pt(q1), pt(q2), pt(q3)) {
            Ok(Decision {
                outcome: Outcome::Sign(s),
                ..
            }) => Some(s),
            _ => None,
        }
    };
    let (Some(oa), Some(ob)) = (orient(6, 9, 12, 0), orient(6, 9, 12, 3)) else {
        return Sign::Negative;
    };
    if oa != Sign::Zero && oa == ob {
        return Sign::Negative;
    }
    let coplanar_pt_in_tri = |p: [f64; 3]| -> Sign {
        let ab = [tri_b[0] - tri_a[0], tri_b[1] - tri_a[1], tri_b[2] - tri_a[2]];
        let ac = [tri_c[0] - tri_a[0], tri_c[1] - tri_a[1], tri_c[2] - tri_a[2]];
        let nx = (ab[1] * ac[2] - ab[2] * ac[1]).abs();
        let ny = (ab[2] * ac[0] - ab[0] * ac[2]).abs();
        let nz = (ab[0] * ac[1] - ab[1] * ac[0]).abs();
        let (i0, i1) = if nx >= ny && nx >= nz {
            (1, 2)
        } else if ny >= nz {
            (2, 0)
        } else {
            (0, 1)
        };
        point_in_triangle_2d(
            [p[i0], p[i1]],
            [tri_a[i0], tri_a[i1]],
            [tri_b[i0], tri_b[i1]],
            [tri_c[i0], tri_c[i1]],
        )
    };
    if oa == Sign::Zero && ob == Sign::Zero {
        let ab = [tri_b[0] - tri_a[0], tri_b[1] - tri_a[1], tri_b[2] - tri_a[2]];
        let ac = [tri_c[0] - tri_a[0], tri_c[1] - tri_a[1], tri_c[2] - tri_a[2]];
        let nx = (ab[1] * ac[2] - ab[2] * ac[1]).abs();
        let ny = (ab[2] * ac[0] - ab[0] * ac[2]).abs();
        let nz = (ab[0] * ac[1] - ab[1] * ac[0]).abs();
        let (i0, i1) = if nx >= ny && nx >= nz {
            (1, 2)
        } else if ny >= nz {
            (2, 0)
        } else {
            (0, 1)
        };
        let sa2 = [seg_a[i0], seg_a[i1]];
        let sb2 = [seg_b[i0], seg_b[i1]];
        let ta2 = [tri_a[i0], tri_a[i1]];
        let tb2 = [tri_b[i0], tri_b[i1]];
        let tc2 = [tri_c[i0], tri_c[i1]];
        if point_in_triangle_2d(sa2, ta2, tb2, tc2) != Sign::Negative
            || point_in_triangle_2d(sb2, ta2, tb2, tc2) != Sign::Negative
            || segment_intersects_segment_2d(sa2, sb2, ta2, tb2)
            || segment_intersects_segment_2d(sa2, sb2, tb2, tc2)
            || segment_intersects_segment_2d(sa2, sb2, tc2, ta2)
        {
            return Sign::Zero;
        }
        return Sign::Negative;
    }
    if oa == Sign::Zero {
        return if coplanar_pt_in_tri(seg_a) != Sign::Negative {
            Sign::Zero
        } else {
            Sign::Negative
        };
    }
    if ob == Sign::Zero {
        return if coplanar_pt_in_tri(seg_b) != Sign::Negative {
            Sign::Zero
        } else {
            Sign::Negative
        };
    }
    // oa and ob have strictly opposite signs: segment straddles the triangle plane.
    let (Some(s0), Some(s1), Some(s2)) = (
        orient(0, 3, 6, 9),
        orient(0, 3, 9, 12),
        orient(0, 3, 12, 6),
    ) else {
        return Sign::Negative;
    };
    // Because orient3d(tri_a, tri_b, tri_c, seg_a) = oa and orient3d(tri_a, tri_b, tri_c, seg_b) = ob = -oa,
    // all three edge-tetrahedra (seg_a, seg_b, e0, e1) must match `ob` (or be Zero on an edge/vertex).
    if [s0, s1, s2].iter().all(|&s| s == ob) {
        return Sign::Positive;
    }
    if [s0, s1, s2].iter().all(|&s| s == ob || s == Sign::Zero) {
        return Sign::Zero;
    }
    Sign::Negative
}

/// Exact adaptive 3D tetrahedron orientation sign alongside the signed determinant magnitude `6 * V`.
///
/// The sign is certified by the exact adaptive `orient3d(a, b, c, d)` predicate, while the
/// scalar `6 * V = ((b - a) × (c - a)) · (d - a)` is evaluated with exact expansion arithmetic
/// (falling back to the binary64 determinant if finite, or `0.0` when exact `orient3d` is `Sign::Zero`).
pub fn tetrahedron_signed_volume_6x(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    d: [f64; 3],
) -> (Sign, f64) {
    let coords = [
        a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], d[0], d[1], d[2],
    ];
    if coords.iter().any(|v| !v.is_finite()) {
        return (Sign::Zero, 0.0);
    }
    let scalars: Vec<_> = coords
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("tet-vol-6x-ephemeral", 0, scalars) else {
        return (Sign::Zero, 0.0);
    };
    let tolerance = ToleranceContext::default_valid();
    let pt = |offset: usize| {
        [
            source.leaf(offset).unwrap(),
            source.leaf(offset + 1).unwrap(),
            source.leaf(offset + 2).unwrap(),
        ]
    };
    let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
    let sign = match orient3d(&mut ctx, pt(0), pt(3), pt(6), pt(9)) {
        Ok(Decision {
            outcome: Outcome::Sign(s),
            ..
        }) => s,
        _ => Sign::Zero,
    };
    if sign == Sign::Zero {
        return (Sign::Zero, 0.0);
    }
    let mut exp_ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
    let inputs: Vec<Expansion> = coords.iter().map(|&v| Expansion::scalar(v)).collect();
    let det_6v = if let Ok(exp) = expression(Predicate::Orient3, &inputs, &mut exp_ctx)
        && let Ok(interval) = exp.enclosure(&mut exp_ctx)
    {
        let mid = 0.5 * (interval.lo + interval.hi);
        if mid.is_finite() && mid != 0.0 {
            mid
        } else {
            let bx = b[0] - a[0];
            let by = b[1] - a[1];
            let bz = b[2] - a[2];
            let cx = c[0] - a[0];
            let cy = c[1] - a[1];
            let cz = c[2] - a[2];
            let dx = d[0] - a[0];
            let dy = d[1] - a[1];
            let dz = d[2] - a[2];
            bx * (cy * dz - cz * dy) + by * (cz * dx - cx * dz) + bz * (cx * dy - cy * dx)
        }
    } else {
        let bx = b[0] - a[0];
        let by = b[1] - a[1];
        let bz = b[2] - a[2];
        let cx = c[0] - a[0];
        let cy = c[1] - a[1];
        let cz = c[2] - a[2];
        let dx = d[0] - a[0];
        let dy = d[1] - a[1];
        let dz = d[2] - a[2];
        bx * (cy * dz - cz * dy) + by * (cz * dx - cx * dz) + bz * (cx * dy - cy * dx)
    };
    (sign, det_6v)
}

/// Exact 2D barycentric sub-triangle orientation signs `(orient2d(b, c, p), orient2d(c, a, p), orient2d(a, b, p))`
/// for exact topological classification of point `p` with respect to triangle `(a, b, c)`
/// (interior, edge 0/1/2, vertex 0/1/2, or exterior).
pub fn barycentric_in_triangle_2d_exact(
    a: [f64; 2],
    b: [f64; 2],
    c: [f64; 2],
    p: [f64; 2],
) -> (Sign, Sign, Sign) {
    let coords = [a[0], a[1], b[0], b[1], c[0], c[1], p[0], p[1]];
    if coords.iter().any(|v| !v.is_finite()) {
        return (Sign::Zero, Sign::Zero, Sign::Zero);
    }
    let scalars = coords
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("bary-tri-2d-ephemeral", 0, scalars) else {
        return (Sign::Zero, Sign::Zero, Sign::Zero);
    };
    let tolerance = ToleranceContext::default_valid();
    let pair = |offset: usize| [source.leaf(offset).unwrap(), source.leaf(offset + 1).unwrap()];
    let orient = |q0: usize, q1: usize, q2: usize| -> Sign {
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        match orient2d(&mut ctx, pair(q0), pair(q1), pair(q2)) {
            Ok(Decision {
                outcome: Outcome::Sign(s),
                ..
            }) => s,
            _ => Sign::Zero,
        }
    };
    (orient(2, 4, 6), orient(4, 0, 6), orient(0, 2, 6))
}

#[path="lib/algorithm_expression.rs"]
mod algorithm_expression;
use algorithm_expression::expression;
#[path="lib/algorithm_exact_inputs.rs"]
mod algorithm_exact_inputs;
use algorithm_exact_inputs::exact_inputs;
#[path="lib/algorithm_evaluate.rs"]
mod algorithm_evaluate;
use algorithm_evaluate::evaluate;
#[path="lib/algorithm_coplanar3d.rs"]
mod algorithm_coplanar3d;
pub use algorithm_coplanar3d::coplanar3d;
#[path="lib/algorithm_collinear3d.rs"]
mod algorithm_collinear3d;
pub use algorithm_collinear3d::collinear3d;
#[path="lib/algorithm_distance_enclosure.rs"]
mod algorithm_distance_enclosure;
pub use algorithm_distance_enclosure::distance_enclosure;
#[path="lib/algorithm_insphere3d.rs"]
mod algorithm_insphere3d;
pub use algorithm_insphere3d::insphere3d;
#[path="lib/algorithm_segment_intersects_segment_2d.rs"]
mod algorithm_segment_intersects_segment_2d;
pub use algorithm_segment_intersects_segment_2d::segment_intersects_segment_2d;
#[path="lib/algorithm_point_in_triangle_2d.rs"]
mod algorithm_point_in_triangle_2d;
pub use algorithm_point_in_triangle_2d::point_in_triangle_2d;
