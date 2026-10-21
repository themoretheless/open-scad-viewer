//! Bounded, geometry-only intersection queries on retained rational definitions.
//!
//! This is the numerical foundation for sectioning and future curved Booleans,
//! not a Boolean topology oracle. Bernstein sign bounds and parameter boxes are
//! retained, but knot insertion/evaluation roundoff and branch adjacency do not
//! yet have independent coverage certificates. Even `NumericallyResolved` MUST
//! NOT be consumed as a certificate authorizing a topology change. No triangles,
//! fitted curves, snapping, or tessellation participate in these queries.
#[cfg(feature = "codec")]
mod serialization;
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};

mod cone_cone;
mod cone_torus;
mod cylinder_cylinder;
mod cylinder_torus;
mod plane_cone;
mod plane_cylinder;
mod plane_sphere;
mod plane_torus;
pub(crate) mod recognize;
mod sphere_cone;
mod sphere_cylinder;
pub mod sphere_sphere;
mod sphere_torus;
#[cfg(test)]
mod test_utils;
mod torus_torus;
pub use cone_cone::{ConeConeComponent, intersect_cone_cone};
pub use cone_torus::{ConeTorusComponent, intersect_cone_torus};
pub use cylinder_cylinder::{CylinderCylinderComponent, intersect_cylinder_cylinder};
pub use cylinder_torus::{CylinderTorusComponent, intersect_cylinder_torus};
pub use plane_cone::{PlaneConeComponent, intersect_plane_cone};
pub use plane_cylinder::{PlaneCylinderComponent, intersect_plane_cylinder};
pub use plane_sphere::{PlanePatchCurve, PlaneSphereComponent, intersect_plane_sphere};
pub use plane_torus::{PlaneTorusComponent, TorusPatchCurve, intersect_plane_torus};
pub use sphere_cone::{SphereConeComponent, intersect_sphere_cone};
pub use sphere_cylinder::{CylinderPatchCurve, SphereCylinderComponent, intersect_sphere_cylinder};
pub use sphere_sphere::{SpherePatchCircle, SphereSphereComponent, intersect_sphere_sphere};
pub use sphere_torus::{SphereTorusComponent, intersect_sphere_torus};
pub use torus_torus::{TorusTorusComponent, intersect_torus_torus};

pub use plane_cone::{CanonicalCone, recognize_cone};
pub use plane_torus::recognize_torus;
pub use sphere_cylinder::{CanonicalCylinder, recognize_cylinder};
pub use sphere_sphere::recognize as recognize_sphere;

