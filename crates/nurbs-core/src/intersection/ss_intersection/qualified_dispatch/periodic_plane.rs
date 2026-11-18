//! Complete periodic homothetic section against an original finite affine plane.
//! Seam/Jordan/quotient authority comes from the original extrusion proof.
//! All level coefficients stay exact rationals, including nonbinary UV clocks.
use super::*;
use crate::surface_injectivity::{PeriodicExtrusionReport, certify_periodic_extrusion};
use num_rational::BigRational as Q;
use num_traits::{One, ToPrimitive, Zero};
#[derive(Clone, Copy, Debug)]
pub struct PeriodicSSBuildLimits {
    pub max_pairs: usize,
    pub max_cells: usize,
}
impl Default for PeriodicSSBuildLimits {
    fn default() -> Self {
        Self {
            max_pairs: 10000,
            max_cells: 100000,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicPlaneSSKind {
    ClosedContour,
    CollapsedPoleContact,
    Disjoint,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Data {
    surface: usize,
    axis: usize,
    normal: usize,
    height: Q,
    kind: PeriodicPlaneSSKind,
    parameter: Option<Q>,
    poles: Vec<[Q; 3]>,
    weights: Vec<Q>,
    pole: Option<[Q; 3]>,
}
#[derive(Clone, Debug)]
pub struct QualifiedPeriodicPlaneSS {
    source: FiberSources,
    limits: PeriodicSSBuildLimits,
    data: Data,
    support: PeriodicExtrusionReport,
    cells: usize,
}
fn q(v: f64) -> Result<Q> {
    Q::from_float(v).ok_or_else(|| crate::input("Nonfinite periodic section coefficient"))
}
fn rectangle(s: &Surface) -> Result<Option<(usize, Q, [[Q; 2]; 2])>> {
    if s.periodic_u
        || s.periodic_v
        || s.degree_u != 1
        || s.degree_v != 1
        || s.control_points.len() != 2
        || s.control_points[0].len() != 2
        || s.control_points.iter().flatten().any(|p| p.len() != 3)
        || s.weights.iter().flatten().any(|w| *w != s.weights[0][0])
    {
        return Ok(None);
    }
    if [&s.knots_u, &s.knots_v]
        .iter()
        .any(|k| k.len() != 4 || k[0] != k[1] || k[2] != k[3])
    {
        return Ok(None);
    }
    for normal in 0..3 {
        let height = q(s.control_points[0][0][normal])?;
        if s.control_points
            .iter()
            .flatten()
            .any(|p| p[normal] != s.control_points[0][0][normal])
        {
            continue;
        }
        let axes: Vec<_> = (0..3).filter(|k| *k != normal).collect();
        for swap in [false, true] {
            let (u, v) = (axes[usize::from(swap)], axes[usize::from(!swap)]);
            let x = [q(s.control_points[0][0][u])?, q(s.control_points[1][0][u])?];
            let y = [q(s.control_points[0][0][v])?, q(s.control_points[0][1][v])?];
            if x[0] == x[1] || y[0] == y[1] {
                continue;
            }
            let mut matches = true;
            for i in 0..2 {
                for j in 0..2 {
                    matches &= q(s.control_points[i][j][u])? == x[i]
                        && q(s.control_points[i][j][v])? == y[j];
                }
            }
            if matches {
                let sorted = |a: [Q; 2]| {
                    if a[0] < a[1] {
                        a
                    } else {
                        [a[1].clone(), a[0].clone()]
                    }
                };
                let mut ranges = [sorted(x), sorted(y)];
                if swap {
                    ranges.swap(0, 1);
                }
                return Ok(Some((normal, height, ranges)));
            }
        }
    }
    Ok(None)
}
pub(super) fn attempt(
    source: &FiberSources,
    limits: PeriodicSSBuildLimits,
) -> Result<Option<QualifiedPeriodicPlaneSS>> {
    check(
        (1..=100000).contains(&limits.max_pairs) && (1..=100000).contains(&limits.max_cells),
        "Invalid periodic section work limits",
    )?;
    let surfaces = [source.first(), source.second()];
    let periodic: Vec<_> = surfaces
        .iter()
        .enumerate()
        .filter(|(_, s)| s.periodic_u || s.periodic_v)
        .map(|(i, _)| i)
        .collect();
    if periodic.len() != 1 {
        return Ok(None);
    }
    let index = periodic[0];
    let s = surfaces[index];
    if s.periodic_u && s.periodic_v {
        return Ok(None);
    }
    let axis = usize::from(s.periodic_v);
    let Some((normal, height, ranges)) = rectangle(source.evaluation_surfaces()[1 - index])? else {
        return Ok(None);
    };
    let proof = certify_periodic_extrusion(s, axis, limits.max_pairs, limits.max_cells)?;
    if proof.reason == "work-limit"
        || (!(proof.proven && proof.regular_interior)
            && (proof.pairs >= limits.max_pairs || proof.cells >= limits.max_cells))
    {
        return Err(crate::resource(
            "Original periodic quotient proof budget exhausted",
        ));
    }
    check(
        proof.proven && proof.regular_interior,
        "Original periodic seam, Jordan profile or immersion remains unproved",
    )?;
    if proof.axial_coordinate != Some(normal) {
        return Ok(None);
    }
    let count = if axis == 0 {
        s.control_points.len()
    } else {
        s.control_points[0].len()
    };
    let point = |i: usize, j: usize| {
        if axis == 0 {
            &s.control_points[i][j]
        } else {
            &s.control_points[j][i]
        }
    };
    let weight = |i: usize, j: usize| {
        if axis == 0 {
            s.weights[i][j]
        } else {
            s.weights[j][i]
        }
    };
    let z = [q(point(0, 0)[normal])?, q(point(0, 1)[normal])?];
    check(z[0] != z[1], "Periodic section has no transverse height")?;
    for i in 0..count {
        for j in 0..2 {
            check(
                q(point(i, j)[normal])? == z[j],
                "Original periodic height identity failed",
            )?;
        }
    }
    let mut data = Data {
        surface: index,
        axis,
        normal,
        height: height.clone(),
        kind: PeriodicPlaneSSKind::Disjoint,
        parameter: None,
        poles: vec![],
        weights: vec![],
        pole: None,
    };
    let mut cells = proof.cells;
    if height >= z[0].clone().min(z[1].clone()) && height <= z[0].clone().max(z[1].clone()) {
        let w = [q(weight(0, 0))?, q(weight(0, 1))?];
        let num = &w[0] * (&height - &z[0]);
        let t = &num / (&num + &w[1] * (&z[1] - &height));
        check(
            t >= Q::zero() && t <= Q::one(),
            "Exact periodic section clock left original source",
        )?;
        let other = if axis == 0 {
            (&s.knots_v, s.degree_v, s.control_points[0].len())
        } else {
            (&s.knots_u, s.degree_u, s.control_points.len())
        };
        data.parameter =
            Some(q(other.0[other.1])? + (q(other.0[other.2])? - q(other.0[other.1])?) * &t);
        let axes: Vec<_> = (0..3).filter(|k| *k != normal).collect();
        for i in 0..count {
            if cells == limits.max_cells {
                return Err(crate::resource(
                    "Exact periodic level coefficient budget exhausted",
                ));
            }
            cells += 1;
            let wi = [q(weight(i, 0))?, q(weight(i, 1))?];
            check(
                &wi[0] * &w[1] == &wi[1] * &w[0],
                "Original periodic row gauge identity failed",
            )?;
            let a = (Q::one() - &t) * &wi[0];
            let b = &t * &wi[1];
            let denominator = &a + &b;
            let mut pole = [Q::zero(), Q::zero(), Q::zero()];
            for k in 0..3 {
                pole[k] = (&a * q(point(i, 0)[k])? + &b * q(point(i, 1)[k])?) / &denominator;
            }
            check(
                pole[normal] == height,
                "Original exact level left target plane",
            )?;
            for (j, k) in axes.iter().enumerate() {
                check(
                    ranges[j][0] < pole[*k] && pole[*k] < ranges[j][1],
                    "Finite plane does not strictly contain the whole exact periodic level",
                )?;
            }
            data.poles.push(pole);
            data.weights.push(denominator);
        }
        let end = if t.is_zero() {
            Some(0)
        } else if t == Q::one() {
            Some(1)
        } else {
            None
        };
        if end.is_some() && end == proof.collapsed_end {
            check(
                data.poles.iter().all(|p| *p == data.poles[0]),
                "Declared polar shore is not one exact world point",
            )?;
            data.kind = PeriodicPlaneSSKind::CollapsedPoleContact;
            data.pole = Some(data.poles[0].clone());
        } else {
            data.kind = PeriodicPlaneSSKind::ClosedContour;
        }
    }
    Ok(Some(QualifiedPeriodicPlaneSS {
        source: source.clone(),
        limits,
        data,
        support: proof,
        cells,
    }))
}
/// Complete original periodic homothetic surface / finite coordinate-plane
/// intersection. This does not clip a partially contained contour or admit Body.
pub fn qualify_periodic_plane_ss(
    source: &FiberSources,
    limits: PeriodicSSBuildLimits,
) -> Result<QualifiedPeriodicPlaneSS> {
    attempt(source, limits)?
        .ok_or_else(|| crate::input("Unsupported original periodic plane section family"))
}
impl QualifiedPeriodicPlaneSS {
    pub fn source(&self) -> &FiberSources {
        &self.source
    }
    pub fn kind(&self) -> PeriodicPlaneSSKind {
        self.data.kind
    }
    pub fn periodic_surface(&self) -> usize {
        self.data.surface
    }
    pub fn periodic_axis(&self) -> usize {
        self.data.axis
    }
    pub fn transverse_parameter(&self) -> Option<&Q> {
        self.data.parameter.as_ref()
    }
    /// Exact rational coefficients on the unchanged original periodic basis.
    /// A polar contact is one point, never a zero-length edge admitted as a loop.
    pub fn level_control_points(&self) -> &[[Q; 3]] {
        &self.data.poles
    }
    pub fn level_weights(&self) -> &[Q] {
        &self.data.weights
    }
    pub fn pole_point(&self) -> Option<&[Q; 3]> {
        self.data.pole.as_ref()
    }
    pub fn support_proof(&self) -> &PeriodicExtrusionReport {
        &self.support
    }
    pub fn cells(&self) -> usize {
        self.cells
    }
    pub fn covers_entire_surfaces(&self) -> bool {
        true
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = qualify_periodic_plane_ss(&self.source, self.limits)?;
        check(
            self.data == fresh.data
                && self.cells == fresh.cells
                && self.support.proven
                && self.support.regular_interior,
            "Original periodic section ownership changed",
        )
    }

    /// Lift a regular complete section into exact shared world/UV edges.
    /// Each edge covers a bounded portion of one period of the original chart.
    /// Requires an interior transverse parameter for strict fiber inclusion.
    pub fn to_root_wire(
        &self,
        limits: OriginalSsCurveLimits,
        max_chart_attempts: usize,
    ) -> Result<OriginalSsRootWire> {
        check(
            self.kind() == PeriodicPlaneSSKind::ClosedContour && max_chart_attempts > 0,
            "A regular closed periodic section and positive chart budget are required",
        )?;
        let surfaces = [self.source.first(), self.source.second()];
        let surface = surfaces[self.data.surface];
        let knots = [&surface.knots_u, &surface.knots_v][self.data.axis];
        let free = 2 * self.data.surface + self.data.axis;
        let axis = [
            FiberAxis::FirstU,
            FiberAxis::FirstV,
            FiberAxis::SecondU,
            FiberAxis::SecondV,
        ][free];
        let [a, b] = surfaces.map(crate::analysis::surface_boundary::domains);
        let domain = [a[0], a[1], b[0], b[1]];
        let [lo, hi] = domain[free];
        let period = hi - lo;
        let transverse = 2 * self.data.surface + (1 - self.data.axis);
        let parameter = self
            .transverse_parameter()
            .ok_or_else(|| crate::input("Missing periodic section parameter"))?;
        let center = parameter
            .to_f64()
            .ok_or_else(|| crate::input("Periodic section parameter has no finite enclosure"))?;
        let [shore_lo, shore_hi] = domain[transverse];
        let radius = (center - shore_lo).min(shore_hi - center) / 4.;
        let transverse_window = [center - radius, center + radius];
        check(
            q(transverse_window[0])? < *parameter && *parameter < q(transverse_window[1])?,
            "Periodic section has no interior transverse chart",
        )?;
        let mut pending: Vec<_> = knots
            .windows(2)
            .filter(|k| lo <= k[0] && k[0] < k[1] && k[1] <= hi)
            .map(|k| [k[0], k[1]])
            .rev()
            .collect();
        let mut charts = Vec::new();
        let mut attempts = 0;
        while let Some([lo, hi]) = pending.pop() {
            if attempts == max_chart_attempts {
                return Err(crate::resource(
                    "Periodic section wire chart budget exhausted",
                ));
            }
            attempts += 1;
            let mut window = domain;
            window[transverse] = transverse_window;
            let padding = (hi - lo) / 32.;
            window[free] = [lo - padding, hi + padding];
            if window[free][1] - window[free][0] < period {
                let result = self.source.certify_quotient(
                    window,
                    axis,
                    FiberLimits {
                        max_rectangles: limits.native_rectangles,
                        preconditioner: None,
                    },
                )?;
                if let Some(fiber) = result.into_fiber() {
                    charts.push((lo, Arc::new(OriginalSsRootCurve::from_quotient(fiber))));
                    continue;
                }
            }
            let mid = lo.midpoint(hi);
            check(
                lo < mid && mid < hi,
                "Periodic section chart cannot be refined",
            )?;
            pending.extend([[mid, hi], [lo, mid]]);
        }
        let points = charts
            .iter()
            .map(|(lo, curve)| OriginalSsRootPoint::new(curve.clone(), *lo).map(Arc::new))
            .collect::<Result<Vec<_>>>()?;
        let edges = charts
            .into_iter()
            .enumerate()
            .map(|(i, (_, curve))| {
                let start =
                    bind_original_ss_port(points[i].clone(), curve.clone(), [0; 4], limits)?;
                let end = QualifiedOriginalSsPort::bind_with_inferred_phase(
                    points[(i + 1) % points.len()].clone(),
                    curve,
                    limits,
                )?;
                let mut expected_phase = [0; 4];
                expected_phase[free] = i32::from(i + 1 == points.len());
                check(
                    end.phase() == expected_phase,
                    "Periodic section edge has wrong phase",
                )?;
                OriginalSsRootEdge::new(start, end)
            })
            .collect::<Result<Vec<_>>>()?;
        OriginalSsRootWire::new(edges)
    }
}