mod query_options;
pub use query_options::{Options, Plane};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    /// Domain partition certified for the frozen analytic/affine matrix only.
    Complete,
    /// All parameter regions were handled numerically. NOT certified complete.
    NumericallyResolved,
    Incomplete,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnresolvedReason {
    BudgetExceeded,
    TangencyOrMultipleRoot,
    NearCoincidence,
    BoundaryCrossing,
    UnsupportedSurface,
    CoincidentTrim,
}
#[derive(Clone, Debug)]
pub struct Unresolved {
    /// Curve: [t0,t1]. Surface: [u0,u1,v0,v1]. Surface pairs concatenate both boxes.
    pub parameter_box: Vec<f64>,
    pub reason: UnresolvedReason,
}
#[derive(Clone, Debug)]
pub struct Report<T> {
    pub components: Vec<T>,
    pub unresolved: Vec<Unresolved>,
    pub boxes_visited: usize,
    pub bernstein_excluded: usize,
    pub coverage: Coverage,
}
impl<T> Default for Report<T> {
    fn default() -> Self {
        Self {
            components: Vec::new(),
            unresolved: Vec::new(),
            boxes_visited: 0,
            bernstein_excluded: 0,
            coverage: Coverage::NumericallyResolved,
        }
    }
}
impl<T> Report<T> {
    /// Always false until independent coverage and correspondence certificates
    /// exist. Keeping this query beside coverage prevents accidental promotion.
    pub fn permits_topology_change(&self) -> bool {
        false
    }
    pub(crate) fn unresolved(&mut self, domain: impl Into<Vec<f64>>, reason: UnresolvedReason) {
        self.coverage = Coverage::Incomplete;
        self.unresolved.push(Unresolved {
            parameter_box: domain.into(),
            reason,
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contact {
    Transverse,
    Boundary,
}
#[derive(Clone, Debug)]
pub struct CurvePoint {
    pub parameter: f64,
    pub parameter_interval: [f64; 2],
    pub point: [f64; 3],
    pub plane_residual: f64,
    pub contact: Contact,
}
#[derive(Clone, Debug)]
pub enum CurvePlaneComponent {
    Point(CurvePoint),
    /// All retained Bezier control points lie numerically in the plane.
    Overlap {
        parameter_interval: [f64; 2],
        control_residual: f64,
    },
}

/// Outward binary64 interval arithmetic is used only for sign exclusion on the
/// current homogeneous Bernstein data. It does not cover preceding knot edits.
#[derive(Clone, Copy, Debug)]
struct Interval {
    lo: f64,
    hi: f64,
}
impl Interval {
    fn whole() -> Self {
        Self {
            lo: f64::NEG_INFINITY,
            hi: f64::INFINITY,
        }
    }
    fn exact(x: f64) -> Self {
        Self { lo: x, hi: x }
    }
    fn add(self, b: Self) -> Self {
        if self.lo == 0. && self.hi == 0. {
            return b;
        }
        if b.lo == 0. && b.hi == 0. {
            return self;
        }
        if self.lo == self.hi && b.lo == b.hi && self.lo.is_finite() && self.lo == -b.lo {
            return Self::exact(0.);
        }
        if (self.lo + b.lo).is_nan() || (self.hi + b.hi).is_nan() {
            return Self::whole();
        }
        Self {
            lo: (self.lo + b.lo).next_down(),
            hi: (self.hi + b.hi).next_up(),
        }
    }
    fn mul(self, b: Self) -> Self {
        if self.lo == 1. && self.hi == 1. {
            return b;
        }
        if b.lo == 1. && b.hi == 1. {
            return self;
        }
        if self.lo == -1. && self.hi == -1. {
            return Self {
                lo: -b.hi,
                hi: -b.lo,
            };
        }
        if b.lo == -1. && b.hi == -1. {
            return Self {
                lo: -self.hi,
                hi: -self.lo,
            };
        }
        if (self.lo == 0. && self.hi == 0. && b.lo.is_finite() && b.hi.is_finite())
            || (b.lo == 0. && b.hi == 0. && self.lo.is_finite() && self.hi.is_finite())
        {
            return Self::exact(0.);
        }
        let values = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        if values.iter().any(|v| v.is_nan()) {
            return Self::whole();
        }
        Self {
            lo: values.into_iter().fold(f64::INFINITY, f64::min).next_down(),
            hi: values
                .into_iter()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        }
    }
    fn sign(self) -> i8 {
        if self.lo > 0. {
            1
        } else if self.hi < 0. {
            -1
        } else {
            0
        }
    }
}

#[derive(Clone, Copy)]
struct Coefficient {
    value: f64,
    bound: Interval,
}
fn coefficients(curve: &Curve, plane: Plane) -> Vec<Coefficient> {
    curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(point, weight)| {
            let mut bound = Interval::exact(-plane.offset);
            for (a, b) in plane.normal.into_iter().zip(point) {
                bound = bound.add(Interval::exact(a).mul(Interval::exact(*b)));
            }
            Coefficient {
                value: plane.distance(point3(point)) * weight,
                bound: bound.mul(Interval::exact(*weight)),
            }
        })
        .collect()
}
fn split(coefficients: &[Coefficient]) -> (Vec<Coefficient>, Vec<Coefficient>) {
    let n = coefficients.len();
    let mut row = coefficients.to_vec();
    let mut left = Vec::with_capacity(n);
    let mut right = Vec::with_capacity(n);
    left.push(row[0]);
    right.push(*row.last().unwrap());
    for len in (1..n).rev() {
        for i in 0..len {
            row[i] = Coefficient {
                value: (row[i].value + row[i + 1].value) * 0.5,
                bound: row[i].bound.add(row[i + 1].bound).mul(Interval::exact(0.5)),
            };
        }
        left.push(row[0]);
        right.push(row[len - 1]);
    }
    right.reverse();
    (left, right)
}
fn excluded(coefficients: &[Coefficient]) -> bool {
    coefficients.iter().all(|c| c.bound.sign() == 1)
        || coefficients.iter().all(|c| c.bound.sign() == -1)
}
fn sign_variations(coefficients: &[Coefficient]) -> usize {
    let mut count = 0usize;
    let mut prev: Option<bool> = None;
    for c in coefficients {
        if c.value != 0. {
            let s = c.value > 0.;
            if let Some(p) = prev
                && p != s
            {
                count += 1;
            }
            prev = Some(s);
        }
    }
    count
}
fn spans(knots: &[f64], degree: usize, count: usize) -> Vec<[f64; 2]> {
    knots[degree..=count]
        .windows(2)
        .filter_map(|s| (s[0] < s[1]).then_some([s[0], s[1]]))
        .collect()
}

/// Half-open box ownership shared by every subdivision query in this file:
/// each box owns its interior and its lower-parameter face; the domain's
/// upper end is owned by the last box. Exactly one box therefore owns any
/// parameter, so a root sitting bitwise on a shared subdivision face is
/// reported once by its owning box and halo boxes stay silent.
fn owns_parameter(lo: f64, hi: f64, domain_hi: f64, x: f64) -> bool {
    lo <= x && (x < hi || (x == hi && hi == domain_hi))
}
/// Snap a converged parameter onto a box face when it rounds within two
/// binary64 steps of it. Adjacent boxes share faces bitwise, so Newton images
/// that disagree by one rounding step resolve to the same owned face
/// parameter. This reconciles rounding at a shared face only — it is not a
/// spatial tolerance and never merges distinct parameters.
fn snap_to_face(x: f64, face: f64) -> f64 {
    let near = if x < face {
        [face.next_down(), face.next_down().next_down()]
    } else {
        [face.next_up(), face.next_up().next_up()]
    };
    if x == face || near.contains(&x) {
        face
    } else {
        x
    }
}

/// Isolate intersections of any validated positive-weight 3D NURBS curve with
/// a plane, over the curve's entire active knot domain. Multiple/tangent roots,
/// precision bands and work exhaustion stay explicit instead of becoming empty.
pub fn curve_plane(
    curve: &Curve,
    plane: Plane,
    options: Options,
) -> Result<Report<CurvePlaneComponent>> {
    curve.validate()?;
    if curve.control_points[0].len() != 3 {
        return Err(invalid("Curve/plane requires a 3D curve"));
    }
    let plane = plane.normalized()?;
    let options = options.validate()?;
    let mut report = Report::default();
    let mut pending: std::collections::VecDeque<([f64; 2], Option<Vec<Coefficient>>, usize)> =
        spans(&curve.knots, curve.degree, curve.control_points.len())
            .into_iter()
            .map(|domain| (domain, None, 0))
            .collect();
    while let Some((interval, values, depth)) = pending.pop_front() {
        if report.boxes_visited >= options.max_boxes {
            report.unresolved(interval, UnresolvedReason::BudgetExceeded);
            continue;
        }
        report.boxes_visited += 1;
        let initial_span = values.is_none();
        let values = match values {
            Some(values) => values,
            None => coefficients(&curve.trim(interval[0], interval[1])?, plane),
        };
        if initial_span && values.iter().all(|c| c.value == 0.) {
            report.components.push(CurvePlaneComponent::Overlap {
                parameter_interval: interval,
                control_residual: 0.,
            });
            continue;
        }
        if excluded(&values) {
            report.bernstein_excluded += 1;
            continue;
        }
        // Endpoints belong to both neighboring spans; only exact numerical
        // zeros are merged, never distinct roots merely closer than a tol.
        for (parameter, coefficient) in [
            (interval[0], values[0]),
            (interval[1], *values.last().unwrap()),
        ] {
            if coefficient.value == 0. {
                let jet = curve.evaluate(parameter)?;
                let point = point3(&jet.point);
                if plane.distance(point).abs() <= options.distance_tolerance {
                    // At a C0 knot the two-sided jet does not exist; screen
                    // transversality with the one-sided span jets and accept
                    // either side that confirms a transverse crossing.
                    let transverse = curve_tangents(curve, parameter)?
                        .iter()
                        .any(|d| dot(plane.normal, *d).abs() > options.distance_tolerance);
                    if parameter != curve.domain()[0]
                        && parameter != curve.domain()[1]
                        && !transverse
                    {
                        report.unresolved(interval, UnresolvedReason::TangencyOrMultipleRoot);
                    } else if !report.components.iter().any(
                        |c| matches!(c, CurvePlaneComponent::Point(p) if p.parameter == parameter),
                    ) {
                        report
                            .components
                            .push(CurvePlaneComponent::Point(CurvePoint {
                                parameter,
                                parameter_interval: [parameter, parameter],
                                point,
                                plane_residual: plane.distance(point).abs(),
                                contact: Contact::Boundary,
                            }));
                    }
                } else {
                    report.unresolved(interval, UnresolvedReason::NearCoincidence);
                }
            }
        }
        let variations = sign_variations(&values);
        if variations == 0 {
            // A root bitwise on a shared box face leaves no sign variation:
            // decide both faces by exact evaluation, but only where the
            // face's own coefficient bound cannot exclude zero — a strict
            // bound proves the face value is no rounding-scale contact.
            // Half-open ownership makes exactly one box report the face;
            // the halo box stays silent.
            let domain_hi = curve.domain()[1];
            let mut handled = false;
            for (face, coefficient) in [
                (interval[0], values[0]),
                (interval[1], *values.last().unwrap()),
            ] {
                if coefficient.bound.sign() != 0 {
                    continue;
                }
                match plane_face_root(curve, plane, interval, face, options)? {
                    FaceRoot::Root(point) => {
                        if owns_parameter(interval[0], interval[1], domain_hi, face)
                            && !report.components.iter().any(
                                |c| matches!(c, CurvePlaneComponent::Point(p) if p.parameter == point.parameter),
                            )
                        {
                            report.components.push(CurvePlaneComponent::Point(point));
                        }
                        handled = true;
                    }
                    FaceRoot::Ambiguous => {
                        report.unresolved(interval, UnresolvedReason::TangencyOrMultipleRoot);
                        handled = true;
                    }
                    FaceRoot::Miss => {}
                }
            }
            if handled {
                // Interior coefficients with zero-containing bounds keep
                // their explicit band; a confirmed face does not clear them.
                if values[1..values.len() - 1]
                    .iter()
                    .any(|c| c.value != 0. && c.bound.sign() == 0)
                {
                    report.unresolved(interval, UnresolvedReason::NearCoincidence);
                }
                continue;
            }
            // Numerically one-sided Bernstein coefficients. This is not an
            // interval exclusion if zero-containing coefficient intervals
            // remain; coverage is deliberately only numerical throughout.
            if values.iter().any(|c| c.value != 0. && c.bound.sign() == 0) {
                report.unresolved(interval, UnresolvedReason::NearCoincidence);
            }
            continue;
        }
        let width = interval[1] - interval[0];
        let midpoint = interval[0] + width * 0.5;
        if width <= options.parameter_tolerance
            || depth == options.max_depth
            || midpoint == interval[0]
            || midpoint == interval[1]
        {
            // Terminal boxes compare the midpoint candidate against exact
            // face evaluations: a face whose residual is not beaten by the
            // midpoint carries the root — report it at the exact face
            // parameter with this box as the isolating interval. Exact
            // parameters dedup against the neighbor's own face report, and
            // the final touching-interval merge absorbs any remaining
            // midpoint/face pair beside a shared face.
            let point = point3(&curve.evaluate(midpoint)?.point);
            let residual = plane.distance(point).abs();
            let midpoint_confirms = variations == 1 && residual <= options.distance_tolerance;
            let mut best_face: Option<(f64, f64)> = None;
            for &face in &[interval[0], interval[1]] {
                let face_residual = plane.distance(point3(&curve.evaluate(face)?.point)).abs();
                if face_residual <= options.distance_tolerance {
                    best_face = Some(match best_face {
                        Some((f, r)) if r <= face_residual => (f, r),
                        _ => (face, face_residual),
                    });
                }
            }
            if let Some((face, face_residual)) = best_face
                && (!midpoint_confirms || face_residual <= residual)
            {
                match plane_face_root(curve, plane, interval, face, options)? {
                    FaceRoot::Root(mut point) => {
                        point.parameter_interval = interval;
                        if !report.components.iter().any(
                                |c| matches!(c, CurvePlaneComponent::Point(p) if p.parameter == point.parameter),
                            ) {
                                report.components.push(CurvePlaneComponent::Point(point));
                            }
                    }
                    FaceRoot::Ambiguous => {
                        report.unresolved(interval, UnresolvedReason::TangencyOrMultipleRoot)
                    }
                    FaceRoot::Miss => {}
                }
                if variations > 1 {
                    report.unresolved(interval, UnresolvedReason::TangencyOrMultipleRoot);
                }
                continue;
            }
            if midpoint_confirms {
                report
                    .components
                    .push(CurvePlaneComponent::Point(CurvePoint {
                        parameter: midpoint,
                        parameter_interval: interval,
                        point,
                        plane_residual: residual,
                        contact: Contact::Transverse,
                    }));
            } else {
                report.unresolved(interval, UnresolvedReason::TangencyOrMultipleRoot);
            }
            continue;
        }
        let (left, right) = split(&values);
        pending.push_back(([interval[0], midpoint], Some(left), depth + 1));
        pending.push_back(([midpoint, interval[1]], Some(right), depth + 1));
    }
    merge_plane_points(&mut report);
    report.components.sort_by(|a, b| {
        let parameter = |c: &CurvePlaneComponent| match c {
            CurvePlaneComponent::Point(p) => p.parameter,
            CurvePlaneComponent::Overlap {
                parameter_interval, ..
            } => parameter_interval[0],
        };
        parameter(a).total_cmp(&parameter(b))
    });
    Ok(report)
}
/// Merge curve/plane point events whose isolating intervals touch in
/// parameter space — the same subdivision-tiling adjacency rule as
/// merge_curve_points, never spatial proximity. The merged event keeps the
/// lowest-residual parameter and the union of the isolating intervals, so a
/// root sitting on a shared box face is reported exactly once.
fn merge_plane_points(report: &mut Report<CurvePlaneComponent>) {
    let interval = |c: &CurvePlaneComponent| match c {
        CurvePlaneComponent::Point(p) => Some(p.parameter_interval),
        _ => None,
    };
    let n = report.components.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for i in 0..n {
        for j in i + 1..n {
            let (Some(a), Some(b)) = (
                interval(&report.components[i]),
                interval(&report.components[j]),
            ) else {
                continue;
            };
            if a[0] <= b[1] && b[0] <= a[1] {
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                if ri != rj {
                    parent[ri] = rj;
                }
            }
        }
    }
    let mut merged: Vec<Option<CurvePlaneComponent>> = (0..n).map(|_| None).collect();
    let mut order: Vec<usize> = Vec::new();
    for (i, component) in report.components.iter().take(n).enumerate() {
        let Some(iv) = interval(component) else {
            continue;
        };
        let root = find(&mut parent, i);
        if merged[root].is_none() {
            order.push(root);
        }
        let CurvePlaneComponent::Point(member) = component else {
            continue;
        };
        match &mut merged[root] {
            None => merged[root] = Some(CurvePlaneComponent::Point(member.clone())),
            Some(CurvePlaneComponent::Point(point)) => {
                if member.plane_residual < point.plane_residual {
                    point.parameter = member.parameter;
                    point.point = member.point;
                    point.plane_residual = member.plane_residual;
                }
                if member.contact == Contact::Boundary {
                    point.contact = Contact::Boundary;
                }
                point.parameter_interval[0] = point.parameter_interval[0].min(iv[0]);
                point.parameter_interval[1] = point.parameter_interval[1].max(iv[1]);
            }
            _ => unreachable!(),
        }
    }
    let mut components = Vec::new();
    let mut index = 0;
    for root in order {
        while index < root {
            if interval(&report.components[index]).is_none() {
                components.push(report.components[index].clone());
            }
            index += 1;
        }
        if let Some(c) = merged[root].take() {
            components.push(c);
        }
        index = root + 1;
    }
    while index < n {
        if interval(&report.components[index]).is_none() {
            components.push(report.components[index].clone());
        }
        index += 1;
    }
    report.components = components;
}

#[derive(Clone, Debug)]
pub struct SurfacePoint {
    pub uv: [f64; 2],
    pub point: [f64; 3],
    pub plane_residual: f64,
}

#[derive(Clone, Debug)]
pub enum SurfaceTrace {
    /// A ruled trace linear in U, parameterized by original V.
    RuledU {
        surface: Surface,
        plane: Plane,
        v_interval: [f64; 2],
    },
    /// Affine line in the surface's original parameter coordinates.
    Line {
        surface: Surface,
        plane: Plane,
        start: [f64; 2],
        end: [f64; 2],
    },
    /// A retained ruled rational surface and plane define v(u) exactly as a
    /// procedural trace. This avoids replacing a conic with fitted samples.
    /// This variant is linear in V; RuledU retains the transposed case.
    Ruled {
        surface: Surface,
        plane: Plane,
        u_interval: [f64; 2],
    },
}
impl SurfaceTrace {
    /// Ordered rational pieces on the original [0,1] trace-fraction domain.
    /// Knot-crossing endpoints remain independent numerical values: this API
    /// does not reconcile them or grant permission to sew topology.
    pub fn to_curve_segments(&self) -> Result<Vec<Curve>> {
        self.evaluate(0.)?;
        self.evaluate(1.)?;
        match self {
            Self::Ruled {
                surface,
                plane,
                u_interval,
            } => return ruled_segments(surface, *plane, *u_interval),
            Self::RuledU {
                surface,
                plane,
                v_interval,
            } => return ruled_segments(&transpose_surface(surface), *plane, *v_interval),
            _ => (),
        }
        if let Self::Line {
            surface,
            plane,
            start,
            end,
        } = self
            && start[0] != end[0]
            && start[1] != end[1]
        {
            if surface.degree_u + surface.degree_v > 25 {
                return Err(invalid(
                    "UV diagonal conversion requires result degree at most 25",
                ));
            }
            return diagonal_segments(surface, *plane, *start, *end);
        }
        let mut curve = self.to_curve()?;
        let [lo, hi] = curve.domain();
        for knot in &mut curve.knots {
            *knot = (*knot - lo) / (hi - lo);
        }
        curve.validate()?;
        Ok(vec![curve])
    }
    /// Algebraic conversion of supported retained traces, never sample fitting.
    /// Floating-point coefficient arithmetic is numerical, not certified exact.
    /// Fraction maps linearly to the returned curve's active knot domain.
    pub fn to_curve(&self) -> Result<Curve> {
        let start_point = self.evaluate(0.)?.point;
        let end_point = self.evaluate(1.)?.point;
        match self {
            Self::RuledU {
                surface,
                plane,
                v_interval,
            } => Self::Ruled {
                surface: transpose_surface(surface),
                plane: *plane,
                u_interval: *v_interval,
            }
            .to_curve(),
            Self::Line {
                surface,
                plane,
                start,
                end,
            } => {
                let (axis, constant, range) = if start[1] == end[1] && start[0] != end[0] {
                    (nurbs_core::surface::Axis::V, start[1], [start[0], end[0]])
                } else if start[0] == end[0] && start[1] != end[1] {
                    (nurbs_core::surface::Axis::U, start[0], [start[1], end[1]])
                } else {
                    if affine_frame(surface).is_some() || start == end {
                        return Curve::from_polyline(vec![
                            start_point.to_vec(),
                            end_point.to_vec(),
                        ]);
                    }
                    let degree = surface.degree_u + surface.degree_v;
                    if degree > 25 {
                        return Err(invalid(
                            "UV diagonal conversion requires result degree at most 25",
                        ));
                    }
                    let piece = surface.trim([
                        start[0].min(end[0]),
                        start[0].max(end[0]),
                        start[1].min(end[1]),
                        start[1].max(end[1]),
                    ])?;
                    let p = piece.degree_u;
                    let q = piece.degree_v;
                    if piece.control_points.len() != p + 1 || piece.control_points[0].len() != q + 1
                    {
                        return diagonal_multispan(surface, *plane, *start, *end);
                    }
                    // Restrict the homogeneous tensor product to u(t), v(t).
                    // B_i^p(t) B_j^q(t) = C(p,i) C(q,j) / C(p+q,i+j) B_(i+j)^(p+q)(t).
                    let binomial = |n: usize, k: usize| {
                        (0..k).fold(1_f64, |value, i| value * (n - i) as f64 / (i + 1) as f64)
                    };
                    let scale = piece.weights.iter().flatten().copied().fold(0., f64::max);
                    let mut controls = vec![[0.; 4]; degree + 1];
                    for i in 0..=p {
                        for j in 0..=q {
                            let u = if start[0] > end[0] { p - i } else { i };
                            let v = if start[1] > end[1] { q - j } else { j };
                            let factor = binomial(p, i) * binomial(q, j) / binomial(degree, i + j);
                            let weight = factor * (piece.weights[u][v] / scale);
                            for axis in 0..3 {
                                controls[i + j][axis] += weight * piece.control_points[u][v][axis];
                            }
                            controls[i + j][3] += weight;
                        }
                    }
                    let curve = Curve {
                        degree,
                        knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
                        control_points: controls
                            .iter()
                            .enumerate()
                            .map(|(index, h)| {
                                // At either end the tensor basis has exactly one
                                // nonzero term. Copy its Cartesian coefficient;
                                // a redundant multiply/divide introduces drift.
                                if index == 0 || index == degree {
                                    let last = index == degree;
                                    let u = if (start[0] > end[0]) != last { p } else { 0 };
                                    let v = if (start[1] > end[1]) != last { q } else { 0 };
                                    piece.control_points[u][v].clone()
                                } else {
                                    (0..3).map(|axis| h[axis] / h[3]).collect()
                                }
                            })
                            .collect(),
                        weights: controls.iter().map(|h| h[3]).collect(),
                        periodic: false,
                    };
                    curve.validate()?;
                    return Ok(curve);
                };
                let curve = surface
                    .iso(axis, constant)?
                    .trim(range[0].min(range[1]), range[0].max(range[1]))?;
                if range[0] > range[1] {
                    curve.reverse()
                } else {
                    Ok(curve)
                }
            }
            Self::Ruled {
                surface,
                plane,
                u_interval,
            } => {
                let degree = surface.degree_u;
                if degree > 12 || u_interval[0] == u_interval[1] {
                    return Err(invalid(
                        "Trace conversion requires nonzero interval and result degree at most 25",
                    ));
                }
                let domain = surface_domain(surface);
                let lo = u_interval[0].min(u_interval[1]);
                let hi = u_interval[0].max(u_interval[1]);
                let piece = surface.trim([lo, hi, domain[2], domain[3]])?;
                if piece.control_points.len() != degree + 1 {
                    let mut breaks: Vec<f64> = piece
                        .knots_u
                        .iter()
                        .copied()
                        .filter(|u| *u >= lo && *u <= hi)
                        .collect();
                    breaks.dedup();
                    let result_degree = 2 * degree;
                    if (breaks.len() - 1) * result_degree + 1 > 256 {
                        return Err(invalid("Converted trace exceeds 256 control points"));
                    }
                    let mut result: Option<Curve> = None;
                    for span in breaks.windows(2) {
                        let segment = Self::Ruled {
                            surface: piece.trim([span[0], span[1], domain[2], domain[3]])?,
                            plane: *plane,
                            u_interval: [span[0], span[1]],
                        }
                        .to_curve()?;
                        if let Some(joined) = &mut result {
                            // Do not weld geometrically nearby endpoints: only identical
                            // Cartesian endpoint values can share a control point here.
                            if joined.control_points.last() != segment.control_points.first() {
                                return Err(invalid(
                                    "Converted span endpoints do not coincide numerically",
                                ));
                            }
                            let factor = joined.weights.last().unwrap() / segment.weights[0];
                            joined.knots.pop();
                            joined
                                .knots
                                .extend_from_slice(&segment.knots[result_degree + 1..]);
                            joined
                                .control_points
                                .extend_from_slice(&segment.control_points[1..]);
                            joined
                                .weights
                                .extend(segment.weights[1..].iter().map(|w| w * factor));
                        } else {
                            result = Some(segment);
                        }
                    }
                    let mut curve = result.ok_or_else(|| invalid("Trace has no nonzero spans"))?;
                    let scale = curve.weights.iter().copied().fold(0., f64::max);
                    for weight in &mut curve.weights {
                        *weight /= scale;
                    }
                    curve.validate()?;
                    return if u_interval[0] > u_interval[1] {
                        curve.reverse()
                    } else {
                        Ok(curve)
                    };
                }
                let plane = plane.normalized()?;
                let scale = piece.weights.iter().flatten().copied().fold(0., f64::max);
                let homogeneous = (0..=degree)
                    .map(|i| {
                        std::array::from_fn::<_, 2, _>(|j| {
                            let w = piece.weights[i][j] / scale;
                            [
                                piece.control_points[i][j][0] * w,
                                piece.control_points[i][j][1] * w,
                                piece.control_points[i][j][2] * w,
                                w,
                            ]
                        })
                    })
                    .collect::<Vec<_>>();
                let values = homogeneous
                    .iter()
                    .map(|row| {
                        row.map(|h| dot(plane.normal, [h[0], h[1], h[2]]) - plane.offset * h[3])
                    })
                    .collect::<Vec<_>>();
                if values.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(invalid("Nonfinite ruled boundary residual"));
                }
                admit_ruled_parameter_bounds(ruled_boundary_bounds(&piece, plane))?;
                let binomial = |n: usize, k: usize| {
                    (0..k).fold(1_f64, |value, i| value * (n - i) as f64 / (i + 1) as f64)
                };
                let mut controls = vec![[0.; 4]; 2 * degree + 1];
                for i in 0..=degree {
                    for j in 0..=degree {
                        let factor =
                            binomial(degree, i) * binomial(degree, j) / binomial(2 * degree, i + j);
                        for axis in 0..4 {
                            controls[i + j][axis] += factor
                                * (homogeneous[i][1][axis] * values[j][0]
                                    - homogeneous[i][0][axis] * values[j][1]);
                        }
                    }
                }
                let sign = if controls.iter().all(|h| h[3] > 0.) {
                    1.
                } else if controls.iter().all(|h| h[3] < 0.) {
                    -1.
                } else {
                    return Err(invalid(
                        "Converted trace has no positive-weight representation in this basis",
                    ));
                };
                let scale = controls.iter().map(|h| h[3].abs()).fold(0., f64::max);
                let curve = Curve {
                    degree: 2 * degree,
                    knots: [vec![lo; 2 * degree + 1], vec![hi; 2 * degree + 1]].concat(),
                    control_points: controls
                        .iter()
                        .map(|h| (0..3).map(|i| h[i] / h[3]).collect())
                        .collect(),
                    weights: controls.iter().map(|h| sign * h[3] / scale).collect(),
                    periodic: false,
                };
                curve.validate()?;
                if u_interval[0] > u_interval[1] {
                    curve.reverse()
                } else {
                    Ok(curve)
                }
            }
        }
    }
    /// `fraction` is in [0,1]. The source surface remains authoritative.
    pub fn evaluate(&self, fraction: f64) -> Result<SurfacePoint> {
        if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
            return Err(invalid("Trace fraction must be in [0,1]"));
        }
        if let Self::RuledU {
            surface,
            plane,
            v_interval,
        } = self
        {
            surface.validate()?;
            let mut point = Self::Ruled {
                surface: transpose_surface(surface),
                plane: *plane,
                u_interval: *v_interval,
            }
            .evaluate(fraction)?;
            point.uv.swap(0, 1);
            return Ok(point);
        }
        let (surface, plane) = match self {
            Self::Line { surface, plane, .. }
            | Self::Ruled { surface, plane, .. }
            | Self::RuledU { surface, plane, .. } => (surface, *plane),
        };
        surface.validate()?;
        let plane = plane.normalized()?;
        let uv = match self {
            Self::RuledU { .. } => unreachable!("Handled above"),
            Self::Line { start, end, .. } => {
                let domain = surface_domain(surface);
                if ![start, end].iter().all(|uv| {
                    (0..2).all(|axis| {
                        uv[axis].is_finite()
                            && uv[axis] >= domain[axis * 2]
                            && uv[axis] <= domain[axis * 2 + 1]
                    })
                }) {
                    return Err(invalid(
                        "Line trace endpoints must lie in the source domain",
                    ));
                }
                std::array::from_fn(|i| start[i] + fraction * (end[i] - start[i]))
            }
            Self::Ruled {
                surface: source,
                u_interval,
                ..
            } => {
                if source.degree_v != 1 || source.control_points[0].len() != 2 {
                    return Err(invalid("Ruled trace requires a surface linear in V"));
                }
                let domain = surface_domain(source);
                if !u_interval
                    .iter()
                    .all(|u| u.is_finite() && *u >= domain[0] && *u <= domain[1])
                {
                    return Err(invalid(
                        "Ruled trace interval must lie in the source domain",
                    ));
                }
                let u = u_interval[0] + fraction * (u_interval[1] - u_interval[0]);
                let bottom = source.evaluate(u, domain[2])?.point;
                let top = source.evaluate(u, domain[3])?.point;
                // Linear interpolation occurs in homogeneous coordinates.
                // Both boundary weights must use the same normalization.
                let basis = nurbs_core::curve::basis(
                    source.degree_u,
                    &source.knots_u,
                    source.control_points.len(),
                    u,
                    source.periodic_u,
                )?;
                let scale = source.weights.iter().flatten().copied().fold(0., f64::max);
                let weights: [f64; 2] = std::array::from_fn(|column| {
                    basis
                        .basis
                        .iter()
                        .zip(&source.weights)
                        .map(|(b, w)| b * (w[column] / scale))
                        .sum()
                });
                let a = weights[0] * plane.distance(bottom);
                let b = weights[1] * plane.distance(top);
                let denominator = a - b;
                if !denominator.is_finite() || denominator == 0. {
                    return Err(invalid("Singular ruled intersection trace"));
                }
                let v = domain[2] + a / denominator * (domain[3] - domain[2]);
                if v < domain[2] || v > domain[3] {
                    return Err(invalid("Ruled trace left the source domain"));
                }
                [u, v]
            }
        };
        let point = surface.evaluate(uv[0], uv[1])?.point;
        Ok(SurfacePoint {
            uv,
            point,
            plane_residual: plane.distance(point).abs(),
        })
    }
}
#[derive(Clone, Debug)]
pub enum SurfacePlaneComponent {
    Curve {
        trace: SurfaceTrace,
        /// Parameter region supporting the branch, including root uncertainty.
        parameter_box: [f64; 4],
        samples: Vec<SurfacePoint>,
        max_sample_residual: f64,
    },
    Point(SurfacePoint),
    Overlap {
        parameter_box: [f64; 4],
        control_residual: f64,
    },
}

/// Enclose residual construction from retained trimmed Cartesian coefficients
/// and the normalized plane. Prior trim and plane normalization are not covered.
fn ruled_boundary_bounds(surface: &Surface, plane: Plane) -> Vec<[Interval; 2]> {
    let scale = surface.weights.iter().flatten().copied().fold(0., f64::max);
    surface
        .control_points
        .iter()
        .zip(&surface.weights)
        .map(|(points, weights)| {
            std::array::from_fn(|side| {
                let mut residual = Interval::exact(-plane.offset);
                for axis in 0..3 {
                    residual = residual.add(
                        Interval::exact(plane.normal[axis])
                            .mul(Interval::exact(points[side][axis])),
                    );
                }
                let weight = if weights[side] == scale {
                    Interval::exact(1.)
                } else {
                    let ratio = weights[side] / scale;
                    Interval {
                        lo: ratio.next_down().max(0.),
                        hi: ratio.next_up(),
                    }
                };
                residual.mul(weight)
            })
        })
        .collect()
}
#[cfg(test)]
fn admit_ruled_parameter_range(values: &[[f64; 2]]) -> Result<()> {
    if values.iter().flatten().any(|v| !v.is_finite()) {
        return Err(invalid("Nonfinite ruled boundary residual"));
    }
    admit_ruled_parameter_bounds(values.iter().map(|r| r.map(Interval::exact)).collect())
}
/// Bernstein bounds cover residual construction and subdivision rounding, but
/// not earlier knot edits or plane normalization. This is not a certificate.
fn admit_ruled_parameter_bounds(bounds: Vec<[Interval; 2]>) -> Result<()> {
    let mut pending = vec![(bounds, 0usize)];
    let mut visited = 0;
    while let Some((row, depth)) = pending.pop() {
        visited += 1;
        if visited > 4096 {
            return Err(Error::new(
                "BREP_INTERSECTION_UNRESOLVED",
                "Ruled parameter range admission exceeded its work budget",
            ));
        }
        let opposite = row.iter().all(|r| r[0].hi <= 0. && r[1].lo >= 0.)
            || row.iter().all(|r| r[0].lo >= 0. && r[1].hi <= 0.);
        if opposite {
            continue;
        }
        // Endpoint Bernstein coefficients enclose actual endpoint residuals.
        // Equal strict signs mean that the plane lies outside that ruling.
        for r in [row.first().unwrap(), row.last().unwrap()] {
            if r[0].sign() != 0 && r[0].sign() == r[1].sign() {
                return Err(invalid("Ruled trace leaves the source parameter domain"));
            }
        }
        if depth >= 32 {
            return Err(Error::new(
                "BREP_INTERSECTION_UNRESOLVED",
                "Ruled parameter range remains unresolved at the subdivision limit",
            ));
        }
        let mut work = row;
        let mut left = vec![work[0]];
        let mut right = vec![*work.last().unwrap()];
        while work.len() > 1 {
            work = work
                .windows(2)
                .map(|r| {
                    std::array::from_fn(|axis| r[0][axis].add(r[1][axis]).mul(Interval::exact(0.5)))
                })
                .collect();
            left.push(work[0]);
            right.push(*work.last().unwrap());
        }
        right.reverse();
        pending.push((right, depth + 1));
        pending.push((left, depth + 1));
    }
    Ok(())
}

/// Keep each ruled section span independently, avoiding any endpoint welding.
/// The source U interval is mapped to the same global fraction as evaluate().
fn ruled_segments(surface: &Surface, plane: Plane, interval: [f64; 2]) -> Result<Vec<Curve>> {
    let degree = surface.degree_u;
    if degree > 12 || interval[0] == interval[1] {
        return Err(invalid(
            "Trace conversion requires nonzero interval and result degree at most 25",
        ));
    }
    let lo = interval[0].min(interval[1]);
    let hi = interval[0].max(interval[1]);
    let domain = surface_domain(surface);
    let mut breaks = vec![lo, hi];
    breaks.extend(
        surface
            .knots_u
            .iter()
            .copied()
            .filter(|u| *u > lo && *u < hi),
    );
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    if (breaks.len() - 1) * (2 * degree + 1) > 256 {
        return Err(invalid("Converted trace exceeds 256 control points"));
    }
    if interval[0] > interval[1] {
        breaks.reverse();
    }
    let mut result = Vec::new();
    for span in breaks.windows(2) {
        let a = span[0].min(span[1]);
        let b = span[0].max(span[1]);
        let piece = surface.trim([a, b, domain[2], domain[3]])?;
        if piece.control_points.len() != degree + 1 {
            return Err(invalid(
                "Ruled knot partition does not isolate one Bezier span",
            ));
        }
        let mut curve = SurfaceTrace::Ruled {
            surface: piece,
            plane,
            u_interval: [span[0], span[1]],
        }
        .to_curve()?;
        let t0 = if span[0] == interval[0] {
            0.
        } else {
            (span[0] - interval[0]) / (interval[1] - interval[0])
        };
        let t1 = if span[1] == interval[1] {
            1.
        } else {
            (span[1] - interval[0]) / (interval[1] - interval[0])
        };
        if !t0.is_finite() || !t1.is_finite() || t0 >= t1 {
            return Err(invalid("Ruled knot crossing interval is not representable"));
        }
        // A Bezier piece has only its two clamped endpoint knots. Reverse()
        // reverses coefficients while retaining this ascending source domain.
        for knot in &mut curve.knots {
            *knot = if *knot == a { t0 } else { t1 };
        }
        curve.validate()?;
        result.push(curve);
    }
    Ok(result)
}

/// Partition an affine UV trace at source knot crossings. Every piece remains
/// a homogeneous tensor restriction, and joins require identical endpoint
/// values; tolerance welding is deliberately not part of this conversion.
fn diagonal_segments(
    surface: &Surface,
    plane: Plane,
    start: [f64; 2],
    end: [f64; 2],
) -> Result<Vec<Curve>> {
    let degree = surface.degree_u + surface.degree_v;
    let mut crossings = vec![(0., None), (1., None)];
    for (axis, knots) in [(0, &surface.knots_u), (1, &surface.knots_v)] {
        for &knot in knots {
            if knot > start[axis].min(end[axis]) && knot < start[axis].max(end[axis]) {
                let t = (knot - start[axis]) / (end[axis] - start[axis]);
                if !t.is_finite() || t <= 0. || t >= 1. {
                    return Err(invalid("UV knot crossing is not representable"));
                }
                crossings.push((t, Some((axis, knot))));
            }
        }
    }
    crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut breaks: Vec<(f64, [f64; 2], [bool; 2])> = Vec::new();
    for (t, crossing) in crossings {
        if breaks.last().is_none_or(|b| b.0 != t) {
            let uv = if t == 0. {
                start
            } else if t == 1. {
                end
            } else {
                std::array::from_fn(|axis| start[axis] + t * (end[axis] - start[axis]))
            };
            breaks.push((t, uv, [false; 2]));
        }
        if let Some((axis, knot)) = crossing {
            let b = breaks.last_mut().unwrap();
            if b.2[axis] && b.1[axis] != knot {
                return Err(invalid("Distinct UV knots collapse to one trace parameter"));
            }
            b.1[axis] = knot;
            b.2[axis] = true;
        }
    }
    if (breaks.len() - 1) * (degree + 1) > 256 {
        return Err(invalid("Converted trace exceeds 256 control points"));
    }
    let mut result = Vec::new();
    for pair in breaks.windows(2) {
        let (t0, a, _) = pair[0];
        let (t1, b, _) = pair[1];
        if a[0] == b[0] || a[1] == b[1] {
            return Err(invalid("UV knot crossing interval is not representable"));
        }
        let piece = surface.trim([
            a[0].min(b[0]),
            a[0].max(b[0]),
            a[1].min(b[1]),
            a[1].max(b[1]),
        ])?;
        if piece.control_points.len() != piece.degree_u + 1
            || piece.control_points[0].len() != piece.degree_v + 1
        {
            return Err(invalid(
                "UV knot partition does not isolate one tensor Bezier span",
            ));
        }
        let mut segment = SurfaceTrace::Line {
            surface: piece,
            plane,
            start: a,
            end: b,
        }
        .to_curve()?
        .elevate(degree)?;
        // Single-span conversion is parameterized on [0,1]. Preserve the
        // original trace fraction globally, including nonuniform crossings.
        for knot in &mut segment.knots {
            *knot = if *knot == 0. { t0 } else { t1 };
        }
        segment.validate()?;
        result.push(segment);
    }
    Ok(result)
}
fn diagonal_multispan(
    surface: &Surface,
    plane: Plane,
    start: [f64; 2],
    end: [f64; 2],
) -> Result<Curve> {
    let degree = surface.degree_u + surface.degree_v;
    let mut result: Option<Curve> = None;
    for segment in diagonal_segments(surface, plane, start, end)? {
        if let Some(joined) = &mut result {
            if joined.control_points.last() != segment.control_points.first() {
                return Err(invalid(
                    "Converted span endpoints do not coincide numerically",
                ));
            }
            let factor = joined.weights.last().unwrap() / segment.weights[0];
            joined.knots.pop();
            joined.knots.extend_from_slice(&segment.knots[degree + 1..]);
            joined
                .control_points
                .extend_from_slice(&segment.control_points[1..]);
            joined
                .weights
                .extend(segment.weights[1..].iter().map(|w| w * factor));
        } else {
            result = Some(segment);
        }
    }
    let mut curve = result.ok_or_else(|| invalid("Trace has no nonzero spans"))?;
    let scale = curve.weights.iter().copied().fold(0., f64::max);
    for weight in &mut curve.weights {
        *weight /= scale;
    }
    curve.validate()?;
    Ok(curve)
}

fn transpose_surface(s: &Surface) -> Surface {
    Surface {
        degree_u: s.degree_v,
        degree_v: s.degree_u,
        knots_u: s.knots_v.clone(),
        knots_v: s.knots_u.clone(),
        control_points: (0..s.control_points[0].len())
            .map(|v| s.control_points.iter().map(|row| row[v].clone()).collect())
            .collect(),
        weights: (0..s.weights[0].len())
            .map(|v| s.weights.iter().map(|row| row[v]).collect())
            .collect(),
        periodic_u: s.periodic_v,
        periodic_v: s.periodic_u,
    }
}
fn swap_trace(trace: SurfaceTrace) -> SurfaceTrace {
    match trace {
        SurfaceTrace::Line {
            surface,
            plane,
            start,
            end,
        } => SurfaceTrace::Line {
            surface: transpose_surface(&surface),
            plane,
            start: [start[1], start[0]],
            end: [end[1], end[0]],
        },
        SurfaceTrace::Ruled {
            surface,
            plane,
            u_interval,
        } => SurfaceTrace::RuledU {
            surface: transpose_surface(&surface),
            plane,
            v_interval: u_interval,
        },
        SurfaceTrace::RuledU {
            surface,
            plane,
            v_interval,
        } => SurfaceTrace::Ruled {
            surface: transpose_surface(&surface),
            plane,
            u_interval: v_interval,
        },
    }
}

fn surface_domain(surface: &Surface) -> [f64; 4] {
    [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.control_points[0].len()],
    ]
}

#[derive(Clone, Copy)]
struct AffineFrame {
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    domain: [f64; 4],
}
fn affine_frame(surface: &Surface) -> Option<AffineFrame> {
    if surface.degree_u != 1
        || surface.degree_v != 1
        || surface.control_points.len() != 2
        || surface.control_points[0].len() != 2
        || surface.periodic_u
        || surface.periodic_v
        || !surface
            .weights
            .iter()
            .flatten()
            .all(|w| *w == surface.weights[0][0])
    {
        return None;
    }
    let p = &surface.control_points;
    let origin = point3(&p[0][0]);
    let u = sub(point3(&p[1][0]), origin);
    let v = sub(point3(&p[0][1]), origin);
    if (0..3).any(|i| p[1][1][i] != origin[i] + u[i] + v[i]) {
        return None;
    }
    let n = cross(u, v);
    if dot(n, n) <= f64::MIN_POSITIVE {
        return None;
    }
    Some(AffineFrame {
        origin,
        u,
        v,
        domain: surface_domain(surface),
    })
}

fn add_trace(
    report: &mut Report<SurfacePlaneComponent>,
    parameter_box: [f64; 4],
    trace: SurfaceTrace,
    tolerance: f64,
) -> Result<()> {
    let samples = (0..=8)
        .map(|i| trace.evaluate(i as f64 / 8.))
        .collect::<Result<Vec<_>>>()?;
    let max_sample_residual = samples.iter().map(|p| p.plane_residual).fold(0., f64::max);
    if max_sample_residual > tolerance {
        report.unresolved(parameter_box, UnresolvedReason::NearCoincidence);
    } else {
        report.components.push(SurfacePlaneComponent::Curve {
            trace,
            parameter_box,
            samples,
            max_sample_residual,
        });
    }
    Ok(())
}

/// Plane sections of untrimmed NURBS patches. Affine planes and rational ruled
/// patches linear in V retain procedural parameter traces. General patches may
/// be excluded by Bernstein control bounds; unresolved regions are explicit.
/// Face trims, branch sewing, tangency certification and arbitrary SS are later
/// stages: callers must not infer a trimmed B-rep section from this report alone.
pub fn surface_plane(
    surface: &Surface,
    plane: Plane,
    options: Options,
) -> Result<Report<SurfacePlaneComponent>> {
    surface.validate()?;
    let plane = plane.normalized()?;
    let options = options.validate()?;
    let linear_u = surface.degree_u == 1;
    let linear_v = surface.degree_v == 1;
    if linear_u && !linear_v {
        let mut report = surface_plane(&transpose_surface(surface), plane, options)?;
        let swap_box = |b: &mut [f64]| {
            b.swap(0, 2);
            b.swap(1, 3);
        };
        for pending in &mut report.unresolved {
            swap_box(&mut pending.parameter_box);
        }
        for component in &mut report.components {
            match component {
                SurfacePlaneComponent::Curve {
                    trace,
                    parameter_box,
                    samples,
                    ..
                } => {
                    *trace = swap_trace(trace.clone());
                    swap_box(parameter_box);
                    for sample in samples {
                        sample.uv.swap(0, 1);
                    }
                }
                SurfacePlaneComponent::Point(point) => point.uv.swap(0, 1),
                SurfacePlaneComponent::Overlap { parameter_box, .. } => swap_box(parameter_box),
            }
        }
        return Ok(report);
    }
    let mut report = Report::default();
    if let Some(frame) = affine_frame(surface) {
        report.boxes_visited = 1;
        let domain = frame.domain;
        let uv = [
            [domain[0], domain[2]],
            [domain[1], domain[2]],
            [domain[1], domain[3]],
            [domain[0], domain[3]],
        ];
        let points = uv
            .map(|p| surface.evaluate(p[0], p[1]).map(|j| j.point))
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let distances: Vec<_> = points.iter().map(|p| plane.distance(*p)).collect();
        if distances.iter().all(|d| *d == 0.) {
            report.components.push(SurfacePlaneComponent::Overlap {
                parameter_box: domain,
                control_residual: 0.,
            });
            return Ok(report);
        }
        let mut hits = Vec::new();
        for i in 0..4 {
            let next = (i + 1) % 4;
            if distances[i] == 0. {
                hits.push(uv[i]);
            }
            if distances[i] * distances[next] < 0. {
                let t = distances[i] / (distances[i] - distances[next]);
                hits.push(std::array::from_fn(|d| {
                    uv[i][d] + t * (uv[next][d] - uv[i][d])
                }));
            }
        }
        hits.dedup();
        match hits.len() {
            0 => {
                report.bernstein_excluded = 1;
            }
            1 => {
                let point = surface.evaluate(hits[0][0], hits[0][1])?.point;
                report
                    .components
                    .push(SurfacePlaneComponent::Point(SurfacePoint {
                        uv: hits[0],
                        point,
                        plane_residual: plane.distance(point).abs(),
                    }));
            }
            2 => add_trace(
                &mut report,
                domain,
                SurfaceTrace::Line {
                    surface: surface.clone(),
                    plane,
                    start: hits[0],
                    end: hits[1],
                },
                options.distance_tolerance,
            )?,
            _ => report.unresolved(domain, UnresolvedReason::NearCoincidence),
        }
        return Ok(report);
    }
    let mut pending = std::collections::VecDeque::new();
    for u in spans(
        &surface.knots_u,
        surface.degree_u,
        surface.control_points.len(),
    ) {
        for v in spans(
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
        ) {
            pending.push_back(([u[0], u[1], v[0], v[1]], 0usize));
        }
    }
    while let Some((domain, depth)) = pending.pop_front() {
        if report.boxes_visited >= options.max_boxes || report.components.len() >= 1024 {
            report.unresolved(domain, UnresolvedReason::BudgetExceeded);
            continue;
        }
        report.boxes_visited += 1;
        let piece = surface.trim(domain)?;
        let values: Vec<_> = piece
            .control_points
            .iter()
            .zip(&piece.weights)
            .flat_map(|(p, w)| {
                let c = Curve {
                    degree: 1,
                    knots: vec![],
                    control_points: p.clone(),
                    weights: w.clone(),
                    periodic: false,
                };
                coefficients(&c, plane)
            })
            .collect();
        if excluded(&values) {
            report.bernstein_excluded += 1;
            continue;
        }
        if values.iter().all(|c| c.value == 0.) {
            report.components.push(SurfacePlaneComponent::Overlap {
                parameter_box: domain,
                control_residual: 0.,
            });
            continue;
        }
        let ruled = piece.degree_v == 1 && piece.control_points[0].len() == 2;
        if !ruled {
            report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
            continue;
        }
        let boundary_scale = piece.weights[0][1] / piece.weights[0][0];
        let proportional_boundaries = boundary_scale.is_finite()
            && boundary_scale > 0.
            && piece.weights.iter().all(|w| w[1] / w[0] == boundary_scale)
            && piece
                .control_points
                .iter()
                .all(|row| plane.distance(point3(&row[0])) == plane.distance(point3(&row[1])));
        let equal_bounds = values.chunks_exact(2).all(|pair| pair[0].value == pair[1].value);
        if equal_bounds || proportional_boundaries {
            // A plane parallel to every ruling reduces to a boundary-curve
            // root query. Each root carries its parameter interval through to
            // a whole generator line; no sampled 3D curve fitting is needed.
            if report.boxes_visited == options.max_boxes {
                report.unresolved(domain, UnresolvedReason::BudgetExceeded);
                continue;
            }
            let boundary = piece.iso(nurbs_core::surface::Axis::V, domain[2])?;
            let roots = curve_plane(
                &boundary,
                plane,
                Options {
                    max_boxes: options.max_boxes - report.boxes_visited,
                    ..options
                },
            )?;
            report.boxes_visited += roots.boxes_visited;
            report.bernstein_excluded += roots.bernstein_excluded;
            for pending in roots.unresolved {
                report.unresolved(
                    [
                        pending.parameter_box[0],
                        pending.parameter_box[1],
                        domain[2],
                        domain[3],
                    ],
                    pending.reason,
                );
            }
            for component in roots.components {
                match component {
                    CurvePlaneComponent::Point(root) => {
                        add_trace(
                            &mut report,
                            [
                                root.parameter_interval[0],
                                root.parameter_interval[1],
                                domain[2],
                                domain[3],
                            ],
                            SurfaceTrace::Line {
                                surface: piece.clone(),
                                plane,
                                start: [root.parameter, domain[2]],
                                end: [root.parameter, domain[3]],
                            },
                            options.distance_tolerance,
                        )?;
                    }
                    CurvePlaneComponent::Overlap {
                        parameter_interval,
                        control_residual,
                    } => {
                        report.components.push(SurfacePlaneComponent::Overlap {
                            parameter_box: [
                                parameter_interval[0],
                                parameter_interval[1],
                                domain[2],
                                domain[3],
                            ],
                            control_residual,
                        });
                    }
                }
            }
            continue;
        }
        let side_step = |offset: usize| {
            let mut it = values.iter().skip(offset).step_by(2);
            let Some(first) = it.next() else { return 0; };
            let s = first.bound.sign();
            if s != 0 && it.all(|c| c.bound.sign() == s) {
                s
            } else {
                0
            }
        };
        let lower_side = side_step(0);
        let upper_side = side_step(1);
        let lower_zero = values.iter().step_by(2).all(|c| c.value == 0.);
        let upper_zero = values.iter().skip(1).step_by(2).all(|c| c.value == 0.);
        if (lower_zero && upper_side != 0) || (upper_zero && lower_side != 0) {
            let v = if lower_zero { domain[2] } else { domain[3] };
            // A continuous shared knot is owned by the preceding V span.
            // Full-multiplicity discontinuities retain both independent sides.
            let shared_lower = lower_zero
                && v > surface_domain(surface)[2]
                && surface.knots_v.iter().filter(|k| **k == v).count() <= surface.degree_v;
            if !shared_lower {
                add_trace(
                    &mut report,
                    [domain[0], domain[1], v, v],
                    SurfaceTrace::Line {
                        surface: piece,
                        plane,
                        start: [domain[0], v],
                        end: [domain[1], v],
                    },
                    options.distance_tolerance,
                )?;
            }
            continue;
        }
        if lower_side * upper_side == -1 {
            // Full cross-ruling branch: the denominator cannot vanish because
            // the two boundary curves are uniformly on opposite plane sides.
            let trace = SurfaceTrace::Ruled {
                surface: piece,
                plane,
                u_interval: [domain[0], domain[1]],
            };
            add_trace(&mut report, domain, trace, options.distance_tolerance)?;
            continue;
        }
        let width = domain[1] - domain[0];
        let midpoint = domain[0] + width * 0.5;
        if width <= options.parameter_tolerance
            || depth == options.max_depth
            || midpoint == domain[0]
            || midpoint == domain[1]
        {
            report.unresolved(domain, UnresolvedReason::BoundaryCrossing);
            continue;
        }
        pending.push_back(([midpoint, domain[1], domain[2], domain[3]], depth + 1));
        pending.push_back(([domain[0], midpoint, domain[2], domain[3]], depth + 1));
    }
    Ok(report)
}

#[derive(Clone, Debug)]
pub enum CurveSurfaceComponent {
    Point {
        curve: CurvePoint,
        uv: [f64; 2],
        surface_residual: f64,
    },
    Overlap {
        curve_interval: [f64; 2],
    },
}

/// Curve/surface entry point currently admits affine rectangular planar patches.
/// Curved support surfaces return an explicit unsupported parameter region.
/// Coplanar curves are admitted only if their entire control hull lies inside
/// the rectangle. Partial coincident clipping is explicitly unresolved.
pub fn curve_surface(
    curve: &Curve,
    surface: &Surface,
    options: Options,
) -> Result<Report<CurveSurfaceComponent>> {
    curve.validate()?;
    if curve.control_points[0].len() != 3 {
        return Err(invalid("Curve/surface requires a 3D curve"));
    }
    surface.validate()?;
    let options = options.validate()?;
    let Some(frame) = affine_frame(surface) else {
        let mut report = Report::default();
        report.unresolved(curve.domain(), UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    let normal = cross(frame.u, frame.v);
    let plane = Plane {
        normal,
        offset: dot(normal, frame.origin),
    };
    let source = curve_plane(curve, plane, options)?;
    let mut report = Report {
        components: vec![],
        unresolved: source.unresolved,
        boxes_visited: source.boxes_visited,
        bernstein_excluded: source.bernstein_excluded,
        coverage: source.coverage,
    };
    let a = dot(frame.u, frame.u);
    let b = dot(frame.u, frame.v);
    let c = dot(frame.v, frame.v);
    let determinant = a * c - b * b;
    if determinant <= 1e-24 * a * c {
        report.unresolved(curve.domain(), UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    }
    let project = |point: [f64; 3]| {
        let d = sub(point, frame.origin);
        let x = dot(d, frame.u);
        let y = dot(d, frame.v);
        let u = (x * c - y * b) / determinant;
        let v = (y * a - x * b) / determinant;
        [
            frame.domain[0] + u * (frame.domain[1] - frame.domain[0]),
            frame.domain[2] + v * (frame.domain[3] - frame.domain[2]),
        ]
    };
    let inside = |uv: [f64; 2]| {
        uv[0] >= frame.domain[0]
            && uv[0] <= frame.domain[1]
            && uv[1] >= frame.domain[2]
            && uv[1] <= frame.domain[3]
    };
    for component in source.components {
        match component {
            CurvePlaneComponent::Point(point) => {
                let uv = project(point.point);
                if point.parameter_interval[0] < point.parameter_interval[1] {
                    let interval_curve =
                        curve.trim(point.parameter_interval[0], point.parameter_interval[1])?;
                    let hull: Vec<_> = interval_curve
                        .control_points
                        .iter()
                        .map(|p| project(point3(p)))
                        .collect();
                    let outside = (0..2).any(|axis| {
                        hull.iter().all(|p| p[axis] < frame.domain[2 * axis])
                            || hull.iter().all(|p| p[axis] > frame.domain[2 * axis + 1])
                    });
                    if outside {
                        continue;
                    }
                    if !hull.iter().all(|p| inside(*p)) {
                        report.unresolved(
                            point.parameter_interval,
                            UnresolvedReason::BoundaryCrossing,
                        );
                        continue;
                    }
                }
                if inside(uv) {
                    let on_surface = surface.evaluate(uv[0], uv[1])?.point;
                    let delta = sub(on_surface, point.point);
                    let residual = dot(delta, delta).sqrt();
                    if residual <= options.distance_tolerance {
                        report.components.push(CurveSurfaceComponent::Point {
                            curve: point,
                            uv,
                            surface_residual: residual,
                        });
                    } else {
                        report.unresolved(
                            point.parameter_interval,
                            UnresolvedReason::NearCoincidence,
                        );
                    }
                }
            }
            CurvePlaneComponent::Overlap {
                parameter_interval, ..
            } => {
                let piece = curve.trim(parameter_interval[0], parameter_interval[1])?;
                if piece
                    .control_points
                    .iter()
                    .all(|p| inside(project(point3(p))))
                {
                    report.components.push(CurveSurfaceComponent::Overlap {
                        curve_interval: parameter_interval,
                    });
                } else {
                    let dual_u =
                        std::array::from_fn(|i| (c * frame.u[i] - b * frame.v[i]) / determinant);
                    let dual_v =
                        std::array::from_fn(|i| (a * frame.v[i] - b * frame.u[i]) / determinant);
                    let mut cuts = vec![parameter_interval[0], parameter_interval[1]];
                    let mut bands = Vec::new();
                    let mut endpoints = Vec::new();
                    for normal in [dual_u, dual_v] {
                        for boundary in [0., 1.] {
                            if report.boxes_visited >= options.max_boxes {
                                bands.push(parameter_interval);
                                report.unresolved(
                                    parameter_interval,
                                    UnresolvedReason::BudgetExceeded,
                                );
                                continue;
                            }
                            let roots = curve_plane(
                                &piece,
                                Plane {
                                    normal,
                                    offset: dot(normal, frame.origin) + boundary,
                                },
                                Options {
                                    max_boxes: options.max_boxes - report.boxes_visited,
                                    ..options
                                },
                            )?;
                            report.boxes_visited += roots.boxes_visited;
                            report.bernstein_excluded += roots.bernstein_excluded;
                            for unresolved in roots.unresolved {
                                let band =
                                    [unresolved.parameter_box[0], unresolved.parameter_box[1]];
                                cuts.extend(band);
                                bands.push(band);
                                report.unresolved(band, unresolved.reason);
                            }
                            for root in roots.components {
                                match root {
                                    CurvePlaneComponent::Point(point) => {
                                        cuts.extend(point.parameter_interval);
                                        if point.parameter_interval[0]
                                            == point.parameter_interval[1]
                                        {
                                            endpoints.push(point);
                                        } else {
                                            bands.push(point.parameter_interval);
                                            report.unresolved(
                                                point.parameter_interval,
                                                UnresolvedReason::BoundaryCrossing,
                                            );
                                        }
                                    }
                                    // A curve along a rectangle edge is admissible;
                                    // the other three boundary constraints still apply.
                                    CurvePlaneComponent::Overlap {
                                        parameter_interval, ..
                                    } => cuts.extend(parameter_interval),
                                }
                            }
                        }
                    }
                    cuts.sort_by(f64::total_cmp);
                    cuts.dedup();
                    for cell in cuts.windows(2) {
                        if cell[0] == cell[1]
                            || bands.iter().any(|b| cell[0] >= b[0] && cell[1] <= b[1])
                        {
                            continue;
                        }
                        if report.boxes_visited >= options.max_boxes {
                            report.unresolved(cell.to_vec(), UnresolvedReason::BudgetExceeded);
                            continue;
                        }
                        report.boxes_visited += 1;
                        let cell_curve = curve.trim(cell[0], cell[1])?;
                        let mut hull = cell_curve
                            .control_points
                            .iter()
                            .map(|p| project(point3(p)))
                            .collect::<Vec<_>>();
                        hull[0] = project(point3(&curve.evaluate(cell[0])?.point));
                        let last = hull.len() - 1;
                        hull[last] = project(point3(&curve.evaluate(cell[1])?.point));
                        if hull.iter().all(|p| inside(*p)) {
                            report.components.push(CurveSurfaceComponent::Overlap {
                                curve_interval: [cell[0], cell[1]],
                            });
                        } else if !(0..2).any(|axis| {
                            (hull.iter().all(|p| p[axis] <= frame.domain[2 * axis])
                                && hull.iter().any(|p| p[axis] < frame.domain[2 * axis]))
                                || (hull.iter().all(|p| p[axis] >= frame.domain[2 * axis + 1])
                                    && hull.iter().any(|p| p[axis] > frame.domain[2 * axis + 1]))
                        }) {
                            report.unresolved(cell.to_vec(), UnresolvedReason::CoincidentTrim);
                        }
                    }
                    for point in endpoints {
                        if report.components.iter().any(|c| match c {
                            CurveSurfaceComponent::Point { curve, .. } => {
                                curve.parameter == point.parameter
                            }
                            CurveSurfaceComponent::Overlap { curve_interval } => {
                                point.parameter >= curve_interval[0]
                                    && point.parameter <= curve_interval[1]
                            }
                        }) {
                            continue;
                        }
                        let uv = project(point.point);
                        if !inside(uv) {
                            continue;
                        }
                        let actual = surface.evaluate(uv[0], uv[1])?.point;
                        let d = sub(actual, point.point);
                        let residual = d[0].hypot(d[1]).hypot(d[2]);
                        if residual <= options.distance_tolerance {
                            report.components.push(CurveSurfaceComponent::Point {
                                curve: point,
                                uv,
                                surface_residual: residual,
                            });
                        } else {
                            report.unresolved(
                                point.parameter_interval,
                                UnresolvedReason::NearCoincidence,
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(report)
}

#[derive(Clone, Debug)]
pub enum SurfaceSurfaceComponent {
    /// Implicitly closed convex boundary, ordered counterclockwise in first UV.
    Overlap {
        first_boundary: Vec<[f64; 2]>,
        second_boundary: Vec<[f64; 2]>,
        points: Vec<[f64; 3]>,
        max_sample_residual: f64,
    },
    Point {
        first: SurfacePoint,
        second: SurfacePoint,
        residual: f64,
    },
    /// Both retained traces use the same fraction in [0,1].
    Curve {
        first: Box<SurfaceTrace>,
        second: Box<SurfaceTrace>,
        max_sample_residual: f64,
    },
}

/// Finite affine-patch intersection with retained UV correspondence. General
/// curved pairs remain explicitly unresolved; coplanar affine overlap retains paired UV polygons.
pub fn surface_surface(
    first: &Surface,
    second: &Surface,
    options: Options,
) -> Result<Report<SurfaceSurfaceComponent>> {
    first.validate()?;
    second.validate()?;
    let options = options.validate()?;
    let mut report = Report::default();
    let domain = [surface_domain(first), surface_domain(second)].concat();
    let (Some(a), Some(b)) = (affine_frame(first), affine_frame(second)) else {
        // G6 narrow bicubic path (Complete-empty or Incomplete); never topology change.
        match crate::nurbs_ss_g6::narrow_transverse_bicubic(first, second, nurbs_intersect::Options { distance_tolerance:options.distance_tolerance,parameter_tolerance:options.parameter_tolerance,max_depth:options.max_depth,max_boxes:options.max_boxes }) {
            Ok(g6) => {
                let mut report = Report {
                    coverage: match g6.coverage {nurbs_intersect::Coverage::Complete=>Coverage::Complete,nurbs_intersect::Coverage::NumericallyResolved=>Coverage::NumericallyResolved,nurbs_intersect::Coverage::Incomplete=>Coverage::Incomplete},
                    boxes_visited: g6.boxes_visited,
                    bernstein_excluded: g6.bernstein_excluded,
                    ..Report::default()
                };
                for pending in g6.unresolved {
                    report.unresolved(pending.parameter_box, match pending.reason {nurbs_intersect::UnresolvedReason::BudgetExceeded=>UnresolvedReason::BudgetExceeded,nurbs_intersect::UnresolvedReason::TangencyOrMultipleRoot=>UnresolvedReason::TangencyOrMultipleRoot,nurbs_intersect::UnresolvedReason::NearCoincidence=>UnresolvedReason::NearCoincidence,nurbs_intersect::UnresolvedReason::BoundaryCrossing=>UnresolvedReason::BoundaryCrossing,nurbs_intersect::UnresolvedReason::UnsupportedSurface=>UnresolvedReason::UnsupportedSurface,nurbs_intersect::UnresolvedReason::CoincidentTrim=>UnresolvedReason::CoincidentTrim});
                }
                // Curve Complete stays inside nurbs_ss_g6; surface_surface only
                // forwards empty Complete / typed Incomplete without fake UV traces.
                return Ok(report);
            }
            Err(_) => {
                report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
                return Ok(report);
            }
        }
    };
    let normal = cross(b.u, b.v);
    let plane_b = Plane {
        normal,
        offset: dot(normal, b.origin),
    }
    .normalized()?;
    let normal = cross(a.u, a.v);
    let plane_a = Plane {
        normal,
        offset: dot(normal, a.origin),
    }
    .normalized()?;
    let source = surface_plane(first, plane_b, options)?;
    report.boxes_visited = source.boxes_visited;
    report.bernstein_excluded = source.bernstein_excluded;
    for pending in source.unresolved {
        report.unresolved(
            [pending.parameter_box, surface_domain(second).to_vec()].concat(),
            pending.reason,
        );
    }
    let uu = dot(b.u, b.u);
    let uv = dot(b.u, b.v);
    let vv = dot(b.v, b.v);
    let determinant = uu * vv - uv * uv;
    if !determinant.is_finite() || determinant <= 1e-24 * uu * vv {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    }
    let project = |p| {
        let d = sub(p, b.origin);
        let x = dot(d, b.u);
        let y = dot(d, b.v);
        [
            b.domain[0] + (x * vv - y * uv) / determinant * (b.domain[1] - b.domain[0]),
            b.domain[2] + (y * uu - x * uv) / determinant * (b.domain[3] - b.domain[2]),
        ]
    };
    for component in source.components {
        if report.boxes_visited >= options.max_boxes {
            report.unresolved(domain.clone(), UnresolvedReason::BudgetExceeded);
            continue;
        }
        report.boxes_visited += 1;
        let (start, end) = match component {
            SurfacePlaneComponent::Curve {
                trace: SurfaceTrace::Line { start, end, .. },
                ..
            } => (start, end),
            SurfacePlaneComponent::Point(point) => (point.uv, point.uv),
            SurfacePlaneComponent::Overlap { .. } => {
                let corners = [
                    [a.domain[0], a.domain[2]],
                    [a.domain[1], a.domain[2]],
                    [a.domain[1], a.domain[3]],
                    [a.domain[0], a.domain[3]],
                ];
                let mut polygon = corners
                    .into_iter()
                    .map(|uv| first.evaluate(uv[0], uv[1]).map(|p| (uv, project(p.point))))
                    .collect::<Result<Vec<_>>>()?;
                let mut failed = false;
                for (axis, boundary, greater) in [
                    (0, b.domain[0], true),
                    (0, b.domain[1], false),
                    (1, b.domain[2], true),
                    (1, b.domain[3], false),
                ] {
                    if polygon.is_empty() {
                        break;
                    }
                    if report.boxes_visited >= options.max_boxes {
                        failed = true;
                        break;
                    }
                    report.boxes_visited += 1;
                    let inside = |p: ([f64; 2], [f64; 2])| {
                        if greater {
                            p.1[axis] >= boundary
                        } else {
                            p.1[axis] <= boundary
                        }
                    };
                    let mut clipped = Vec::new();
                    for i in 0..polygon.len() {
                        let previous = polygon[(i + polygon.len() - 1) % polygon.len()];
                        let current = polygon[i];
                        let before = inside(previous);
                        let after = inside(current);
                        if before != after {
                            let t = (boundary - previous.1[axis])
                                / (current.1[axis] - previous.1[axis]);
                            if !t.is_finite() || !(0. ..=1.).contains(&t) {
                                failed = true;
                                break;
                            }
                            let first_uv = std::array::from_fn(|j| {
                                previous.0[j] + t * (current.0[j] - previous.0[j])
                            });
                            let mut second_uv = std::array::from_fn(|j| {
                                previous.1[j] + t * (current.1[j] - previous.1[j])
                            });
                            second_uv[axis] = boundary;
                            clipped.push((first_uv, second_uv));
                        }
                        if after {
                            clipped.push(current);
                        }
                    }
                    if failed {
                        break;
                    }
                    clipped.dedup_by(|a, b| a.0 == b.0);
                    if clipped.len() > 1 && clipped.first().unwrap().0 == clipped.last().unwrap().0
                    {
                        clipped.pop();
                    }
                    polygon = clipped;
                }
                if failed {
                    report.unresolved(
                        domain.clone(),
                        if report.boxes_visited >= options.max_boxes {
                            UnresolvedReason::BudgetExceeded
                        } else {
                            UnresolvedReason::NearCoincidence
                        },
                    );
                    continue;
                }
                if polygon.is_empty() {
                    continue;
                }
                if polygon.len() >= 3 {
                    let origin = polygon[0].0;
                    let area = polygon
                        .windows(2)
                        .skip(1)
                        .map(|p| {
                            let x = [p[0].0[0] - origin[0], p[0].0[1] - origin[1]];
                            let y = [p[1].0[0] - origin[0], p[1].0[1] - origin[1]];
                            x[0] * y[1] - x[1] * y[0]
                        })
                        .sum::<f64>();
                    if !area.is_finite() || area <= 0. {
                        report.unresolved(domain.clone(), UnresolvedReason::NearCoincidence);
                        continue;
                    }
                    let mut points = Vec::new();
                    let mut residual = 0_f64;
                    for (u, v) in &polygon {
                        let valid = |p: [f64; 2], d: [f64; 4]| {
                            (0..2).all(|i| {
                                p[i].is_finite() && p[i] >= d[2 * i] && p[i] <= d[2 * i + 1]
                            })
                        };
                        if !valid(*u, a.domain) || !valid(*v, b.domain) {
                            residual = f64::INFINITY;
                            break;
                        }
                        let p = first.evaluate(u[0], u[1])?.point;
                        let q = second.evaluate(v[0], v[1])?.point;
                        let d = sub(p, q);
                        residual = residual.max(d[0].hypot(d[1]).hypot(d[2]));
                        points.push(p);
                    }
                    if residual > options.distance_tolerance {
                        report.unresolved(domain.clone(), UnresolvedReason::NearCoincidence);
                        continue;
                    }
                    report.components.push(SurfaceSurfaceComponent::Overlap {
                        first_boundary: polygon.iter().map(|p| p.0).collect(),
                        second_boundary: polygon.iter().map(|p| p.1).collect(),
                        points,
                        max_sample_residual: residual,
                    });
                    continue;
                }
                (polygon[0].0, polygon.last().unwrap().0)
            }
            _ => {
                report.unresolved(domain.clone(), UnresolvedReason::UnsupportedSurface);
                continue;
            }
        };
        let p = first.evaluate(start[0], start[1])?.point;
        let q = first.evaluate(end[0], end[1])?.point;
        let u = project(p);
        let v = project(q);
        let mut low = 0_f64;
        let mut high = 1_f64;
        let mut invalid_numeric = false;
        for axis in 0..2 {
            let delta = v[axis] - u[axis];
            let min = b.domain[2 * axis];
            let max = b.domain[2 * axis + 1];
            if !delta.is_finite() || !u[axis].is_finite() {
                invalid_numeric = true;
                break;
            }
            if delta == 0. {
                if u[axis] < min || u[axis] > max {
                    high = -1.;
                    break;
                }
            } else {
                let x = (min - u[axis]) / delta;
                let y = (max - u[axis]) / delta;
                if !x.is_finite() || !y.is_finite() {
                    invalid_numeric = true;
                    break;
                }
                low = low.max(x.min(y));
                high = high.min(x.max(y));
            }
        }
        if invalid_numeric {
            report.unresolved(domain.clone(), UnresolvedReason::NearCoincidence);
            continue;
        }
        if low > high {
            continue;
        }
        let interpolate =
            |x: [f64; 2], y: [f64; 2], t: f64| std::array::from_fn(|i| x[i] + t * (y[i] - x[i]));
        let first_trace = SurfaceTrace::Line {
            surface: first.clone(),
            plane: plane_b,
            start: interpolate(start, end, low),
            end: interpolate(start, end, high),
        };
        let second_trace = SurfaceTrace::Line {
            surface: second.clone(),
            plane: plane_a,
            start: interpolate(u, v, low),
            end: interpolate(u, v, high),
        };
        let mut max_residual = 0_f64;
        let mut sample = None;
        for i in 0..=8 {
            let (Ok(a), Ok(b)) = (
                first_trace.evaluate(i as f64 / 8.),
                second_trace.evaluate(i as f64 / 8.),
            ) else {
                max_residual = f64::INFINITY;
                break;
            };
            let d = sub(a.point, b.point);
            let residual = d[0].hypot(d[1]).hypot(d[2]);
            max_residual = max_residual
                .max(residual)
                .max(a.plane_residual)
                .max(b.plane_residual);
            sample = Some((a, b, residual));
        }
        if max_residual > options.distance_tolerance {
            report.unresolved(domain.clone(), UnresolvedReason::NearCoincidence);
            continue;
        }
        if low == high || start == end {
            let (first, second, residual) = sample.unwrap();
            report.components.push(SurfaceSurfaceComponent::Point {
                first,
                second,
                residual,
            });
        } else {
            report.components.push(SurfaceSurfaceComponent::Curve {
                first: Box::new(first_trace),
                second: Box::new(second_trace),
                max_sample_residual: max_residual,
            });
        }
    }
    if report.unresolved.is_empty() {
        // Affine×affine pairs above are algebraically partitioned; promote only
        // when every retained contact survived residual checks with no bands left.
        report.coverage = Coverage::Complete;
    }
    Ok(report)
}

fn invalid(message: &str) -> Error {
    Error::new("BREP_INTERSECTION_INVALID_INPUT", message)
}

/// Numerical curve/finite-segment correspondence. Overlaps retain the original
/// curve interval; degree-one coincidences are clipped in source parameters.
/// Higher-degree clipping isolates segment-boundary roots and retains unresolved bands.
#[derive(Clone, Debug)]
pub enum CurveSegmentComponent {
    Point {
        curve: CurvePoint,
        segment_parameter: f64,
        line_residual: f64,
    },
    Overlap {
        curve_interval: [f64; 2],
    },
}

/// Intersect a retained NURBS curve with a finite 3D segment using two supporting
/// planes. Both plane queries share max_boxes; results remain uncertified and
/// cannot authorize topology changes. No sampled polyline defines the curve.
pub fn curve_segment(
    curve: &Curve,
    start: [f64; 3],
    end: [f64; 3],
    options: Options,
) -> Result<Report<CurveSegmentComponent>> {
    curve.validate()?;
    let options = options.validate()?;
    if curve.control_points[0].len() != 3 || !start.iter().chain(&end).all(|x| x.is_finite()) {
        return Err(invalid("Curve/segment requires finite 3D inputs"));
    }
    let delta = sub(end, start);
    let length = delta[0].hypot(delta[1]).hypot(delta[2]);
    if !length.is_finite() || length == 0. {
        return Err(invalid("Segment must have finite nonzero length"));
    }
    let direction = delta.map(|x| x / length);
    let axis = (0..3)
        .min_by(|a, b| direction[*a].abs().total_cmp(&direction[*b].abs()))
        .unwrap();
    let mut basis = [0.; 3];
    basis[axis] = 1.;
    let n = cross(direction, basis);
    let first_plane = Plane {
        normal: n,
        offset: dot(n, start),
    }
    .normalized()?;
    let n = cross(direction, first_plane.normal);
    let second_plane = Plane {
        normal: n,
        offset: dot(n, start),
    }
    .normalized()?;
    let source = curve_plane(curve, first_plane, options)?;
    let mut report = Report {
        components: vec![],
        unresolved: source.unresolved,
        boxes_visited: source.boxes_visited,
        bernstein_excluded: source.bernstein_excluded,
        coverage: source.coverage,
    };
    let mut candidates = Vec::new();
    for component in source.components {
        match component {
            CurvePlaneComponent::Point(point) => candidates.push(CurvePlaneComponent::Point(point)),
            CurvePlaneComponent::Overlap {
                parameter_interval, ..
            } => {
                if report.boxes_visited >= options.max_boxes {
                    report.unresolved(parameter_interval, UnresolvedReason::BudgetExceeded);
                    continue;
                }
                let piece = curve.trim(parameter_interval[0], parameter_interval[1])?;
                let other = curve_plane(
                    &piece,
                    second_plane,
                    Options {
                        max_boxes: options.max_boxes - report.boxes_visited,
                        ..options
                    },
                )?;
                report.boxes_visited += other.boxes_visited;
                report.bernstein_excluded += other.bernstein_excluded;
                if other.coverage == Coverage::Incomplete {
                    report.coverage = Coverage::Incomplete;
                }
                report.unresolved.extend(other.unresolved);
                candidates.extend(other.components);
            }
        }
    }
    let project = |p| dot(sub(p, start), direction) / length;
    for candidate in candidates {
        let interval = match &candidate {
            CurvePlaneComponent::Point(p) => p.parameter_interval,
            CurvePlaneComponent::Overlap {
                parameter_interval, ..
            } => *parameter_interval,
        };
        let hull = if interval[0] < interval[1] {
            curve
                .trim(interval[0], interval[1])?
                .control_points
                .iter()
                .map(|p| point3(p))
                .collect::<Vec<_>>()
        } else {
            vec![point3(&curve.evaluate(interval[0])?.point)]
        };
        let parameters = hull.iter().map(|p| project(*p)).collect::<Vec<_>>();
        if parameters.iter().all(|t| *t < 0.) || parameters.iter().all(|t| *t > 1.) {
            continue;
        }
        let distances = hull
            .iter()
            .map(|p| second_plane.distance(*p))
            .collect::<Vec<_>>();
        if distances.iter().all(|d| *d > options.distance_tolerance)
            || distances.iter().all(|d| *d < -options.distance_tolerance)
        {
            continue;
        }
        if !parameters
            .iter()
            .all(|t| t.is_finite() && (0. ..=1.).contains(t))
        {
            if matches!(&candidate, CurvePlaneComponent::Overlap { .. })
                && curve.degree == 1
                && parameters.len() == 2
                && parameters.iter().all(|t| t.is_finite())
            {
                let piece = curve.trim(interval[0], interval[1])?;
                let [a, b] = [parameters[0], parameters[1]];
                let low = a.min(b).max(0.);
                let high = a.max(b).min(1.);
                let scale = piece.weights[0].max(piece.weights[1]);
                let [wa, wb] = [piece.weights[0] / scale, piece.weights[1] / scale];
                let parameter = |q: f64| {
                    let r = if q == a {
                        0.
                    } else if q == b {
                        1.
                    } else {
                        let numerator = wa * (q - a);
                        numerator / (wb * (b - q) + numerator)
                    };
                    interval[0] + r * (interval[1] - interval[0])
                };
                let [x, y] = [parameter(low), parameter(high)];
                if !x.is_finite()
                    || !y.is_finite()
                    || x < interval[0]
                    || x > interval[1]
                    || y < interval[0]
                    || y > interval[1]
                {
                    report.unresolved(interval, UnresolvedReason::NearCoincidence);
                    continue;
                }
                let clipped = [x.min(y), x.max(y)];
                if clipped[0] == clipped[1] && low != high {
                    report.unresolved(interval, UnresolvedReason::NearCoincidence);
                    continue;
                }
                if clipped[0] == clipped[1] {
                    let point = point3(&curve.evaluate(clipped[0])?.point);
                    let t = low;
                    let on_line = std::array::from_fn(|i| start[i] + t * delta[i]);
                    let d = sub(point, on_line);
                    let residual = d[0].hypot(d[1]).hypot(d[2]);
                    if residual > options.distance_tolerance {
                        report.unresolved(interval, UnresolvedReason::NearCoincidence);
                        continue;
                    }
                    if report.components.iter().any(|c| {
                        matches!(c,CurveSegmentComponent::Point {curve,..}
                        if curve.parameter==clipped[0])
                    }) {
                        continue;
                    }
                    report.components.push(CurveSegmentComponent::Point {
                        curve: CurvePoint {
                            parameter: clipped[0],
                            parameter_interval: clipped,
                            point,
                            plane_residual: first_plane.distance(point).abs(),
                            contact: Contact::Boundary,
                        },
                        segment_parameter: t,
                        line_residual: residual,
                    });
                } else {
                    report.components.push(CurveSegmentComponent::Overlap {
                        curve_interval: clipped,
                    });
                }
                continue;
            }
            if matches!(&candidate, CurvePlaneComponent::Overlap { .. }) {
                let piece = curve.trim(interval[0], interval[1])?;
                let mut cuts = vec![interval[0], interval[1]];
                let mut bands = Vec::new();
                let mut endpoints = Vec::new();
                for (position, segment_parameter) in [(start, 0.), (end, 1.)] {
                    if report.boxes_visited >= options.max_boxes {
                        bands.push(interval);
                        report.unresolved(interval, UnresolvedReason::BudgetExceeded);
                        break;
                    }
                    let boundary = curve_plane(
                        &piece,
                        Plane {
                            normal: direction,
                            offset: dot(direction, position),
                        },
                        Options {
                            max_boxes: options.max_boxes - report.boxes_visited,
                            ..options
                        },
                    )?;
                    report.boxes_visited += boundary.boxes_visited;
                    report.bernstein_excluded += boundary.bernstein_excluded;
                    for unresolved in boundary.unresolved {
                        let band = [unresolved.parameter_box[0], unresolved.parameter_box[1]];
                        cuts.extend(band);
                        bands.push(band);
                        report.unresolved(band, unresolved.reason);
                    }
                    for component in boundary.components {
                        match component {
                            CurvePlaneComponent::Point(point) => {
                                cuts.extend(point.parameter_interval);
                                if point.parameter_interval[0] == point.parameter_interval[1] {
                                    endpoints.push((point, segment_parameter));
                                } else {
                                    bands.push(point.parameter_interval);
                                    report.unresolved(
                                        point.parameter_interval,
                                        UnresolvedReason::BoundaryCrossing,
                                    );
                                }
                            }
                            CurvePlaneComponent::Overlap {
                                parameter_interval, ..
                            } => {
                                cuts.extend(parameter_interval);
                                bands.push(parameter_interval);
                                report.unresolved(
                                    parameter_interval,
                                    UnresolvedReason::CoincidentTrim,
                                );
                            }
                        }
                    }
                }
                cuts.sort_by(f64::total_cmp);
                cuts.dedup();
                for cell in cuts.windows(2) {
                    if cell[0] == cell[1]
                        || bands.iter().any(|b| cell[0] >= b[0] && cell[1] <= b[1])
                    {
                        continue;
                    }
                    if report.boxes_visited >= options.max_boxes {
                        report.unresolved(cell.to_vec(), UnresolvedReason::BudgetExceeded);
                        continue;
                    }
                    report.boxes_visited += 1;
                    let cell_curve = curve.trim(cell[0], cell[1])?;
                    let mut projection = cell_curve
                        .control_points
                        .iter()
                        .map(|p| project(point3(p)))
                        .collect::<Vec<_>>();
                    // Knot insertion can round the trim's endpoint controls.
                    // Query the authoritative source at the retained cut parameters.
                    projection[0] = project(point3(&curve.evaluate(cell[0])?.point));
                    let last = projection.len() - 1;
                    projection[last] = project(point3(&curve.evaluate(cell[1])?.point));

                    if projection.iter().all(|t| *t <= 0.) || projection.iter().all(|t| *t >= 1.) {
                        continue;
                    }
                    if projection
                        .iter()
                        .all(|t| t.is_finite() && (0. ..=1.).contains(t))
                    {
                        report.components.push(CurveSegmentComponent::Overlap {
                            curve_interval: [cell[0], cell[1]],
                        });
                    } else {
                        report.unresolved(cell.to_vec(), UnresolvedReason::CoincidentTrim);
                    }
                }
                for (point, t) in endpoints {
                    let covered = report.components.iter().any(|c| match c {
                        CurveSegmentComponent::Point { curve, .. } => {
                            curve.parameter == point.parameter
                        }
                        CurveSegmentComponent::Overlap { curve_interval } => {
                            point.parameter >= curve_interval[0]
                                && point.parameter <= curve_interval[1]
                        }
                    });
                    if covered {
                        continue;
                    }
                    let on_line = std::array::from_fn(|i| start[i] + t * delta[i]);
                    let d = sub(point.point, on_line);
                    let residual = d[0].hypot(d[1]).hypot(d[2]);
                    if residual <= options.distance_tolerance {
                        report.components.push(CurveSegmentComponent::Point {
                            curve: point,
                            segment_parameter: t,
                            line_residual: residual,
                        });
                    } else {
                        report.unresolved(
                            point.parameter_interval,
                            UnresolvedReason::NearCoincidence,
                        );
                    }
                }
                continue;
            }
            report.unresolved(
                interval,
                match candidate {
                    CurvePlaneComponent::Overlap { .. } => UnresolvedReason::CoincidentTrim,
                    _ => UnresolvedReason::BoundaryCrossing,
                },
            );
            continue;
        }
        match candidate {
            CurvePlaneComponent::Point(point) => {
                let t = project(point.point);
                let on_line = std::array::from_fn(|i| start[i] + t * delta[i]);
                let d = sub(point.point, on_line);
                let residual = d[0].hypot(d[1]).hypot(d[2]);
                if residual <= options.distance_tolerance {
                    if !report.components.iter().any(|c|matches!(c,CurveSegmentComponent::Point {curve,..} if curve.parameter==point.parameter)) {
                        report.components.push(CurveSegmentComponent::Point {curve:point,segment_parameter:t,line_residual:residual});
                    }
                } else {
                    report.unresolved(interval, UnresolvedReason::NearCoincidence);
                }
            }
            CurvePlaneComponent::Overlap { .. } => {
                report.components.push(CurveSegmentComponent::Overlap {
                    curve_interval: interval,
                })
            }
        }
    }
    report.components.sort_by(|a, b| {
        let parameter = |c: &CurveSegmentComponent| match c {
            CurveSegmentComponent::Point { curve, .. } => curve.parameter,
            CurveSegmentComponent::Overlap { curve_interval } => curve_interval[0],
        };
        parameter(a).total_cmp(&parameter(b))
    });
    Ok(report)
}

/// Numerical curve/curve correspondence for positive-weight 3D NURBS pairs.
/// Points retain both source parameters and their isolating boxes; overlaps
/// retain ascending source intervals on both curves plus the run direction.
#[derive(Clone, Debug)]
pub enum CurveCurveComponent {
    Point {
        first: f64,
        first_interval: [f64; 2],
        second: f64,
        second_interval: [f64; 2],
        point: [f64; 3],
        residual: f64,
        contact: Contact,
    },
    /// Whole coincident span pair or clipped collinear coincidence. Intervals
    /// ascend in each source knot domain; `reversed` marks opposite runs.
    Overlap {
        first_interval: [f64; 2],
        second_interval: [f64; 2],
        reversed: bool,
        max_control_residual: f64,
    },
}

/// Crossings below this unit-tangent sine stay explicitly unresolved:
/// near-parallel contacts cannot be certified transverse numerically.
const TRANSVERSE_SINE: f64 = 1e-6;

fn homogeneous4(curve: &Curve) -> Vec<[f64; 4]> {
    curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(p, w)| [p[0] * w, p[1] * w, p[2] * w, *w])
        .collect()
}
fn split_homogeneous(h: &[[f64; 4]]) -> (Vec<[f64; 4]>, Vec<[f64; 4]>) {
    let mut row = h.to_vec();
    let mut left = vec![row[0]];
    let mut right = vec![*row.last().unwrap()];
    while row.len() > 1 {
        row = row
            .windows(2)
            .map(|p| std::array::from_fn(|i| (p[0][i] + p[1][i]) * 0.5))
            .collect();
        left.push(row[0]);
        right.push(*row.last().unwrap());
    }
    right.reverse();
    (left, right)
}
/// Outward-rounded Cartesian control ranges of a positive-weight homogeneous
/// Bezier piece. Convex-hull containment makes axis gaps strict exclusions.
fn hull_ranges(h: &[[f64; 4]]) -> [[f64; 2]; 3] {
    std::array::from_fn(|axis| {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for p in h {
            let x = p[axis] / p[3];
            lo = lo.min(x.next_down());
            hi = hi.max(x.next_up());
        }
        [lo, hi]
    })
}
fn hulls_excluded(a: &[[f64; 4]], b: &[[f64; 4]]) -> bool {
    let ra = hull_ranges(a);
    let rb = hull_ranges(b);
    (0..3).any(|axis| ra[axis][1] < rb[axis][0] || rb[axis][1] < ra[axis][0])
}
#[inline(always)]
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = sub(a, b);
    d[0].hypot(d[1]).hypot(d[2])
}
/// One-sided first derivatives at a parameter where the two-sided jet may not
/// exist (a C0 knot): the left jet from the Bézier span ending at t and the
/// right jet from the span starting at t. Entries outside the active domain
/// stay None.
fn one_sided_curve_d1(curve: &Curve, t: f64) -> Result<[Option<Vec<f64>>; 2]> {
    let domain = curve.domain();
    let mut jets = [None, None];
    if t > domain[0]
        && let Some(a) = curve
            .knots
            .iter()
            .copied()
            .filter(|&k| k < t)
            .max_by(f64::total_cmp)
    {
        jets[0] = curve.trim(a, t)?.evaluate(t)?.d1;
    }
    if t < domain[1]
        && let Some(b) = curve
            .knots
            .iter()
            .copied()
            .filter(|&k| k > t)
            .min_by(f64::total_cmp)
    {
        jets[1] = curve.trim(t, b)?.evaluate(t)?.d1;
    }
    Ok(jets)
}
/// Tangent vectors at t: the two-sided derivative when it exists, otherwise
/// every available one-sided span derivative at a C0 knot, in [left, right]
/// order. An empty result means no usable jet exists at all.
fn curve_tangents(curve: &Curve, t: f64) -> Result<Vec<[f64; 3]>> {
    let jet = curve.evaluate(t)?;
    if let Some(d1) = &jet.d1 {
        return Ok(vec![point3(d1)]);
    }
    Ok(one_sided_curve_d1(curve, t)?
        .into_iter()
        .flatten()
        .map(|d| point3(&d))
        .collect())
}
/// Surface tangent vectors at (u,v): the two-sided jets when they exist,
/// otherwise the available one-sided span jets across C0 knots in U and V.
type SurfaceTangentPair = (Vec<[f64; 3]>, Vec<[f64; 3]>);

fn surface_tangents(surface: &Surface, u: f64, v: f64) -> Result<SurfaceTangentPair> {
    let jet = surface.evaluate(u, v)?;
    if let Some((du, dv)) = jet.first_derivatives() {
        return Ok((vec![du], vec![dv]));
    }
    let d = surface_domain(surface);
    let mut us = Vec::new();
    let mut vs = Vec::new();
    if u > d[0]
        && let Some(a) = surface
            .knots_u
            .iter()
            .copied()
            .filter(|&k| k < u)
            .max_by(f64::total_cmp)
        && let Some((du, dv)) = surface
            .trim([a, u, d[2], d[3]])?
            .evaluate(u, v)?
            .first_derivatives()
    {
        us.push(du);
        vs.push(dv);
    }
    if u < d[1]
        && let Some(b) = surface
            .knots_u
            .iter()
            .copied()
            .filter(|&k| k > u)
            .min_by(f64::total_cmp)
        && let Some((du, dv)) = surface
            .trim([u, b, d[2], d[3]])?
            .evaluate(u, v)?
            .first_derivatives()
    {
        us.push(du);
        vs.push(dv);
    }
    if vs.is_empty()
        && v > d[2]
        && let Some(a) = surface
            .knots_v
            .iter()
            .copied()
            .filter(|&k| k < v)
            .max_by(f64::total_cmp)
        && let Some((du, dv)) = surface
            .trim([d[0], d[1], a, v])?
            .evaluate(u, v)?
            .first_derivatives()
    {
        us.push(du);
        vs.push(dv);
    }
    if vs.is_empty()
        && v < d[3]
        && let Some(b) = surface
            .knots_v
            .iter()
            .copied()
            .filter(|&k| k > v)
            .min_by(f64::total_cmp)
        && let Some((du, dv)) = surface
            .trim([d[0], d[1], v, b])?
            .evaluate(u, v)?
            .first_derivatives()
    {
        us.push(du);
        vs.push(dv);
    }
    Ok((us, vs))
}
/// Unit-tangent sine at a parameter pair; zero/invalid tangents stay 0. At a
/// C0 knot every one-sided side pair is screened and the largest sine counts:
/// a crossing transverse from either side is resolvable.
fn tangent_sine(first: &Curve, second: &Curve, t: f64, u: f64) -> Result<f64> {
    let mut best: f64 = 0.;
    for va in curve_tangents(first, t)? {
        for vb in curve_tangents(second, u)? {
            let la = distance(va, [0.; 3]);
            let lb = distance(vb, [0.; 3]);
            if !la.is_finite() || la <= 0. || !lb.is_finite() || lb <= 0. {
                continue;
            }
            let c = cross(va.map(|x| x / la), vb.map(|x| x / lb));
            best = best.max(distance(c, [0.; 3]));
        }
    }
    Ok(best)
}
/// Outcome of an exact evaluation at a box face shared with the adjacent box.
enum FaceRoot {
    /// No confirmed root at the face.
    Miss,
    /// A confirmed transverse root at the face.
    Root(CurvePoint),
    /// A confirmed contact without a transverse one-sided jet.
    Ambiguous,
}
/// Exact curve/plane evaluation at a box face. Transversality is screened
/// with the one-sided jet interior to this box (the right side at the lower
/// face, the left side at the upper face), so a C0 knot root resolves when
/// its one-sided geometry is transverse. Ownership of shared faces is the
/// caller's decision (see `owns_parameter`).
fn plane_face_root(
    curve: &Curve,
    plane: Plane,
    interval: [f64; 2],
    face: f64,
    options: Options,
) -> Result<FaceRoot> {
    let domain = curve.domain();
    let point = point3(&curve.evaluate(face)?.point);
    let residual = plane.distance(point).abs();
    if residual > options.distance_tolerance {
        return Ok(FaceRoot::Miss);
    }
    let at_end = face == domain[0] || face == domain[1];
    let tangents = curve_tangents(curve, face)?;
    let side = if face == interval[0] {
        tangents.last()
    } else {
        tangents.first()
    };
    let transverse = side.is_some_and(|d| dot(plane.normal, *d).abs() > options.distance_tolerance);
    if !at_end && !transverse {
        return Ok(FaceRoot::Ambiguous);
    }
    Ok(FaceRoot::Root(CurvePoint {
        parameter: face,
        parameter_interval: [face, face],
        point,
        plane_residual: residual,
        contact: if at_end || curve.knots.contains(&face) {
            Contact::Boundary
        } else {
            Contact::Transverse
        },
    }))
}
/// Push a point event, deduplicating by exact parameter pair only — never by
/// spatial proximity, so repeated visits to one location stay distinct.
struct CurvePointEvent {
    first: f64,
    first_interval: [f64; 2],
    second: f64,
    second_interval: [f64; 2],
    point: [f64; 3],
    residual: f64,
    contact: Contact,
}

fn push_curve_point(report: &mut Report<CurveCurveComponent>, event: CurvePointEvent) {
    let CurvePointEvent {
        first,
        first_interval,
        second,
        second_interval,
        point,
        residual,
        contact,
    } = event;
    let duplicate = report.components.iter().any(|c| {
        matches!(c, CurveCurveComponent::Point { first: f, second: s, .. }
            if *f == first && *s == second)
    });
    if duplicate {
        return;
    }
    report.components.push(CurveCurveComponent::Point {
        first,
        first_interval,
        second,
        second_interval,
        point,
        residual,
        contact,
    });
}
/// Exact knot-corner contact admission. Interior corners need a transverse
/// crossing; domain-boundary corners are admitted on distance alone, matching
/// the curve/plane endpoint rule. Corners covered by an existing overlap are
/// owned by its interval, not reported again.
fn admit_curve_corner(
    first: &Curve,
    second: &Curve,
    t: f64,
    u: f64,
    options: Options,
    report: &mut Report<CurveCurveComponent>,
) -> Result<bool> {
    let covered = report.components.iter().any(|c| {
        matches!(c, CurveCurveComponent::Overlap { first_interval, second_interval, .. }
            if first_interval[0] <= t && t <= first_interval[1]
                && second_interval[0] <= u && u <= second_interval[1])
    });
    if covered {
        return Ok(true);
    }
    let pa = point3(&first.evaluate(t)?.point);
    let pb = point3(&second.evaluate(u)?.point);
    let residual = distance(pa, pb);
    if residual > options.distance_tolerance {
        return Ok(false);
    }
    let at_boundary = t == first.domain()[0]
        || t == first.domain()[1]
        || u == second.domain()[0]
        || u == second.domain()[1];
    if !at_boundary && tangent_sine(first, second, t, u)? <= TRANSVERSE_SINE {
        return Ok(false);
    }
    let contact = if at_boundary || first.knots.contains(&t) || second.knots.contains(&u) {
        Contact::Boundary
    } else {
        Contact::Transverse
    };
    push_curve_point(
        report,
        CurvePointEvent {
            first: t,
            first_interval: [t, t],
            second: u,
            second_interval: [u, u],
            point: std::array::from_fn(|i| (pa[i] + pb[i]) * 0.5),
            residual,
            contact,
        },
    );
    Ok(true)
}

#[derive(Debug)]
enum Refinement {
    /// Converged inside the isolating box: parameters, point, residual.
    Root(f64, f64, [f64; 3], f64),
    /// The Newton image left the box; the root's own box reports it.
    Outside,
    /// Degenerate tangent basis, nonfinite step, or stalled residual.
    Failed,
}
/// Newton refinement of a transverse midpoint candidate on the two dominant
/// cross-product axes. Only iterates that stay inside the isolating box are
/// reported by that box; halo boxes whose image lands elsewhere stay silent
/// instead of duplicating the root with shifted parameters.
fn refine_curve_root(
    first: &Curve,
    second: &Curve,
    ta: [f64; 2],
    tb: [f64; 2],
    tm: f64,
    um: f64,
    options: Options,
) -> Result<Refinement> {
    let (mut t, mut u) = (tm, um);
    let width_t = ta[1] - ta[0];
    let width_u = tb[1] - tb[0];
    for _ in 0..16 {
        if !(ta[0] <= t && t <= ta[1] && tb[0] <= u && u <= tb[1]) {
            return Ok(Refinement::Outside);
        }
        let ja = first.evaluate(t)?;
        let jb = second.evaluate(u)?;
        let r = sub(point3(&ja.point), point3(&jb.point));
        if distance(r, [0.; 3]) <= options.distance_tolerance * 1e-3 {
            // Converged; knots with discontinuous derivatives may have no
            // d1 here, and the box midpoint already screened transversality.
            break;
        }
        // At a C0 knot the two-sided jet does not exist: step with the
        // one-sided span jets, preferring the side the iterate came from,
        // and accept either side that yields a valid step.
        let mut a_sides = curve_tangents(first, t)?;
        if a_sides.len() == 2 && tm > t {
            a_sides.swap(0, 1);
        }
        let mut b_sides = curve_tangents(second, u)?;
        if b_sides.len() == 2 && um > u {
            b_sides.swap(0, 1);
        }
        let mut step = None;
        'sides: for va in &a_sides {
            for vb in &b_sides {
                let la = distance(*va, [0.; 3]);
                let lb = distance(*vb, [0.; 3]);
                let c = cross(*va, *vb);
                let dominant = c.map(|x| x.abs()).into_iter().fold(0., f64::max);
                if !dominant.is_finite() || dominant <= TRANSVERSE_SINE * la * lb {
                    continue;
                }
                // Solve A(t)-B(u)=0 on the two axes with the best-conditioned minor.
                let drop = if c[0].abs() == dominant {
                    0
                } else if c[1].abs() == dominant {
                    1
                } else {
                    2
                };
                let keep: [usize; 2] = match drop {
                    0 => [1, 2],
                    1 => [0, 2],
                    _ => [0, 1],
                };
                // va[k]*dt - vb[k]*du = -r[k] on both kept axes.
                let (a, b, e0) = (va[keep[0]], -vb[keep[0]], -r[keep[0]]);
                let (c2, d2, e1) = (va[keep[1]], -vb[keep[1]], -r[keep[1]]);
                let det = a * d2 - b * c2;
                if !det.is_finite() || det.abs() <= 0. {
                    continue;
                }
                let dt = (e0 * d2 - b * e1) / det;
                let du = (a * e1 - e0 * c2) / det;
                if !dt.is_finite() || !du.is_finite() {
                    continue;
                }
                step = Some((dt, du));
                break 'sides;
            }
        }
        let Some((dt, du)) = step else {
            return Ok(Refinement::Failed);
        };
        t += dt;
        u += du;
        if dt.abs() <= width_t * 1e-6 && du.abs() <= width_u * 1e-6 {
            break;
        }
    }
    // Half-open ownership: reconcile the converged image with the bitwise
    // shared faces, then exactly one box — interior and lower face, plus the
    // domain's upper end for the last box — reports a boundary-sitting root.
    let t = snap_to_face(snap_to_face(t, ta[0]), ta[1]);
    let u = snap_to_face(snap_to_face(u, tb[0]), tb[1]);
    if !owns_parameter(ta[0], ta[1], first.domain()[1], t)
        || !owns_parameter(tb[0], tb[1], second.domain()[1], u)
    {
        return Ok(Refinement::Outside);
    }
    let ja = first.evaluate(t)?;
    let jb = second.evaluate(u)?;
    let pa = point3(&ja.point);
    let pb = point3(&jb.point);
    let residual = distance(pa, pb);
    if residual > options.distance_tolerance {
        return Ok(Refinement::Failed);
    }
    // One-sided jets screen a converged C0-knot root: it resolves when either
    // side is transverse, and stays unresolved when both sides are tangent.
    if tangent_sine(first, second, t, u)? <= TRANSVERSE_SINE {
        return Ok(Refinement::Failed);
    }
    Ok(Refinement::Root(
        t,
        u,
        std::array::from_fn(|i| (pa[i] + pb[i]) * 0.5),
        residual,
    ))
}
/// Terminal box resolution. The hull-diagonal lower bound separates provably
/// disjoint pieces; ambiguous residual bands stay unresolved; transverse
/// midpoint candidates must refine to a certified in-box root. Tangent or
/// multiple-root regions are never collapsed to a guessed point.
#[allow(clippy::too_many_arguments)]
fn resolve_curve_box(
    first: &Curve,
    second: &Curve,
    ta: [f64; 2],
    tb: [f64; 2],
    tm: f64,
    um: f64,
    ha: &[[f64; 4]],
    hb: &[[f64; 4]],
    options: Options,
    report: &mut Report<CurveCurveComponent>,
) -> Result<()> {
    let box4 = || vec![ta[0], ta[1], tb[0], tb[1]];
    let diagonal = |h: &[[f64; 4]]| {
        hull_ranges(h)
            .iter()
            .map(|r| {
                let w = r[1] - r[0];
                w * w
            })
            .sum::<f64>()
            .sqrt()
    };
    let diag = diagonal(ha) + diagonal(hb);
    let pa = point3(&first.evaluate(tm)?.point);
    let pb = point3(&second.evaluate(um)?.point);
    let residual = distance(pa, pb);
    // |A(t)-B(u)| >= residual - diag everywhere in the box (triangle bound).
    if residual - diag > options.distance_tolerance {
        return Ok(());
    }
    if residual > options.distance_tolerance {
        report.unresolved(box4(), UnresolvedReason::NearCoincidence);
        return Ok(());
    }
    if tangent_sine(first, second, tm, um)? <= TRANSVERSE_SINE {
        report.unresolved(box4(), UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(());
    }
    match refine_curve_root(first, second, ta, tb, tm, um, options)? {
        Refinement::Root(t, u, point, residual) => push_curve_point(
            report,
            CurvePointEvent {
                first: t,
                first_interval: ta,
                second: u,
                second_interval: tb,
                point,
                residual,
                contact: Contact::Transverse,
            },
        ),
        Refinement::Outside => (),
        Refinement::Failed => report.unresolved(box4(), UnresolvedReason::TangencyOrMultipleRoot),
    }
    Ok(())
}
/// Merge point events whose isolating intervals touch in parameter space.
/// Subdivision tiles parameter space with exactly shared binary64 boundaries,
/// so adjacency is decided on interval endpoints — never on spatial
/// proximity. The merged event keeps the lowest-residual parameter pair.
fn merge_curve_points(report: &mut Report<CurveCurveComponent>) {
    let intervals = |c: &CurveCurveComponent| match c {
        CurveCurveComponent::Point {
            first_interval,
            second_interval,
            ..
        } => Some((*first_interval, *second_interval)),
        _ => None,
    };
    let n = report.components.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for i in 0..n {
        for j in i + 1..n {
            let (Some((af, as_)), Some((bf, bs))) = (
                intervals(&report.components[i]),
                intervals(&report.components[j]),
            ) else {
                continue;
            };
            if af[0] <= bf[1] && bf[0] <= af[1] && as_[0] <= bs[1] && bs[0] <= as_[1] {
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                if ri != rj {
                    parent[ri] = rj;
                }
            }
        }
    }
    let mut merged: Vec<Option<CurveCurveComponent>> = (0..n).map(|_| None).collect();
    let mut order: Vec<usize> = Vec::new();
    for (i, component) in report.components.iter().take(n).enumerate() {
        let Some((fi, si)) = intervals(component) else {
            continue;
        };
        let root = find(&mut parent, i);
        if merged[root].is_none() {
            order.push(root);
        }
        let CurveCurveComponent::Point {
            first,
            second,
            point,
            residual,
            contact,
            ..
        } = component
        else {
            continue;
        };
        let member = (*first, *second, *point, *residual, *contact, (fi, si));
        match &mut merged[root] {
            None => {
                merged[root] = Some(CurveCurveComponent::Point {
                    first: member.0,
                    first_interval: fi,
                    second: member.1,
                    second_interval: si,
                    point: member.2,
                    residual: member.3,
                    contact: member.4,
                });
            }
            Some(CurveCurveComponent::Point {
                first,
                first_interval,
                second,
                second_interval,
                point,
                residual,
                contact,
            }) => {
                if member.3 < *residual {
                    *first = member.0;
                    *second = member.1;
                    *point = member.2;
                    *residual = member.3;
                }
                if member.4 == Contact::Boundary {
                    *contact = Contact::Boundary;
                }
                first_interval[0] = first_interval[0].min(member.5.0[0]);
                first_interval[1] = first_interval[1].max(member.5.0[1]);
                second_interval[0] = second_interval[0].min(member.5.1[0]);
                second_interval[1] = second_interval[1].max(member.5.1[1]);
            }
            _ => unreachable!(),
        }
    }
    let mut components = Vec::new();
    let mut index = 0;
    for root in order {
        while index < root {
            if intervals(&report.components[index]).is_none() {
                components.push(report.components[index].clone());
            }
            index += 1;
        }
        if let Some(c) = merged[root].take() {
            components.push(c);
        }
        index = root + 1;
    }
    while index < n {
        if intervals(&report.components[index]).is_none() {
            components.push(report.components[index].clone());
        }
        index += 1;
    }
    report.components = components;
}

enum LineInversion {
    Root(f64),
    Ambiguous,
    BudgetExhausted,
}
/// Invert the rational line support of a collinear span at one line offset
/// through the shared-budget curve/plane isolator. Multiple or uncertain
/// roots keep the coincidence clip ambiguous rather than guessed.
fn invert_line_parameter(
    piece: &Curve,
    direction: [f64; 3],
    offset: f64,
    options: Options,
    report: &mut Report<CurveCurveComponent>,
) -> Result<LineInversion> {
    if report.boxes_visited >= options.max_boxes {
        return Ok(LineInversion::BudgetExhausted);
    }
    let roots = curve_plane(
        piece,
        Plane {
            normal: direction,
            offset,
        },
        Options {
            max_boxes: options.max_boxes - report.boxes_visited,
            ..options
        },
    )?;
    report.boxes_visited += roots.boxes_visited;
    report.bernstein_excluded += roots.bernstein_excluded;
    if !roots.unresolved.is_empty() {
        return Ok(LineInversion::Ambiguous);
    }
    let mut found = None;
    for component in roots.components {
        match component {
            CurvePlaneComponent::Point(point) => {
                if found.is_some() {
                    return Ok(LineInversion::Ambiguous);
                }
                found = Some(point.parameter);
            }
            CurvePlaneComponent::Overlap { .. } => return Ok(LineInversion::Ambiguous),
        }
    }
    Ok(found.map_or(LineInversion::Ambiguous, LineInversion::Root))
}
/// Coincidence admission for one source span pair. Collinear pairs clip the
/// shared line interval by inverting both rational parameterizations; curved
/// pairs coincide only under proportional homogeneous polygons after degree
/// elevation. Anything else returns false so subdivision can isolate points.
/// Returns true when the pair is fully handled (component, empty, or an
/// explicit unresolved region) and must not enter point subdivision.
fn curve_coincidence(
    piece_a: &Curve,
    piece_b: &Curve,
    ta: [f64; 2],
    tb: [f64; 2],
    options: Options,
    report: &mut Report<CurveCurveComponent>,
) -> Result<bool> {
    let box4 = || vec![ta[0], ta[1], tb[0], tb[1]];
    let a0 = point3(piece_a.control_points.first().unwrap());
    let a1 = point3(piece_a.control_points.last().unwrap());
    let d = sub(a1, a0);
    let length = d[0].hypot(d[1]).hypot(d[2]);
    if !length.is_finite() || length <= 0. {
        // Degenerate span: no support line; leave it to point isolation.
        return Ok(false);
    }
    let direction = d.map(|x| x / length);
    let off_line = |c: &[f64]| {
        let v = cross(direction, sub(point3(c), a0));
        v[0].hypot(v[1]).hypot(v[2])
    };
    let residual_a = piece_a
        .control_points
        .iter()
        .map(|c| off_line(c))
        .fold(0., f64::max);
    let residual_b = piece_b
        .control_points
        .iter()
        .map(|c| off_line(c))
        .fold(0., f64::max);
    let max_control_residual = residual_a.max(residual_b);
    if max_control_residual <= options.distance_tolerance {
        // Both spans numerically lie on one support line, so their
        // intersection is exactly the shared line interval (possibly empty).
        let projections = |piece: &Curve| {
            piece
                .control_points
                .iter()
                .map(|c| dot(direction, point3(c)))
                .collect::<Vec<_>>()
        };
        let sa = projections(piece_a);
        let sb = projections(piece_b);
        let monotone =
            |s: &[f64]| s.windows(2).all(|w| w[1] >= w[0]) || s.windows(2).all(|w| w[1] <= w[0]);
        if !monotone(&sa) || !monotone(&sb) {
            report.unresolved(box4(), UnresolvedReason::CoincidentTrim);
            return Ok(true);
        }
        let (sa0, sa1) = (*sa.first().unwrap(), *sa.last().unwrap());
        let (sb0, sb1) = (*sb.first().unwrap(), *sb.last().unwrap());
        let lo = sa0.max(sb0.min(sb1));
        let hi = sa1.min(sb0.max(sb1));
        if hi < lo {
            return Ok(true);
        }
        let mut parameters = [0.; 4];
        for (index, (piece, s)) in [(piece_a, lo), (piece_a, hi), (piece_b, lo), (piece_b, hi)]
            .into_iter()
            .enumerate()
        {
            match invert_line_parameter(piece, direction, s, options, report)? {
                LineInversion::Root(parameter) => parameters[index] = parameter,
                LineInversion::Ambiguous => {
                    report.unresolved(box4(), UnresolvedReason::CoincidentTrim);
                    return Ok(true);
                }
                LineInversion::BudgetExhausted => {
                    report.unresolved(box4(), UnresolvedReason::BudgetExceeded);
                    return Ok(true);
                }
            }
        }
        let [ta_lo, ta_hi, tb_lo, tb_hi] = parameters;
        if hi == lo {
            let point_a = point3(&piece_a.evaluate(ta_lo)?.point);
            let point_b = point3(&piece_b.evaluate(tb_lo)?.point);
            let residual = distance(point_a, point_b);
            if residual > options.distance_tolerance {
                report.unresolved(box4(), UnresolvedReason::NearCoincidence);
                return Ok(true);
            }
            admit_curve_corner(piece_a, piece_b, ta_lo, tb_lo, options, report)?;
            return Ok(true);
        }
        if ta_lo == ta_hi || tb_lo == tb_hi {
            report.unresolved(box4(), UnresolvedReason::NearCoincidence);
            return Ok(true);
        }
        let reversed = (ta_lo < ta_hi) != (tb_lo < tb_hi);
        push_curve_overlap(
            [ta_lo.min(ta_hi), ta_lo.max(ta_hi)],
            [tb_lo.min(tb_hi), tb_lo.max(tb_hi)],
            reversed,
            max_control_residual,
            report,
        );
        return Ok(true);
    }
    // Curved spans coincide only with proportional homogeneous polygons after
    // elevation; other partial curved coincidences stay with point isolation
    // and remain explicit unresolved bands where they cannot be separated.
    let degree = piece_a.degree.max(piece_b.degree);
    if degree > 25 {
        return Ok(false);
    }
    let ea = piece_a.elevate(degree)?;
    let eb = piece_b.elevate(degree)?;
    for reversed in [false, true] {
        let lambda = {
            let index = ea
                .weights
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, _)| i)
                .unwrap();
            let j = if reversed { degree - index } else { index };
            eb.weights[j] / ea.weights[index]
        };
        if !lambda.is_finite() || lambda <= 0. {
            continue;
        }
        let weight_scale = eb.weights.iter().copied().fold(0., f64::max);
        let weight_residual = ea
            .weights
            .iter()
            .enumerate()
            .map(|(i, wa)| {
                let j = if reversed { degree - i } else { i };
                (eb.weights[j] - lambda * wa).abs()
            })
            .fold(0., f64::max)
            / weight_scale;
        let control_residual = ea
            .control_points
            .iter()
            .enumerate()
            .map(|(i, pa)| {
                let j = if reversed { degree - i } else { i };
                distance(point3(pa), point3(&eb.control_points[j]))
            })
            .fold(0., f64::max);
        if weight_residual <= 1e-9 && control_residual <= options.distance_tolerance {
            push_curve_overlap(ta, tb, reversed, control_residual, report);
            return Ok(true);
        }
    }
    Ok(false)
}
/// Push an overlap and drop corner events covered by both intervals. The
/// overlap owns its boundary contacts; containment is by parameter, never
/// by spatial proximity.
fn push_curve_overlap(
    first_interval: [f64; 2],
    second_interval: [f64; 2],
    reversed: bool,
    max_control_residual: f64,
    report: &mut Report<CurveCurveComponent>,
) {
    let covered = |c: &CurveCurveComponent| {
        matches!(c, CurveCurveComponent::Point { first, second, .. }
            if first_interval[0] <= *first && *first <= first_interval[1]
                && second_interval[0] <= *second && *second <= second_interval[1])
    };
    report.components.retain(|c| !covered(c));
    report.components.push(CurveCurveComponent::Overlap {
        first_interval,
        second_interval,
        reversed,
        max_control_residual,
    });
}

/// Intersect two retained positive-weight 3D NURBS curves over their entire
/// active knot domains. Knot spans decompose into homogeneous rational Bezier
/// pieces traversed by one FIFO queue across all span pairs and subdivisions;
/// positive-weight control hulls exclude disjoint boxes with outward rounding.
/// Transverse points carry both source parameters and their isolating boxes,
/// exact knot corners own boundary contacts (dedup by parameter pair only),
/// and coincident spans retain explicit trim intervals on both curves.
/// Tangencies, ambiguous coincidence clipping and exhausted budgets stay
/// unresolved; results never authorize topology changes.
pub fn curve_curve(
    first: &Curve,
    second: &Curve,
    options: Options,
) -> Result<Report<CurveCurveComponent>> {
    first.validate()?;
    second.validate()?;
    if first.control_points[0].len() != 3 || second.control_points[0].len() != 3 {
        return Err(invalid("Curve/curve requires two 3D curves"));
    }
    let options = options.validate()?;
    let mut report = Report::default();
    type CurveCurvePending = (
        [f64; 2],
        [f64; 2],
        Option<Vec<[f64; 4]>>,
        Option<Vec<[f64; 4]>>,
        usize,
    );
    let mut pending: std::collections::VecDeque<CurveCurvePending> =
        spans(&first.knots, first.degree, first.control_points.len())
            .into_iter()
            .flat_map(|ta| {
                spans(&second.knots, second.degree, second.control_points.len())
                    .into_iter()
                    .map(move |tb| (ta, tb, None, None, 0))
            })
            .collect();
    while let Some((ta, tb, ha, hb, depth)) = pending.pop_front() {
        if report.boxes_visited >= options.max_boxes {
            report.unresolved(
                vec![ta[0], ta[1], tb[0], tb[1]],
                UnresolvedReason::BudgetExceeded,
            );
            continue;
        }
        report.boxes_visited += 1;
        let (pieces, ha, hb) = match (ha, hb) {
            (Some(ha), Some(hb)) => (None, ha, hb),
            _ => {
                let pa = first.trim(ta[0], ta[1])?;
                let pb = second.trim(tb[0], tb[1])?;
                let (ha, hb) = (homogeneous4(&pa), homogeneous4(&pb));
                (Some((pa, pb)), ha, hb)
            }
        };
        if let Some((pa, pb)) = &pieces {
            for &t in &ta {
                for &u in &tb {
                    admit_curve_corner(first, second, t, u, options, &mut report)?;
                }
            }
            if !hulls_excluded(&ha, &hb) && curve_coincidence(pa, pb, ta, tb, options, &mut report)?
            {
                continue;
            }
        }
        if hulls_excluded(&ha, &hb) {
            report.bernstein_excluded += 1;
            continue;
        }
        let width_a = ta[1] - ta[0];
        let width_b = tb[1] - tb[0];
        let tm = ta[0] + width_a * 0.5;
        let um = tb[0] + width_b * 0.5;
        let can_a = tm > ta[0] && tm < ta[1];
        let can_b = um > tb[0] && um < tb[1];
        if (width_a <= options.parameter_tolerance && width_b <= options.parameter_tolerance)
            || depth == options.max_depth
            || (!can_a && !can_b)
        {
            resolve_curve_box(
                first,
                second,
                ta,
                tb,
                tm,
                um,
                &ha,
                &hb,
                options,
                &mut report,
            )?;
            continue;
        }
        // Bisect both sides per depth level so depth 48 bounds each width by
        // 2^-48 of the source span; four children keep the FIFO fair order.
        let (al, ar) = split_homogeneous(&ha);
        let (bl, br) = split_homogeneous(&hb);
        let a_side: Vec<([f64; 2], Vec<[f64; 4]>)> = if can_a {
            vec![([ta[0], tm], al), ([tm, ta[1]], ar)]
        } else {
            vec![(ta, ha.clone())]
        };
        let b_side: Vec<([f64; 2], Vec<[f64; 4]>)> = if can_b {
            vec![([tb[0], um], bl), ([um, tb[1]], br)]
        } else {
            vec![(tb, hb.clone())]
        };
        for (ta2, h) in &a_side {
            for (tb2, g) in &b_side {
                pending.push_back((*ta2, *tb2, Some(h.clone()), Some(g.clone()), depth + 1));
            }
        }
    }
    merge_curve_points(&mut report);
    report.components.sort_by(|a, b| {
        let parameters = |c: &CurveCurveComponent| match c {
            CurveCurveComponent::Point { first, second, .. } => (*first, *second),
            CurveCurveComponent::Overlap {
                first_interval,
                second_interval,
                ..
            } => (first_interval[0], second_interval[0]),
        };
        let (af, as_) = parameters(a);
        let (bf, bs) = parameters(b);
        af.total_cmp(&bf).then(as_.total_cmp(&bs))
    });
    Ok(report)
}

#[derive(Clone, Debug)]
pub enum CurveRuledSurfaceComponent {
    /// Newton-confirmed root of C(t) = S(u,v) inside its isolating box.
    Point {
        t: f64,
        t_interval: [f64; 2],
        uv: [f64; 2],
        /// Isolating surface box [u0,u1,v0,v1]; V never subdivides on a ruling.
        uv_box: [f64; 4],
        point: [f64; 3],
        residual: f64,
        contact: Contact,
    },
    /// Curve span lying on the ruled surface: a ruling (constant U) or an
    /// iso-V directrix blend. The lifted UV path is the UV segment from
    /// uv_start to uv_end; its endpoints correspond to the curve interval
    /// ends. `correspondence` carries three (t,u,v) samples at the curve
    /// interval fractions 0, 1/2 and 1, sufficient to reconstruct the exact
    /// per-parameter map: a Möbius t->v map along a ruling (unequal endpoint
    /// weights make V a cross-ratio function of t) or an affine t->u map
    /// along an iso-V directrix. It is None only where no single map covers
    /// the merged interval (seam-joined pieces). `seam_wrap` marks a
    /// component unified across an exactly closed U seam: the canonical
    /// representative sits at u = u_min and the u_max-side copy is folded
    /// in, or an iso-V path leaves the domain at u_max and re-enters at
    /// u_min.
    Overlap {
        curve_interval: [f64; 2],
        uv_start: [f64; 2],
        uv_end: [f64; 2],
        max_control_residual: f64,
        seam_wrap: bool,
        correspondence: Option<OverlapCorrespondence>,
    },
}

/// Per-parameter map samples along a curve/ruled-surface overlap. `samples`
/// are exact (t,u,v) triples at curve-interval fractions 0, 1/2, 1.
#[derive(Clone, Debug)]
pub struct OverlapCorrespondence {
    pub kind: OverlapCorrespondenceKind,
    pub samples: [[f64; 3]; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlapCorrespondenceKind {
    /// Ruling overlap: u is constant; v(t) is the unique Möbius map through
    /// the three (t,v) samples, reconstructed by the cross-ratio identity
    /// (v-v0)(v1-v2)/((v-v2)(v1-v0)) = (t-t0)(t1-t2)/((t-t2)(t1-t0)).
    MobiusV,
    /// Iso-V overlap: v is constant; u(t) is the affine map through the
    /// three (t,u) samples.
    AffineU,
}

/// Homogeneous control net of a ruled patch, indexed [u][v] with two V rows.
type HomogeneousGrid = Vec<Vec<[f64; 4]>>;
fn homogeneous_grid(surface: &Surface) -> HomogeneousGrid {
    surface
        .control_points
        .iter()
        .zip(&surface.weights)
        .map(|(points, weights)| {
            points
                .iter()
                .zip(weights)
                .map(|(p, w)| [p[0] * w, p[1] * w, p[2] * w, *w])
                .collect()
        })
        .collect()
}
/// de Casteljau split of both V rows at the U midpoint of a Bezier patch.
fn split_grid_u(grid: &HomogeneousGrid) -> (HomogeneousGrid, HomogeneousGrid) {
    let columns: Vec<Vec<[f64; 4]>> = (0..grid[0].len())
        .map(|j| grid.iter().map(|row| row[j]).collect())
        .collect();
    let mut left: HomogeneousGrid = vec![Vec::new(); grid.len()];
    let mut right: HomogeneousGrid = vec![Vec::new(); grid.len()];
    for (j, column) in columns.iter().enumerate() {
        let (l, r) = split_homogeneous(column);
        for (i, row) in left.iter_mut().enumerate() {
            row.push(l[i]);
        }
        for (i, row) in right.iter_mut().enumerate() {
            row.push(r[i]);
        }
        let _ = j;
    }
    (left, right)
}
fn grid_ranges(grid: &HomogeneousGrid) -> [[f64; 2]; 3] {
    hull_ranges(&grid.iter().flatten().copied().collect::<Vec<_>>())
}
fn grids_excluded(a: &[[f64; 4]], b: &HomogeneousGrid) -> bool {
    let ra = hull_ranges(a);
    let rb = grid_ranges(b);
    (0..3).any(|axis| ra[axis][1] < rb[axis][0] || rb[axis][1] < ra[axis][0])
}
/// Seam state of the U direction of a canonical ruled-in-V surface. Closure
/// is geometric: the two seam ruling curves (the first and last homogeneous
/// control rows) must coincide as rational curves, i.e. be exactly
/// proportional with a positive ratio. Merging is never by tolerance: only a
/// bitwise-exact proportionality closes the seam; a near-proportional seam
/// (relative mismatch within 1e-6) stays duplicate and is reported as
/// explicit near_coincidence bands at both domain ends instead of a guess.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RuledSeam {
    Open,
    Closed,
    Uncertain,
}
fn ruled_seam_state(surface: &Surface) -> RuledSeam {
    let grid = homogeneous_grid(surface);
    let (first, last) = (&grid[0], &grid[grid.len() - 1]);
    let anchor = (0..4)
        .max_by(|&a, &b| first[0][a].abs().total_cmp(&first[0][b].abs()))
        .unwrap();
    if !first[0][anchor].is_finite() || first[0][anchor].abs() <= 0. {
        return RuledSeam::Open;
    }
    let ratio = last[0][anchor] / first[0][anchor];
    if !ratio.is_finite() || ratio <= 0. {
        return RuledSeam::Open;
    }
    let scale = first
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0., f64::max)
        .max(1.);
    let mismatch = |row: &[[f64; 4]], other: &[[f64; 4]]| {
        row.iter().zip(other).fold(0_f64, |m, (a, b)| {
            m.max((0..4).fold(0_f64, |m2, k| m2.max((b[k] - ratio * a[k]).abs())))
        })
    };
    let delta = mismatch(first, last);
    if delta == 0. {
        RuledSeam::Closed
    } else if delta <= 1e-6 * scale {
        RuledSeam::Uncertain
    } else {
        RuledSeam::Open
    }
}
/// Unit-column determinant of (C'(t), S_u, S_v); zero/invalid jets stay 0.
/// This vanishes exactly when the curve direction lies in the surface tangent
/// plane, so small values mark tangency rather than transverse crossings.
fn cs_transversality(curve: &Curve, surface: &Surface, t: f64, u: f64, v: f64) -> Result<f64> {
    let (us, vs) = surface_tangents(surface, u, v)?;
    let mut best: f64 = 0.;
    for a in curve_tangents(curve, t)? {
        for b in &us {
            for c in &vs {
                let (la, lb, lc) = (
                    distance(a, [0.; 3]),
                    distance(*b, [0.; 3]),
                    distance(*c, [0.; 3]),
                );
                if !la.is_finite()
                    || la <= 0.
                    || !lb.is_finite()
                    || lb <= 0.
                    || !lc.is_finite()
                    || lc <= 0.
                {
                    continue;
                }
                best = best.max(
                    dot(
                        a.map(|x| x / la),
                        cross(b.map(|x| x / lb), c.map(|x| x / lc)),
                    )
                    .abs(),
                );
            }
        }
    }
    Ok(best)
}
/// 3x3 solve with partial pivoting; ill-conditioned systems return None.
fn solve3(a: [[f64; 3]; 3], b: [f64; 3]) -> Option<[f64; 3]> {
    let scale = a.iter().flatten().fold(0_f64, |m, x| m.max(x.abs()));
    if !scale.is_finite() || scale <= 0. {
        return None;
    }
    let mut m = a;
    let mut r = b;
    for col in 0..3 {
        let pivot = (col..3).max_by(|&i, &j| m[i][col].abs().total_cmp(&m[j][col].abs()))?;
        if !m[pivot][col].is_finite() || m[pivot][col].abs() <= 1e-14 * scale {
            return None;
        }
        m.swap(col, pivot);
        r.swap(col, pivot);
        for row in col + 1..3 {
            let f = m[row][col] / m[col][col];
            for k in col..3 {
                m[row][k] -= f * m[col][k];
            }
            r[row] -= f * r[col];
        }
    }
    let mut x = [0.; 3];
    for i in (0..3).rev() {
        x[i] = (r[i] - (i + 1..3).map(|k| m[i][k] * x[k]).sum::<f64>()) / m[i][i];
    }
    if x.iter().all(|v| v.is_finite()) {
        Some(x)
    } else {
        None
    }
}
/// The overlap UV path is a UV segment; coverage is bounding-box containment
/// of the event parameters, never spatial proximity.
fn cs_overlap_covers(report: &Report<CurveRuledSurfaceComponent>, t: f64, uv: [f64; 2]) -> bool {
    report.components.iter().any(|c| {
        matches!(c, CurveRuledSurfaceComponent::Overlap { curve_interval, uv_start, uv_end, .. }
            if curve_interval[0] <= t && t <= curve_interval[1]
                && uv_start[0].min(uv_end[0]) <= uv[0] && uv[0] <= uv_start[0].max(uv_end[0])
                && uv_start[1].min(uv_end[1]) <= uv[1] && uv[1] <= uv_start[1].max(uv_end[1]))
    })
}
struct CurveSurfacePointEvent {
    t: f64,
    t_interval: [f64; 2],
    uv: [f64; 2],
    uv_box: [f64; 4],
    point: [f64; 3],
    residual: f64,
    contact: Contact,
}

fn push_cs_point(report: &mut Report<CurveRuledSurfaceComponent>, event: CurveSurfacePointEvent) {
    let CurveSurfacePointEvent {
        t,
        t_interval,
        uv,
        uv_box,
        point,
        residual,
        contact,
    } = event;
    let duplicate = report.components.iter().any(|c| {
        matches!(c, CurveRuledSurfaceComponent::Point { t: et, uv: euv, .. }
            if *et == t && *euv == uv)
    });
    if duplicate || cs_overlap_covers(report, t, uv) {
        return;
    }
    report.components.push(CurveRuledSurfaceComponent::Point {
        t,
        t_interval,
        uv,
        uv_box,
        point,
        residual,
        contact,
    });
}
fn push_cs_overlap(
    curve_interval: [f64; 2],
    uv_start: [f64; 2],
    uv_end: [f64; 2],
    max_control_residual: f64,
    seam_wrap: bool,
    correspondence: Option<OverlapCorrespondence>,
    report: &mut Report<CurveRuledSurfaceComponent>,
) {
    report.components.push(CurveRuledSurfaceComponent::Overlap {
        curve_interval,
        uv_start,
        uv_end,
        max_control_residual,
        seam_wrap,
        correspondence,
    });
    let (umin, umax) = (uv_start[0].min(uv_end[0]), uv_start[0].max(uv_end[0]));
    let (vmin, vmax) = (uv_start[1].min(uv_end[1]), uv_start[1].max(uv_end[1]));
    report.components.retain(|c| {
        !matches!(c, CurveRuledSurfaceComponent::Point { t, uv, .. }
            if curve_interval[0] <= *t && *t <= curve_interval[1]
                && umin <= uv[0] && uv[0] <= umax
                && vmin <= uv[1] && uv[1] <= vmax)
    });
}
/// Exact parameter-corner contact admission for curve/ruled-surface boxes.
/// Curve-domain endpoints are admitted on distance alone; interior contacts
/// need a transverse crossing so grazing corner touches stay unresolved.
#[allow(clippy::too_many_arguments)]
fn admit_cs_corner(
    curve: &Curve,
    surface: &Surface,
    t: f64,
    u: f64,
    v: f64,
    options: Options,
    report: &mut Report<CurveRuledSurfaceComponent>,
) -> Result<bool> {
    if cs_overlap_covers(report, t, [u, v]) {
        return Ok(true);
    }
    let pc = point3(&curve.evaluate(t)?.point);
    let ps = surface.evaluate(u, v)?.point;
    let residual = distance(pc, ps);
    if residual > options.distance_tolerance {
        return Ok(false);
    }
    let curve_end = t == curve.domain()[0] || t == curve.domain()[1];
    if !curve_end && cs_transversality(curve, surface, t, u, v)? <= TRANSVERSE_SINE {
        return Ok(false);
    }
    push_cs_point(
        report,
        CurveSurfacePointEvent {
            t,
            t_interval: [t, t],
            uv: [u, v],
            uv_box: [u, u, v, v],
            point: std::array::from_fn(|i| (pc[i] + ps[i]) * 0.5),
            residual,
            contact: Contact::Boundary,
        },
    );
    Ok(true)
}
/// Evaluated boundary weight of a positive-weight curve at a parameter.
fn curve_weight_at(curve: &Curve, u: f64) -> Result<f64> {
    let basis = nurbs_core::curve::basis(
        curve.degree,
        &curve.knots,
        curve.control_points.len(),
        u,
        curve.periodic,
    )?;
    Ok(basis
        .basis
        .iter()
        .zip(&curve.weights)
        .map(|(b, w)| b * w)
        .sum())
}
/// Curve-on-surface admission for one (curve span, surface U-span) pair in the
/// canonical ruled-in-V orientation. Two algebraically exact families are
/// admitted: the piece is collinear with a ruling (constant U, Möbius-lifted
/// V endpoints), or its homogeneous polygon is a proportional blend of the two
/// boundary polygons (constant V). Anything ambiguous is an explicit
/// unresolved region; non-coincident pairs return false for point isolation.
#[allow(clippy::too_many_arguments)]
fn ruled_coincidence(
    piece: &Curve,
    patch: &Surface,
    ta: [f64; 2],
    ua: [f64; 2],
    vd: [f64; 2],
    seam_u: Option<[f64; 2]>,
    options: Options,
    report: &mut Report<CurveRuledSurfaceComponent>,
) -> Result<bool> {
    let box6 = || vec![ta[0], ta[1], ua[0], ua[1], vd[0], vd[1]];
    let b0 = patch.iso(nurbs_core::surface::Axis::V, vd[0])?;
    let b1 = patch.iso(nurbs_core::surface::Axis::V, vd[1])?;
    let a0 = point3(piece.control_points.first().unwrap());
    let a1 = point3(piece.control_points.last().unwrap());
    let chord = sub(a1, a0);
    let length = chord[0].hypot(chord[1]).hypot(chord[2]);
    let mut boundary_on_line = false;
    if length > 0. {
        let dir = chord.map(|x| x / length);
        let off_line = |p: [f64; 3]| {
            let v = cross(dir, sub(p, a0));
            v[0].hypot(v[1]).hypot(v[2])
        };
        let control_residual = piece
            .control_points
            .iter()
            .map(|c| off_line(point3(c)))
            .fold(0., f64::max);
        if control_residual <= options.distance_tolerance {
            // A straight piece lies on the ruled surface only as a ruling:
            // find U where both boundary points sit on the support line.
            // Candidates are certified plane roots of B0 near the line;
            // halo bands touching a certified root are absorbed, bands that
            // provably stay off the second plane are dropped, and anything
            // else keeps the coincidence ambiguous instead of guessed.
            let axis = (0..3)
                .min_by(|a, b| dir[*a].abs().total_cmp(&dir[*b].abs()))
                .unwrap();
            let mut basis = [0.; 3];
            basis[axis] = 1.;
            let n = cross(dir, basis);
            let plane1 = Plane {
                normal: n,
                offset: dot(n, a0),
            }
            .normalized()?;
            let n = cross(dir, plane1.normal);
            let plane2 = Plane {
                normal: n,
                offset: dot(n, a0),
            }
            .normalized()?;
            if report.boxes_visited >= options.max_boxes {
                report.unresolved(box6(), UnresolvedReason::BudgetExceeded);
                return Ok(true);
            }
            let roots = curve_plane(
                &b0,
                plane1,
                Options {
                    max_boxes: options.max_boxes - report.boxes_visited,
                    ..options
                },
            )?;
            report.boxes_visited += roots.boxes_visited;
            report.bernstein_excluded += roots.bernstein_excluded;
            let mut line_points = Vec::new();
            let mut certified: Vec<[f64; 2]> = Vec::new();
            let mut bands: Vec<[f64; 2]> = Vec::new();
            let mut joint_bands: Vec<[f64; 2]> = Vec::new();
            for component in roots.components {
                match component {
                    CurvePlaneComponent::Point(point) => {
                        certified.push(point.parameter_interval);
                        if plane2.distance(point.point).abs() <= options.distance_tolerance {
                            line_points.push(point.parameter);
                        }
                    }
                    // B0 lies in the first plane over this interval: isolate
                    // its second-plane crossings through one more shared-budget
                    // query instead of assuming a line coincidence.
                    CurvePlaneComponent::Overlap {
                        parameter_interval, ..
                    } => {
                        if report.boxes_visited >= options.max_boxes {
                            report.unresolved(box6(), UnresolvedReason::BudgetExceeded);
                            return Ok(true);
                        }
                        let piece0 = b0.trim(parameter_interval[0], parameter_interval[1])?;
                        let second = curve_plane(
                            &piece0,
                            plane2,
                            Options {
                                max_boxes: options.max_boxes - report.boxes_visited,
                                ..options
                            },
                        )?;
                        report.boxes_visited += second.boxes_visited;
                        report.bernstein_excluded += second.bernstein_excluded;
                        for nested in second.components {
                            match nested {
                                CurvePlaneComponent::Point(point) => {
                                    certified.push(point.parameter_interval);
                                    line_points.push(point.parameter);
                                }
                                CurvePlaneComponent::Overlap { .. } => boundary_on_line = true,
                            }
                        }
                        for pending in second.unresolved {
                            joint_bands.push([pending.parameter_box[0], pending.parameter_box[1]]);
                        }
                    }
                }
            }
            for pending in roots.unresolved {
                bands.push([pending.parameter_box[0], pending.parameter_box[1]]);
            }
            let mut ambiguous = false;
            for band in bands {
                // A band provably staying off the second plane cannot hide a
                // ruling candidate; outward-rounded Bernstein bounds decide.
                let clear = if band[0] == band[1] {
                    let p = point3(&b0.evaluate(band[0])?.point);
                    plane2.distance(p).abs() > options.distance_tolerance
                } else {
                    let piece = b0.trim(band[0], band[1])?;
                    let values = coefficients(&piece, plane2);
                    values
                        .iter()
                        .all(|c| c.bound.lo > options.distance_tolerance)
                        || values
                            .iter()
                            .all(|c| c.bound.hi < -options.distance_tolerance)
                };
                if clear {
                    continue;
                }
                joint_bands.push(band);
            }
            // Remaining bands touching a certified root are its rounding halo;
            // isolated ones keep the coincidence ambiguous, never guessed.
            for band in joint_bands {
                if certified
                    .iter()
                    .any(|iv| iv[0] <= band[1] && band[0] <= iv[1])
                {
                    continue;
                }
                ambiguous = true;
            }
            // A line point of B0 is a ruling candidate only if the other
            // boundary point at the same U also sits on the support line.
            let mut candidates = Vec::new();
            for u in line_points {
                if off_line(point3(&b1.evaluate(u)?.point)) <= options.distance_tolerance {
                    candidates.push(u);
                }
            }
            candidates.dedup_by(|a, b| *a == *b);
            // An exactly closed seam makes the u_min and u_max rulings the
            // same curve: fold the u_max candidate into the canonical u_min
            // representative (exact parameter correspondence only).
            if let Some([s0, s1]) = seam_u
                && candidates.contains(&s0)
                && candidates.contains(&s1)
            {
                candidates.retain(|&u| u != s1);
            }
            if candidates.len() > 1 {
                report.unresolved(box6(), UnresolvedReason::NearCoincidence);
                return Ok(true);
            }
            if candidates.is_empty() && ambiguous {
                report.unresolved(box6(), UnresolvedReason::NearCoincidence);
                return Ok(true);
            }
            if let [u] = candidates[..] {
                if ambiguous {
                    report.unresolved(box6(), UnresolvedReason::NearCoincidence);
                    return Ok(true);
                }
                let r0 = point3(&b0.evaluate(u)?.point);
                let r1 = point3(&b1.evaluate(u)?.point);
                let ruling = sub(r1, r0);
                let dd = dot(ruling, ruling);
                let w0 = curve_weight_at(&b0, u)?;
                let w1 = curve_weight_at(&b1, u)?;
                if !dd.is_finite()
                    || dd <= 0.
                    || !w0.is_finite()
                    || w0 <= 0.
                    || !w1.is_finite()
                    || w1 <= 0.
                {
                    report.unresolved(box6(), UnresolvedReason::NearCoincidence);
                    return Ok(true);
                }
                // Möbius lift of a Cartesian line fraction s to ruling V:
                // s = λw1/((1-λ)w0+λw1) inverts to λ = s·w0/(w1-s(w1-w0)).
                let lift = |p: [f64; 3]| {
                    let s = dot(sub(p, r0), ruling) / dd;
                    let denominator = w1 - s * (w1 - w0);
                    if !denominator.is_finite() || denominator <= 0. {
                        return None;
                    }
                    let lambda = s * w0 / denominator;
                    (lambda.is_finite()).then(|| vd[0] + lambda * (vd[1] - vd[0]))
                };
                let mut lifts = Vec::new();
                for control in &piece.control_points {
                    let Some(v) = lift(point3(control)) else {
                        report.unresolved(box6(), UnresolvedReason::NearCoincidence);
                        return Ok(true);
                    };
                    lifts.push(v);
                }
                // The piece image lies inside its control polygon, so the whole
                // span stays on the surface exactly when every lift does.
                let vtol = options.parameter_tolerance;
                if lifts.iter().any(|v| *v < vd[0] - vtol || *v > vd[1] + vtol) {
                    report.unresolved(box6(), UnresolvedReason::CoincidentTrim);
                    return Ok(true);
                }
                let clamp = |v: f64| v.max(vd[0]).min(vd[1]);
                let uv_start = [u, clamp(lifts[0])];
                let uv_end = [u, clamp(*lifts.last().unwrap())];
                let mut max_residual = control_residual;
                for (t, uv) in [(ta[0], uv_start), (ta[1], uv_end)] {
                    let pc = point3(&piece.evaluate(t)?.point);
                    let ps = patch.evaluate(uv[0], uv[1])?.point;
                    let residual = distance(pc, ps);
                    if residual > options.distance_tolerance {
                        report.unresolved(box6(), UnresolvedReason::NearCoincidence);
                        return Ok(true);
                    }
                    max_residual = max_residual.max(residual);
                }
                // The Möbius t->v correspondence along the ruling, sampled at
                // the interval fractions 0, 1/2, 1: three samples determine
                // the cross-ratio map exactly.
                let mut samples = [[0.; 3]; 3];
                let mut sampled = true;
                for (i, s) in samples.iter_mut().enumerate() {
                    let t = ta[0] + (ta[1] - ta[0]) * (i as f64) * 0.5;
                    let p = point3(&piece.evaluate(t)?.point);
                    match lift(p) {
                        Some(v) => *s = [t, u, clamp(v)],
                        None => {
                            sampled = false;
                            break;
                        }
                    }
                }
                let correspondence = sampled.then_some(OverlapCorrespondence {
                    kind: OverlapCorrespondenceKind::MobiusV,
                    samples,
                });
                push_cs_overlap(
                    ta,
                    uv_start,
                    uv_end,
                    max_residual,
                    false,
                    correspondence,
                    report,
                );
                return Ok(true);
            }
        }
    }
    // Iso-V coincidence: the elevated homogeneous polygon must be a positive
    // proportional blend (1-λ)H0 + λH1 of the two boundary polygons.
    let degree = piece.degree.max(patch.degree_u);
    if degree > 25 {
        return Ok(false);
    }
    let ec = homogeneous4(&piece.elevate(degree)?);
    let e0 = homogeneous4(&b0.elevate(degree)?);
    let e1 = homogeneous4(&b1.elevate(degree)?);
    let weight_scale = ec.iter().map(|h| h[3].abs()).fold(0., f64::max);
    for reversed in [false, true] {
        let at = |row: &[[f64; 4]], i: usize| row[if reversed { degree - i } else { i }];
        let i0 = ec
            .iter()
            .enumerate()
            .max_by(|a, b| a.1[3].abs().total_cmp(&b.1[3].abs()))
            .map(|(i, _)| i)
            .unwrap();
        let (a, b, c) = (at(&e0, i0), at(&e1, i0), ec[i0]);
        let d = std::array::from_fn::<_, 4, _>(|k| b[k] - a[k]);
        // α·H0 + β·(H1-H0) = Hc is linear in (α, β); solve from the dominant
        // polygon row by 2x2 normal equations, then verify every row.
        let (aa, ad, dd) = (dot4(a, a), dot4(a, d), dot4(d, d));
        let determinant = aa * dd - ad * ad;
        if !determinant.is_finite() || determinant <= 0. {
            continue;
        }
        let alpha = (dd * dot4(a, c) - ad * dot4(d, c)) / determinant;
        let beta = (aa * dot4(d, c) - ad * dot4(a, c)) / determinant;
        if !alpha.is_finite() || alpha <= 0. || !beta.is_finite() {
            continue;
        }
        let lambda = beta / alpha;
        if !(-1e-9..=1. + 1e-9).contains(&lambda) {
            continue;
        }
        let mut weight_residual = 0_f64;
        let mut control_residual = 0_f64;
        let mut valid = true;
        for (i, c) in ec.iter().enumerate() {
            let (a, b) = (at(&e0, i), at(&e1, i));
            let h = std::array::from_fn::<_, 4, _>(|k| alpha * a[k] + beta * (b[k] - a[k]));
            if !h[3].is_finite() || h[3] <= 0. {
                valid = false;
                break;
            }
            weight_residual = weight_residual.max((h[3] - c[3]).abs() / weight_scale);
            let ph = std::array::from_fn(|k| h[k] / h[3]);
            let pc = std::array::from_fn(|k| c[k] / c[3]);
            control_residual = control_residual.max(distance(ph, pc));
        }
        if !valid || weight_residual > 1e-9 || control_residual > options.distance_tolerance {
            continue;
        }
        let v = vd[0] + lambda.clamp(0., 1.) * (vd[1] - vd[0]);
        let (uv_start, uv_end) = if reversed {
            ([ua[1], v], [ua[0], v])
        } else {
            ([ua[0], v], [ua[1], v])
        };
        let mut endpoint_residual = 0_f64;
        for (t, uv) in [(ta[0], uv_start), (ta[1], uv_end)] {
            let pc = point3(&piece.evaluate(t)?.point);
            let ps = patch.evaluate(uv[0], uv[1])?.point;
            let residual = distance(pc, ps);
            if residual > options.distance_tolerance {
                report.unresolved(box6(), UnresolvedReason::NearCoincidence);
                return Ok(true);
            }
            endpoint_residual = endpoint_residual.max(residual);
        }
        push_cs_overlap(
            ta,
            uv_start,
            uv_end,
            control_residual.max(endpoint_residual),
            false,
            // Affine t->u correspondence at constant v, sampled at the
            // interval fractions 0, 1/2, 1.
            Some(OverlapCorrespondence {
                kind: OverlapCorrespondenceKind::AffineU,
                samples: std::array::from_fn(|i| {
                    let f = (i as f64) * 0.5;
                    [
                        ta[0] + (ta[1] - ta[0]) * f,
                        uv_start[0] + (uv_end[0] - uv_start[0]) * f,
                        v,
                    ]
                }),
            }),
            report,
        );
        return Ok(true);
    }
    if boundary_on_line {
        report.unresolved(box6(), UnresolvedReason::CoincidentTrim);
        return Ok(true);
    }
    Ok(false)
}
fn dot4(a: [f64; 4], b: [f64; 4]) -> f64 {
    (0..4).map(|i| a[i] * b[i]).sum()
}

enum CsRefinement {
    Root(f64, [f64; 2], [f64; 3], f64),
    Outside,
    Failed,
}
/// Newton refinement of C(t)-S(u,v)=0 inside [ta]x[ua]x[vd]. Only iterates
/// that stay inside the isolating box are reported by that box.
#[allow(clippy::too_many_arguments)]
fn refine_cs_root(
    curve: &Curve,
    surface: &Surface,
    ta: [f64; 2],
    ua: [f64; 2],
    vd: [f64; 2],
    tm: f64,
    um: f64,
    vm: f64,
    options: Options,
) -> Result<CsRefinement> {
    let (mut t, mut u, mut v) = (tm, um, vm);
    let (wt, wu, wv) = (ta[1] - ta[0], ua[1] - ua[0], vd[1] - vd[0]);
    let inside = |t: f64, u: f64, v: f64| {
        ta[0] <= t && t <= ta[1] && ua[0] <= u && u <= ua[1] && vd[0] <= v && v <= vd[1]
    };
    for _ in 0..16 {
        if !inside(t, u, v) {
            return Ok(CsRefinement::Outside);
        }
        let jc = curve.evaluate(t)?;
        let js = surface.evaluate(u, v)?;
        let r = sub(point3(&jc.point), js.point);
        if distance(r, [0.; 3]) <= options.distance_tolerance * 1e-3 {
            break;
        }
        // C0 knots have no two-sided jets: step with one-sided span jets,
        // preferring the sides the iterate came from; any confirming side
        // combination is accepted.
        let mut c_sides = curve_tangents(curve, t)?;
        if c_sides.len() == 2 && tm > t {
            c_sides.swap(0, 1);
        }
        let (mut u_sides, mut v_sides) = surface_tangents(surface, u, v)?;
        if u_sides.len() == 2 && um > u {
            u_sides.swap(0, 1);
        }
        if v_sides.len() == 2 && vm > v {
            v_sides.swap(0, 1);
        }
        let mut step = None;
        'sides: for dc in &c_sides {
            for du in &u_sides {
                for dv in &v_sides {
                    let rows = [
                        [dc[0], -du[0], -dv[0]],
                        [dc[1], -du[1], -dv[1]],
                        [dc[2], -du[2], -dv[2]],
                    ];
                    if let Some(s) = solve3(rows, [-r[0], -r[1], -r[2]]) {
                        step = Some(s);
                        break 'sides;
                    }
                }
            }
        }
        let Some(step) = step else {
            return Ok(CsRefinement::Failed);
        };
        t += step[0];
        u += step[1];
        v += step[2];
        if step[0].abs() <= wt * 1e-6 && step[1].abs() <= wu * 1e-6 && step[2].abs() <= wv * 1e-6 {
            break;
        }
    }
    // Half-open ownership in t and u; V never subdivides, so both V faces
    // remain owned by the only V band.
    let t = snap_to_face(snap_to_face(t, ta[0]), ta[1]);
    let u = snap_to_face(snap_to_face(u, ua[0]), ua[1]);
    let v = snap_to_face(snap_to_face(v, vd[0]), vd[1]);
    let sd = surface_domain(surface);
    if !owns_parameter(ta[0], ta[1], curve.domain()[1], t)
        || !owns_parameter(ua[0], ua[1], sd[1], u)
        || !(vd[0] <= v && v <= vd[1])
    {
        return Ok(CsRefinement::Outside);
    }
    let jc = curve.evaluate(t)?;
    let js = surface.evaluate(u, v)?;
    let pc = point3(&jc.point);
    let ps = js.point;
    let residual = distance(pc, ps);
    if residual > options.distance_tolerance {
        return Ok(CsRefinement::Failed);
    }
    // One-sided jets screen a converged C0-knot root: it resolves when any
    // side combination is transverse and stays unresolved when none is.
    if cs_transversality(curve, surface, t, u, v)? <= TRANSVERSE_SINE {
        return Ok(CsRefinement::Failed);
    }
    Ok(CsRefinement::Root(
        t,
        [u, v],
        std::array::from_fn(|i| (pc[i] + ps[i]) * 0.5),
        residual,
    ))
}
/// Terminal curve/ruled-surface box resolution, mirroring the curve/curve
/// rule: provable separation by hull diagonals, ambiguous bands stay
/// unresolved, tangencies are never collapsed to guessed points.
#[allow(clippy::too_many_arguments)]
fn resolve_cs_box(
    curve: &Curve,
    surface: &Surface,
    ta: [f64; 2],
    ua: [f64; 2],
    vd: [f64; 2],
    hc: &[[f64; 4]],
    hs: &HomogeneousGrid,
    options: Options,
    report: &mut Report<CurveRuledSurfaceComponent>,
) -> Result<()> {
    let box6 = || vec![ta[0], ta[1], ua[0], ua[1], vd[0], vd[1]];
    let diagonal = |ranges: [[f64; 2]; 3]| {
        ranges
            .iter()
            .map(|r| {
                let w = r[1] - r[0];
                w * w
            })
            .sum::<f64>()
            .sqrt()
    };
    let diag = diagonal(hull_ranges(hc)) + diagonal(grid_ranges(hs));
    let tm = ta[0] + (ta[1] - ta[0]) * 0.5;
    let um = ua[0] + (ua[1] - ua[0]) * 0.5;
    let vm = vd[0] + (vd[1] - vd[0]) * 0.5;
    let pc = point3(&curve.evaluate(tm)?.point);
    let ps = surface.evaluate(um, vm)?.point;
    let residual = distance(pc, ps);
    // |C(t)-S(u,v)| >= residual - diag everywhere in the box (triangle bound).
    if residual - diag > options.distance_tolerance {
        return Ok(());
    }
    // V never subdivides on a ruling, so the midpoint's V coordinate is not
    // converged: a large midpoint residual does not disprove an in-box root.
    // Newton runs either way; only a converged, in-box, low-residual,
    // transverse iterate is reported by this box.
    if residual <= options.distance_tolerance
        && cs_transversality(curve, surface, tm, um, vm)? <= TRANSVERSE_SINE
    {
        report.unresolved(box6(), UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(());
    }
    let near_midpoint = residual <= options.distance_tolerance;
    match refine_cs_root(curve, surface, ta, ua, vd, tm, um, vm, options)? {
        CsRefinement::Root(t, uv, point, residual) => {
            let td = curve.domain();
            let sd = surface_domain(surface);
            let boundary = t == td[0]
                || t == td[1]
                || curve.knots.contains(&t)
                || uv[0] == sd[0]
                || uv[0] == sd[1]
                || surface.knots_u.contains(&uv[0])
                || uv[1] == sd[2]
                || uv[1] == sd[3];
            push_cs_point(
                report,
                CurveSurfacePointEvent {
                    t,
                    t_interval: ta,
                    uv,
                    uv_box: [ua[0], ua[1], vd[0], vd[1]],
                    point,
                    residual,
                    contact: if boundary {
                        Contact::Boundary
                    } else {
                        Contact::Transverse
                    },
                },
            );
        }
        CsRefinement::Outside => (),
        CsRefinement::Failed => {
            // A box whose midpoint was already a contact candidate fails as a
            // tangency; a far midpoint that Newton could not bring in-box is a
            // near-surface band. Both stay explicit.
            report.unresolved(
                box6(),
                if near_midpoint {
                    UnresolvedReason::TangencyOrMultipleRoot
                } else {
                    UnresolvedReason::NearCoincidence
                },
            )
        }
    }
    Ok(())
}
/// Merge point events whose isolating intervals touch in the (t,u) plane.
/// Subdivision tiles exactly, so adjacency uses interval endpoints only.
fn merge_cs_points(report: &mut Report<CurveRuledSurfaceComponent>) {
    let intervals = |c: &CurveRuledSurfaceComponent| match c {
        CurveRuledSurfaceComponent::Point {
            t_interval, uv_box, ..
        } => Some((*t_interval, [uv_box[0], uv_box[1]])),
        _ => None,
    };
    let n = report.components.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for i in 0..n {
        for j in i + 1..n {
            let (Some((at, au)), Some((bt, bu))) = (
                intervals(&report.components[i]),
                intervals(&report.components[j]),
            ) else {
                continue;
            };
            if at[0] <= bt[1] && bt[0] <= at[1] && au[0] <= bu[1] && bu[0] <= au[1] {
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                if ri != rj {
                    parent[ri] = rj;
                }
            }
        }
    }
    let mut merged: Vec<Option<CurveRuledSurfaceComponent>> = (0..n).map(|_| None).collect();
    let mut order: Vec<usize> = Vec::new();
    for (i, component) in report.components.iter().take(n).enumerate() {
        let Some((ti, ui)) = intervals(component) else {
            continue;
        };
        let root = find(&mut parent, i);
        if merged[root].is_none() {
            order.push(root);
        }
        let CurveRuledSurfaceComponent::Point {
            t,
            uv,
            uv_box,
            point,
            residual,
            contact,
            ..
        } = component
        else {
            continue;
        };
        let member = (*t, *uv, *point, *residual, *contact, (ti, ui), *uv_box);
        match &mut merged[root] {
            None => {
                merged[root] = Some(CurveRuledSurfaceComponent::Point {
                    t: member.0,
                    t_interval: ti,
                    uv: member.1,
                    uv_box: member.6,
                    point: member.2,
                    residual: member.3,
                    contact: member.4,
                });
            }
            Some(CurveRuledSurfaceComponent::Point {
                t,
                t_interval,
                uv,
                uv_box,
                point,
                residual,
                contact,
            }) => {
                if member.3 < *residual {
                    *t = member.0;
                    *uv = member.1;
                    *point = member.2;
                    *residual = member.3;
                }
                if member.4 == Contact::Boundary {
                    *contact = Contact::Boundary;
                }
                t_interval[0] = t_interval[0].min(member.5.0[0]);
                t_interval[1] = t_interval[1].max(member.5.0[1]);
                uv_box[0] = uv_box[0].min(member.6[0]);
                uv_box[1] = uv_box[1].max(member.6[1]);
            }
            _ => unreachable!(),
        }
    }
    let mut components = Vec::new();
    let mut index = 0;
    for root in order {
        while index < root {
            if intervals(&report.components[index]).is_none() {
                components.push(report.components[index].clone());
            }
            index += 1;
        }
        if let Some(c) = merged[root].take() {
            components.push(c);
        }
        index = root + 1;
    }
    while index < n {
        if intervals(&report.components[index]).is_none() {
            components.push(report.components[index].clone());
        }
        index += 1;
    }
    report.components = components;
}

/// Seam unification on an exactly closed ruled U direction. Point events at
/// the two seam sides merge only when their curve-parameter isolating
/// intervals touch and their V boxes overlap — parameter-based, never a
/// spatial tolerance. The canonical representative sits at u = u_min and
/// keeps the better-residual witness. Overlaps unify when their curve
/// intervals touch and their UV paths meet exactly at the seam: a ruling
/// pair at u_min/u_max folds to the u_min copy (identical lifted V
/// endpoints), and an iso-V chain leaving at u_max re-enters at u_min with
/// the wrap marked; merged pieces keep one correspondence map only when a
/// single map spans the whole merged interval.
fn unify_cs_seam(report: &mut Report<CurveRuledSurfaceComponent>, su: [f64; 2]) {
    let point_key = |c: &CurveRuledSurfaceComponent| match c {
        CurveRuledSurfaceComponent::Point {
            t_interval, uv_box, ..
        } => Some((*t_interval, *uv_box)),
        _ => None,
    };
    let mut dropped: Vec<usize> = Vec::new();
    let n = report.components.len();
    for i in 0..n {
        if dropped.contains(&i) {
            continue;
        }
        let Some((at, abox)) = point_key(&report.components[i]) else {
            continue;
        };
        if abox[0] != su[0] {
            continue;
        }
        for j in i + 1..n {
            if dropped.contains(&j) {
                continue;
            }
            let Some((bt, bbox)) = point_key(&report.components[j]) else {
                continue;
            };
            if bbox[1] != su[1] {
                continue;
            }
            let touch = at[0] <= bt[1] && bt[0] <= at[1];
            let v_overlap = abox[2] <= bbox[3] && bbox[2] <= abox[3];
            if !touch || !v_overlap {
                continue;
            }
            let (a, b) = {
                let (head, tail) = report.components.split_at_mut(j);
                (&mut head[i], &tail[0])
            };
            let (
                CurveRuledSurfaceComponent::Point {
                    t,
                    t_interval,
                    uv,
                    uv_box,
                    point,
                    residual,
                    contact,
                },
                CurveRuledSurfaceComponent::Point {
                    t: bt2,
                    t_interval: bt_interval,
                    uv: buv,
                    uv_box: buv_box,
                    point: bpoint,
                    residual: bresidual,
                    contact: bcontact,
                },
            ) = (a, b)
            else {
                unreachable!()
            };
            if *bresidual < *residual {
                *t = *bt2;
                uv[1] = buv[1];
                *point = *bpoint;
                *residual = *bresidual;
            }
            if *bcontact == Contact::Boundary {
                *contact = Contact::Boundary;
            }
            uv[0] = su[0];
            t_interval[0] = t_interval[0].min(bt_interval[0]);
            t_interval[1] = t_interval[1].max(bt_interval[1]);
            uv_box[0] = su[0];
            uv_box[2] = uv_box[2].min(buv_box[2]);
            uv_box[3] = uv_box[3].max(buv_box[3]);
            dropped.push(j);
            break;
        }
    }
    // Overlap unification across the seam.
    let overlap_key = |c: &CurveRuledSurfaceComponent| match c {
        CurveRuledSurfaceComponent::Overlap {
            curve_interval,
            uv_start,
            uv_end,
            ..
        } => Some((*curve_interval, *uv_start, *uv_end)),
        _ => None,
    };
    loop {
        let mut merged = false;
        'outer: for i in 0..report.components.len() {
            if dropped.contains(&i) {
                continue;
            }
            for j in 0..report.components.len() {
                if j == i || dropped.contains(&j) {
                    continue;
                }
                let (Some((aci, aus, aue)), Some((bci, bus, bue))) = (
                    overlap_key(&report.components[i]),
                    overlap_key(&report.components[j]),
                ) else {
                    continue;
                };
                // Ruling pair: same lifted V endpoints, touching intervals.
                let ruling_pair = aus[0] == aue[0]
                    && aus[0] == su[0]
                    && bus[0] == bue[0]
                    && bus[0] == su[1]
                    && aus[1] == bus[1]
                    && aue[1] == bue[1]
                    && aci[0] <= bci[1]
                    && bci[0] <= aci[1];
                // Iso-V chain crossing the seam: A ends at u_max exactly where
                // B begins at u_min, with touching curve intervals.
                let iso_cross = aue[0] == su[1]
                    && bus[0] == su[0]
                    && aue[1] == bus[1]
                    && aus[1] == aue[1]
                    && bus[1] == bue[1]
                    && aci[1] == bci[0];
                if !ruling_pair && !iso_cross {
                    continue;
                }
                let same_interval = aci == bci;
                let (lo, hi) = if i < j { (i, j) } else { (j, i) };
                let (head, tail) = report.components.split_at_mut(hi);
                let (first, second) = (&mut head[lo], &mut tail[0]);
                let (a, b) = if i < j {
                    (first, second)
                } else {
                    (second, first)
                };
                let CurveRuledSurfaceComponent::Overlap {
                    curve_interval,
                    uv_end,
                    max_control_residual,
                    seam_wrap,
                    correspondence,
                    ..
                } = a
                else {
                    unreachable!()
                };
                let CurveRuledSurfaceComponent::Overlap {
                    max_control_residual: bres,
                    correspondence: bcorr,
                    ..
                } = b
                else {
                    unreachable!()
                };
                let bres = *bres;
                let bcorr = bcorr.clone();
                if iso_cross {
                    *uv_end = bue;
                    *correspondence = None;
                } else if !same_interval {
                    *correspondence = None;
                } else if correspondence.is_none() {
                    *correspondence = bcorr;
                }
                curve_interval[0] = curve_interval[0].min(bci[0]);
                curve_interval[1] = curve_interval[1].max(bci[1]);
                *max_control_residual = max_control_residual.max(bres);
                *seam_wrap = true;
                dropped.push(j);
                merged = true;
                break 'outer;
            }
        }
        if !merged {
            break;
        }
    }
    if !dropped.is_empty() {
        let mut index = 0;
        report.components.retain(|_| {
            let keep = !dropped.contains(&index);
            index += 1;
            keep
        });
    }
}

/// Intersect a retained positive-weight 3D NURBS curve with a ruled NURBS
/// surface: rational in one direction (any degree, multiple knot spans) and
/// linear with positive, possibly unequal endpoint weights in the other.
/// Curve spans and surface U-spans decompose into homogeneous rational Bezier
/// pieces traversed by one FIFO queue; outward-rounded control hulls exclude
/// separated boxes. Terminal boxes confirm C(t)=S(u,v) by 3x3 Newton with the
/// root required inside its own box; exact parameter corners own boundary
/// contacts (dedup by exact (t,u,v) only). Ruling and iso-V coincidences lift
/// to explicit UV path endpoints with sampled per-parameter correspondence
/// (Möbius in V along a ruling, affine in U along an iso-V); exactly closed
/// U seams unify seam-side contacts and wrap overlaps canonically at u_min,
/// while near-closed seams stay duplicate behind explicit near_coincidence
/// bands; tangencies, near-surface bands, partial ruling trims and exhausted
/// budgets stay unresolved. Reports never authorize topology changes.
pub fn curve_ruled_surface(
    curve: &Curve,
    surface: &Surface,
    options: Options,
) -> Result<Report<CurveRuledSurfaceComponent>> {
    curve.validate()?;
    if curve.control_points[0].len() != 3 {
        return Err(invalid("Curve/ruled-surface requires a 3D curve"));
    }
    surface.validate()?;
    let options = options.validate()?;
    let mut report = Report::default();
    let unsupported = |report: &mut Report<CurveRuledSurfaceComponent>| {
        let d = surface_domain(surface);
        report.unresolved(
            vec![curve.domain()[0], curve.domain()[1], d[0], d[1], d[2], d[3]],
            UnresolvedReason::UnsupportedSurface,
        );
    };
    // Canonical orientation is ruled in V: linear V with exactly two rows.
    let swapped =
        if surface.degree_v == 1 && surface.control_points[0].len() == 2 && !surface.periodic_v {
            false
        } else if surface.degree_u == 1 && surface.control_points.len() == 2 && !surface.periodic_u
        {
            true
        } else {
            unsupported(&mut report);
            return Ok(report);
        };
    let canonical = if swapped {
        transpose_surface(surface)
    } else {
        surface.clone()
    };
    let domain = surface_domain(&canonical);
    let vd = [domain[2], domain[3]];
    // Geometric U-seam closure of the canonical orientation: an exactly
    // proportional seam unifies contacts across u_min/u_max; a near-closed
    // seam stays duplicate and both seam bands are explicit unresolved
    // regions (merging is never by tolerance).
    let seam = ruled_seam_state(&canonical);
    let seam_u = (seam == RuledSeam::Closed).then_some([domain[0], domain[1]]);
    if seam == RuledSeam::Uncertain {
        let td = curve.domain();
        for &u in &domain[..2] {
            report.unresolved(
                vec![td[0], td[1], u, u, vd[0], vd[1]],
                UnresolvedReason::NearCoincidence,
            );
        }
    }
    type CurveSurfacePending = (
        [f64; 2],
        [f64; 2],
        Option<Vec<[f64; 4]>>,
        Option<HomogeneousGrid>,
        usize,
    );
    let mut pending: std::collections::VecDeque<CurveSurfacePending> =
        spans(&curve.knots, curve.degree, curve.control_points.len())
            .into_iter()
            .flat_map(|ta| {
                spans(
                    &canonical.knots_u,
                    canonical.degree_u,
                    canonical.control_points.len(),
                )
                .into_iter()
                .map(move |ua| (ta, ua, None, None, 0))
            })
            .collect();
    while let Some((ta, ua, hc, hs, depth)) = pending.pop_front() {
        if report.boxes_visited >= options.max_boxes {
            report.unresolved(
                vec![ta[0], ta[1], ua[0], ua[1], vd[0], vd[1]],
                UnresolvedReason::BudgetExceeded,
            );
            continue;
        }
        report.boxes_visited += 1;
        let (pieces, hc, hs) = match (hc, hs) {
            (Some(hc), Some(hs)) => (None, hc, hs),
            _ => {
                let pc = curve.trim(ta[0], ta[1])?;
                let ps = canonical.trim([ua[0], ua[1], vd[0], vd[1]])?;
                let (hc, hs) = (homogeneous4(&pc), homogeneous_grid(&ps));
                (Some((pc, ps)), hc, hs)
            }
        };
        if let Some((pc, ps)) = &pieces {
            for &t in &ta {
                for &u in &ua {
                    for &v in &vd {
                        admit_cs_corner(curve, &canonical, t, u, v, options, &mut report)?;
                    }
                }
            }
            if !grids_excluded(&hc, &hs)
                && ruled_coincidence(pc, ps, ta, ua, vd, seam_u, options, &mut report)?
            {
                continue;
            }
        }
        if grids_excluded(&hc, &hs) {
            report.bernstein_excluded += 1;
            continue;
        }
        let width_t = ta[1] - ta[0];
        let width_u = ua[1] - ua[0];
        let tm = ta[0] + width_t * 0.5;
        let um = ua[0] + width_u * 0.5;
        let can_t = tm > ta[0] && tm < ta[1];
        let can_u = um > ua[0] && um < ua[1];
        if (width_t <= options.parameter_tolerance && width_u <= options.parameter_tolerance)
            || depth == options.max_depth
            || (!can_t && !can_u)
        {
            resolve_cs_box(
                curve,
                &canonical,
                ta,
                ua,
                vd,
                &hc,
                &hs,
                options,
                &mut report,
            )?;
            continue;
        }
        let c_side: Vec<([f64; 2], Vec<[f64; 4]>)> = if can_t {
            let (l, r) = split_homogeneous(&hc);
            vec![([ta[0], tm], l), ([tm, ta[1]], r)]
        } else {
            vec![(ta, hc.clone())]
        };
        let s_side: Vec<([f64; 2], HomogeneousGrid)> = if can_u {
            let (l, r) = split_grid_u(&hs);
            vec![([ua[0], um], l), ([um, ua[1]], r)]
        } else {
            vec![(ua, hs.clone())]
        };
        for (ta2, h) in &c_side {
            for (ua2, g) in &s_side {
                pending.push_back((*ta2, *ua2, Some(h.clone()), Some(g.clone()), depth + 1));
            }
        }
    }
    merge_cs_points(&mut report);
    if let Some(su) = seam_u {
        unify_cs_seam(&mut report, su);
    }
    report.components.sort_by(|a, b| {
        let key = |c: &CurveRuledSurfaceComponent| match c {
            CurveRuledSurfaceComponent::Point { t, uv, .. } => (*t, uv[0], uv[1]),
            CurveRuledSurfaceComponent::Overlap {
                curve_interval,
                uv_start,
                ..
            } => (curve_interval[0], uv_start[0], uv_start[1]),
        };
        let (x, y) = (key(a), key(b));
        x.0.total_cmp(&y.0)
            .then(x.1.total_cmp(&y.1))
            .then(x.2.total_cmp(&y.2))
    });
    if swapped {
        for pending in &mut report.unresolved {
            pending.parameter_box.swap(2, 4);
            pending.parameter_box.swap(3, 5);
        }
        for component in &mut report.components {
            match component {
                CurveRuledSurfaceComponent::Point { uv, uv_box, .. } => {
                    uv.swap(0, 1);
                    uv_box.swap(0, 2);
                    uv_box.swap(1, 3);
                }
                CurveRuledSurfaceComponent::Overlap {
                    uv_start, uv_end, ..
                } => {
                    uv_start.swap(0, 1);
                    uv_end.swap(0, 1);
                }
            }
        }
    }
    Ok(report)
}

#[inline(always)]
fn point3(p: &[f64]) -> [f64; 3] {
    [p[0], p[1], p[2]]
}
#[inline(always)]
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
#[inline(always)]
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
#[inline(always)]
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
#[path = "intersections/tests/tests_integration.rs"]
mod tests;

#[doc(hidden)]
pub use plane_sphere::{CanonicalPlane, recognize_plane};
#[doc(hidden)]
pub use plane_torus::CanonicalTorus;
