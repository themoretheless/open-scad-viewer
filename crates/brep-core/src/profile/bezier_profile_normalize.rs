//! Even-odd arrangement of authored spans. Native intersections split original
//! carriers; winding probes separated from every control hull choose material.
use super::arrangement_ops as arrangement;
use super::{Result, controls, injective, point_inside, refused};
use crate::Curve;
use nurbs_core::intersection::{CurveCurveComponentKind, intersect_curve_curve_report};

/// Authored boundaries admitted in one normalization.
const MAX_WIRES: usize = 32;
/// Roundoff multiple admitted when conforming trim endpoints and jets.
const JET_ROUNDOFF: f64 = 512.;
/// Side-probe distances, in tolerances; both must agree.
const PROBE_DISTANCES: [f64; 2] = [32., 64.];
/// Subdivision work allowed while separating side probes from other hulls.
const PROBE_WORK_BUDGET: usize = 100_000;
/// Subdivision depth allowed while separating a side probe from one hull.
const PROBE_SPLIT_DEPTH: usize = 40;
/// Roundoff multiple of the tangent-coordinate monotonicity test.
const TANGENT_ROUNDOFF: f64 = 256.;
/// Roundoff multiple of the probe/hull separation test.
const PROBE_ROUNDOFF: f64 = 512.;

/// Upward-rounded L1 distance between the first two coordinates.
fn l1_upper(p: &[f64], q: &[f64]) -> f64 {
    ((p[0] - q[0]).abs().next_up() + (p[1] - q[1]).abs().next_up()).next_up()
}

/// Roundoff budget for endpoint and jet conformance of one control net.
fn jet_roundoff_limit(points: &[Vec<f64>], tolerance: f64) -> f64 {
    let scale = points
        .iter()
        .flatten()
        .map(|v| v.abs())
        .fold(1., f64::max);
    (JET_ROUNDOFF * f64::EPSILON * scale).min(tolerance * arrangement::ROUNDOFF_TOLERANCE_FRACTION)
}

/// Remove only roundoff-sized endpoint jet noise from a represented trim.
/// Positive weights bound the complete curve displacement by the maximum
/// Euclidean control displacement, independently of sampling or tangent size.
fn conform_endpoint_jets(c: &mut Curve, tolerance: f64) -> Result<()> {
    let original = c.control_points.clone();
    let limit = jet_roundoff_limit(&original, tolerance);
    let (first, last) = arrangement::control_ends(&original)?;
    for endpoint in [first, last] {
        for i in 1..original.len() - 1 {
            for axis in 0..2 {
                if (c.control_points[i][axis] - endpoint[axis]).abs().next_up() > limit * 0.5 {
                    continue;
                }
                let mut candidate = c.control_points[i].clone();
                candidate[axis] = endpoint[axis];
                if l1_upper(&candidate, &original[i]) <= limit {
                    c.control_points[i] = candidate;
                }
            }
        }
    }
    Ok(())
}

pub fn normalize(wires: &[Vec<Curve>], tolerance: f64) -> Result<Vec<Vec<Curve>>> {
    if wires.is_empty() {
        return Ok(Vec::new());
    }
    if !tolerance.is_finite() || tolerance <= 0. || wires.len() > MAX_WIRES {
        return Err(refused("Invalid Bézier normalization options"));
    }
    let mut work = 0;
    let spans = AuthoredSpans::split(wires, tolerance, &mut work)?;
    let mut census = NormalizeCensus::new(&spans, tolerance, work);
    for i in 0..spans.curves.len() {
        for j in i + 1..spans.curves.len() {
            census.pair(i, j)?;
        }
    }
    let NormalizeCensus {
        mut cuts,
        aliases,
        linear,
        mut work,
        ..
    } = census;
    // Share every proposed cut across admitted carriers before comparing
    // fragments. This also handles a third curve crossing a shared subspan.
    super::coincidence_ops::synchronize(&spans.curves, &aliases, &mut cuts, tolerance, &mut work)?;
    let parity = FragmentParity {
        aliases: &aliases,
        linear: &linear,
        tolerance,
    };
    let curves = parity.odd_fragments(&spans.curves, &mut cuts, &mut work)?;
    let pieces = orient_fragments(&curves, tolerance, &mut work)?;
    super::boolean_ops::assemble(pieces, tolerance, true)
}

/// Checked self-intersection census of a high-degree parent.
type ExactCuts = (Vec<[f64; 2]>, Vec<f64>);

/// Injective, stationary-split trims of the authored spans with their parents.
#[derive(Default)]
struct AuthoredSpans {
    curves: Vec<Curve>,
    parents: Vec<Curve>,
    parent_ids: Vec<usize>,
    /// Indexed by parent id; `None` also for degenerate point spans.
    exact_parents: Vec<Option<ExactCuts>>,
}

impl AuthoredSpans {
    fn split(wires: &[Vec<Curve>], tolerance: f64, work: &mut usize) -> Result<Self> {
        let mut spans = Self::default();
        let mut next_parent_id = 0usize;
        for wire in wires {
            if wire.is_empty() {
                return Err(refused("Empty authored boundary"));
            }
            for (i, c) in wire.iter().enumerate() {
                let parent_id = next_parent_id;
                next_parent_id += 1;
                controls(c)?;
                if c.control_points.last() != wire[(i + 1) % wire.len()].control_points.first() {
                    return Err(refused("Authored boundary is open"));
                }
                if c.control_points.iter().all(|p| p == &c.control_points[0]) {
                    spans.exact_parents.push(None);
                    continue;
                }
                spans.split_parent(c, parent_id, tolerance, work)?;
            }
        }
        Ok(spans)
    }

    fn split_parent(
        &mut self,
        c: &Curve,
        parent_id: usize,
        tolerance: f64,
        work: &mut usize,
    ) -> Result<()> {
        let exact = if c.degree > 3 {
            super::boolean_ops::checked_cuts(c, c, true)
        } else {
            None
        };
        let mut cuts = c.domain().to_vec();
        if let Some(r) = &exact {
            for p in &r.0 {
                cuts.extend(*p);
            }
            cuts.extend(&r.1);
        }
        self.exact_parents.push(exact);
        stationary_cuts(c, tolerance, &mut cuts)?;
        super::coincidence_ops::coalesce(c, &mut cuts, tolerance, work)?;
        for interval in cuts.windows(2) {
            let piece = conformed_trim(c, [interval[0], interval[1]], tolerance)?;
            self.parents.push(c.clone());
            self.parent_ids.push(parent_id);
            self.curves.push(piece);
            if self.curves.len() > arrangement::MAX_ARRANGEMENT_SPANS {
                return Err(refused("Bézier normalization exceeds 1022 spans"));
            }
        }
        Ok(())
    }
}

/// One injective coordinate suffices. Splitting every coordinate extremum
/// rounds otherwise exact tangent/implicit identities. Stationary roots on the
/// chosen coordinate still split zero jets.
fn stationary_cuts(c: &Curve, tolerance: f64, cuts: &mut Vec<f64>) -> Result<()> {
    let (first, last) = arrangement::control_ends(&c.control_points)?;
    let delta = |k: usize| (last[k] - first[k]).abs();
    let injective_axis = (0..2)
        .filter(|&axis| {
            delta(axis) > arrangement::HULL_GAP * tolerance
                && nurbs_core::intersection::planar_scalar::monotone(c, axis)
        })
        .max_by(|&a, &b| delta(a).total_cmp(&delta(b)));
    let axes = injective_axis.map_or_else(|| vec![0, 1], |k| vec![k]);
    for k in axes {
        cuts.extend(match nurbs_core::intersection::planar_scalar::stationary(c, k)? {
            Some(roots) => roots,
            None if c.degree > 3 && crate::profile_certificate::nonstationary_span(c, k)? => {
                Vec::new()
            }
            None => {
                return Err(refused(
                    "Authored stationary parameter completeness is unresolved",
                ));
            }
        });
    }
    Ok(())
}

/// Trim one stationary interval and conform its endpoints to the source.
fn conformed_trim(c: &Curve, interval: [f64; 2], tolerance: f64) -> Result<Curve> {
    let mut piece = c.trim(interval[0], interval[1])?;
    // A shared cut is evaluated once by the same source definition,
    // so independent trim roundoff cannot split a cusp into two
    // almost-coincident vertices. This is only endpoint conformance.
    let limit = jet_roundoff_limit(&c.control_points, tolerance);
    let (source_first, source_last) = arrangement::control_ends(&c.control_points)?;
    for (index, t) in [
        (0, interval[0]),
        (piece.control_points.len() - 1, interval[1]),
    ] {
        let point = if t == c.domain()[0] {
            source_first.clone()
        } else if t == c.domain()[1] {
            source_last.clone()
        } else {
            c.evaluate(t)?.point
        };
        if l1_upper(&piece.control_points[index], &point) > limit {
            return Err(refused(
                "Stationary trim endpoint conformance exceeds roundoff budget",
            ));
        }
        piece.control_points[index] = point;
    }
    conform_endpoint_jets(&mut piece, tolerance)?;
    if !injective(&piece, tolerance) {
        return Err(refused("Authored cusp/injectivity remains unresolved"));
    }
    Ok(piece)
}

/// Pairwise event census of the authored trims.
struct NormalizeCensus<'a> {
    spans: &'a AuthoredSpans,
    tolerance: f64,
    cuts: Vec<Vec<f64>>,
    aliases: Vec<usize>,
    linear: Vec<bool>,
    work: usize,
}

impl<'a> NormalizeCensus<'a> {
    fn new(spans: &'a AuthoredSpans, tolerance: f64, work: usize) -> Self {
        let count = spans.curves.len();
        Self {
            spans,
            tolerance,
            cuts: spans.curves.iter().map(|c| c.domain().to_vec()).collect(),
            aliases: (0..count).collect(),
            linear: vec![false; count],
            work,
        }
    }

    fn pair(&mut self, i: usize, j: usize) -> Result<()> {
        if self.rejected_by_hulls(i, j)? {
            return Ok(());
        }
        self.work += 1;
        if self.work > arrangement::PAIR_BUDGET {
            return Err(refused("Bézier normalization pair budget exceeded"));
        }
        let spans = self.spans;
        if spans.parent_ids[i] == spans.parent_ids[j]
            && spans.exact_parents[spans.parent_ids[i]].is_some()
        {
            return Ok(());
        }
        if self.duplicate_carrier(i, j) {
            arrangement::merge_alias(&mut self.aliases, i, j);
            return Ok(());
        }
        if self.same_parent_events(i, j)?
            || self.parent_events(i, j)?
            || self.trim_relation(i, j)?
        {
            return Ok(());
        }
        self.report_events(i, j)
    }

    fn rejected_by_hulls(&self, i: usize, j: usize) -> Result<bool> {
        let a = controls(&self.spans.curves[i])?;
        let b = controls(&self.spans.curves[j])?;
        let gap = arrangement::HULL_GAP * self.tolerance;
        if (0..2).any(|k| {
            a.iter().all(|p| b.iter().all(|q| p[k] < q[k] - gap))
                || a.iter().all(|p| b.iter().all(|q| p[k] > q[k] + gap))
        }) {
            return Ok(true);
        }
        let (a_first, a_last) = arrangement::control_ends(&a)?;
        let (b_first, b_last) = arrangement::control_ends(&b)?;
        let shared = [*a_first, *a_last]
            .into_iter()
            .find(|p| p == b_first || p == b_last);
        Ok(shared.is_some_and(|p| super::shared_hulls_separate(&a, &b, p)))
    }

    fn duplicate_carrier(&self, i: usize, j: usize) -> bool {
        let curves = &self.spans.curves;
        [false, true].into_iter().any(|reverse| {
            let d = curves[j].domain();
            let domain = if reverse { [d[1], d[0]] } else { d };
            crate::profile_certificate::certify_nonlinear_curve_agreement(
                &curves[i],
                &curves[j],
                curves[i].domain(),
                domain,
            )
            .is_ok()
        })
    }

    /// Record `(t, u)` when both parameters lie in the two trims' domains.
    fn push_if_inside(&mut self, i: usize, j: usize, t: f64, u: f64) {
        let a = self.spans.curves[i].domain();
        let b = self.spans.curves[j].domain();
        if a[0] <= t && t <= a[1] && b[0] <= u && u <= b[1] {
            self.cuts[i].push(t);
            self.cuts[j].push(u);
        }
    }

    /// Self-intersections of a shared parent, in both parameter orders.
    fn same_parent_events(&mut self, i: usize, j: usize) -> Result<bool> {
        let spans = self.spans;
        if spans.parent_ids[i] != spans.parent_ids[j] {
            return Ok(false);
        }
        let Some(events) = nurbs_core::intersection::planar_scalar::self_intersections(
            &spans.parents[i],
            self.tolerance,
        )?
        else {
            return Ok(false);
        };
        for point in events {
            for (t, u) in [(point.first, point.second), (point.second, point.first)] {
                self.push_if_inside(i, j, t, u);
            }
        }
        Ok(true)
    }

    /// Stationary splitting retains the authored parent definition. Count
    /// contacts on that source before rounding a trim control net; otherwise a
    /// multiple root can spuriously bifurcate after a cut.
    fn parent_events(&mut self, i: usize, j: usize) -> Result<bool> {
        use nurbs_core::intersection::planar_scalar::{Relation, intersect};
        let spans = self.spans;
        if spans.parents[i].domain() == spans.curves[i].domain()
            && spans.parents[j].domain() == spans.curves[j].domain()
        {
            return Ok(false);
        }
        let Some(Relation::Points(events)) =
            intersect(&spans.parents[i], &spans.parents[j], self.tolerance)?
        else {
            return Ok(false);
        };
        for point in events {
            self.push_if_inside(i, j, point.first, point.second);
        }
        Ok(true)
    }

    fn trim_relation(&mut self, i: usize, j: usize) -> Result<bool> {
        use nurbs_core::intersection::planar_scalar::{Relation, intersect};
        let curves = &self.spans.curves;
        let Some(relation) = intersect(&curves[i], &curves[j], self.tolerance)? else {
            return Ok(false);
        };
        let (first, second, straight) = match relation {
            Relation::Points(points) => {
                for point in points {
                    self.cuts[i].push(point.first);
                    self.cuts[j].push(point.second);
                }
                return Ok(true);
            }
            Relation::Coincident { linear, .. } => (curves[i].domain(), curves[j].domain(), linear),
            Relation::Overlap {
                points,
                first,
                second,
                linear,
                ..
            } => {
                for point in points {
                    self.cuts[i].push(point[0]);
                    self.cuts[j].push(point[1]);
                }
                (first, second, linear)
            }
        };
        self.cuts[i].extend(first);
        self.cuts[j].extend(second);
        let (old, new) = (self.aliases[j], self.aliases[i]);
        self.linear[new] = straight || self.linear[new] || self.linear[old];
        arrangement::merge_alias(&mut self.aliases, i, j);
        Ok(true)
    }

    fn report_events(&mut self, i: usize, j: usize) -> Result<()> {
        let curves = &self.spans.curves;
        let report = intersect_curve_curve_report(
            &super::boolean_ops::lift(&curves[i]),
            &super::boolean_ops::lift(&curves[j]),
            None,
        )?;
        if !report.report.unresolved.is_empty() {
            if let Some((events, _)) =
                super::boolean_ops::checked_cuts(&curves[i], &curves[j], false)
            {
                for p in events {
                    self.cuts[i].push(p[0]);
                    self.cuts[j].push(p[1]);
                }
                return Ok(());
            }
            return Err(refused(&format!(
                "Self-intersection normalization is unresolved for spans {i} and {j}"
            )));
        }
        for event in report.report.components {
            if event.kind != CurveCurveComponentKind::Point || event.residual > self.tolerance {
                return Err(refused(
                    "Self-overlap normalization requires a resolved point event",
                ));
            }
            self.cuts[i].push(event.first);
            self.cuts[j].push(event.second);
        }
        Ok(())
    }
}

/// Even-odd parity of coincident fragments within carrier alias classes.
struct FragmentParity<'a> {
    aliases: &'a [usize],
    linear: &'a [bool],
    tolerance: f64,
}

impl FragmentParity<'_> {
    fn odd_fragments(
        &self,
        curves: &[Curve],
        cuts: &mut [Vec<f64>],
        work: &mut usize,
    ) -> Result<Vec<Curve>> {
        let mut fragments: Vec<(Curve, usize, bool)> = Vec::new();
        for (i, source) in curves.iter().enumerate() {
            super::coincidence_ops::coalesce(source, &mut cuts[i], self.tolerance, work)?;
            for range in cuts[i].windows(2) {
                if range[1] <= range[0] {
                    continue;
                }
                let piece = source.trim(range[0], range[1])?;
                if let Some(j) = self.duplicate(&piece, self.aliases[i], &fragments)? {
                    fragments[j].2 = !fragments[j].2;
                } else {
                    fragments.push((piece, self.aliases[i], true));
                }
                if fragments.len() > arrangement::FRAGMENT_BUDGET {
                    return Err(refused("Normalization fragment budget exceeded"));
                }
            }
        }
        Ok(fragments
            .into_iter()
            .filter_map(|(c, _, odd)| odd.then_some(c))
            .collect())
    }

    fn duplicate(
        &self,
        piece: &Curve,
        target: usize,
        fragments: &[(Curve, usize, bool)],
    ) -> Result<Option<usize>> {
        for (j, (other, alias, _)) in fragments.iter().enumerate() {
            if *alias != target {
                continue;
            }
            let limit = arrangement::agreement_limit(piece, other, self.tolerance);
            if self.linear[*alias] && linear_endpoints_match(piece, other, limit)? {
                return Ok(Some(j));
            }
            for reversed in [false, true] {
                let b = if reversed {
                    other.reverse()?
                } else {
                    other.clone()
                };
                if arrangement::fragments_agree(piece, &b, self.tolerance)? {
                    return Ok(Some(j));
                }
            }
        }
        Ok(None)
    }
}

/// Endpoint agreement, in either orientation, of two straight fragments.
fn linear_endpoints_match(piece: &Curve, other: &Curve, limit: f64) -> Result<bool> {
    let (p_first, p_last) = arrangement::control_ends(&piece.control_points)?;
    let (o_first, o_last) = arrangement::control_ends(&other.control_points)?;
    Ok([false, true].into_iter().any(|reversed| {
        let b = if reversed {
            [o_last, o_first]
        } else {
            [o_first, o_last]
        };
        [p_first, p_last]
            .iter()
            .zip(b)
            .all(|(p, q)| l1_upper(p, q) <= limit)
    }))
}

/// Orient every odd fragment with material on its left.
fn orient_fragments(
    curves: &[Curve],
    tolerance: f64,
    work: &mut usize,
) -> Result<Vec<(Curve, usize)>> {
    let mut pieces = Vec::new();
    for (i, source) in curves.iter().enumerate() {
        let (left, right) = material_sides(curves, i, tolerance, work)?;
        if left == right {
            return Err(refused(
                "Normalization side probes do not resolve the material boundary",
            ));
        }
        let c = if left {
            source.clone()
        } else {
            source.reverse()?
        };
        pieces.push((c, i));
        if pieces.len() > arrangement::FRAGMENT_BUDGET {
            return Err(refused("Normalization fragment budget exceeded"));
        }
    }
    Ok(pieces)
}

/// Midpoint normal probe of one fragment.
struct NormalProbe {
    point: Vec<f64>,
    /// Scaled midpoint tangent.
    d: [f64; 2],
    length: f64,
    /// Unit left normal.
    n: [f64; 2],
}

impl NormalProbe {
    fn new(source: &Curve, tolerance: f64) -> Result<Self> {
        let interval = source.domain();
        let e = source.evaluate(interval[0] * 0.5 + interval[1] * 0.5)?;
        let raw = e.d1.ok_or_else(|| refused("Normalization tangent is unresolved"))?;
        let span = interval[1] - interval[0];
        let d = [raw[0] * span, raw[1] * span];
        let length = d[0].hypot(d[1]);
        if !length.is_finite() || length <= tolerance {
            return Err(refused("Normalization cusp is unresolved"));
        }
        Ok(Self {
            point: e.point,
            d,
            length,
            n: [-d[1] / length, d[0] / length],
        })
    }

    fn offset(&self, distance: f64) -> [f64; 2] {
        [
            self.point[0] + self.n[0] * distance,
            self.point[1] + self.n[1] * distance,
        ]
    }

    /// Ordered tangent-coordinate controls give a fast uniqueness proof.
    /// Unordered controls need not mean an ambiguous curve.
    fn tangent_monotone(&self, source: &Curve) -> Result<bool> {
        let d = self.d;
        Ok(controls(source)?.windows(2).all(|p| {
            if p[0] == p[1] {
                return true;
            }
            let value = (p[1][0] - p[0][0]) * d[0] + (p[1][1] - p[0][1]) * d[1];
            let round = TANGENT_ROUNDOFF
                * f64::EPSILON
                * ((p[0][0].abs() + p[1][0].abs()) * d[0].abs()
                    + (p[0][1].abs() + p[1][1].abs()) * d[1].abs());
            value > round
        }))
    }
}

/// Two distances must agree. Each winding call admits its decision only
/// after complete control-hull separation from the probe.
fn material_sides(
    curves: &[Curve],
    i: usize,
    tolerance: f64,
    work: &mut usize,
) -> Result<(bool, bool)> {
    let source = &curves[i];
    let probe = NormalProbe::new(source, tolerance)?;
    let mut decision = None;
    for factor in PROBE_DISTANCES {
        let distance = factor * tolerance;
        let monotone = probe.tangent_monotone(source)?;
        let endpoints = [probe.offset(-distance), probe.offset(distance)];
        if !monotone {
            require_unique_probe_crossing(source, &endpoints, tolerance)?;
        }
        for (other_id, other) in curves.iter().enumerate() {
            if other_id != i {
                separate_probe(other, &endpoints, &probe, tolerance, work)?;
            }
        }
        let left = point_inside(curves, probe.offset(distance), tolerance)?;
        let right = point_inside(curves, probe.offset(-distance), tolerance)?;
        if decision.is_some_and(|v| v != (left, right)) {
            return Err(refused(
                "Normalization material side is tolerance-ambiguous",
            ));
        }
        decision = Some((left, right));
    }
    decision.ok_or_else(|| {
        refused("Normalization side probes do not resolve the material boundary")
    })
}

/// Count intersections with the actual represented finite probe. This uses
/// original curve coordinates and weights, not rounded rotated controls. An
/// uncertain census cannot become an empty or unique intersection report.
fn require_unique_probe_crossing(
    source: &Curve,
    endpoints: &[[f64; 2]; 2],
    tolerance: f64,
) -> Result<()> {
    let probe = Curve::from_polyline(endpoints.iter().map(|p| p.to_vec()).collect())?;
    let unique = matches!(
        nurbs_core::intersection::planar_scalar::intersect(source, &probe, tolerance)?,
        Some(nurbs_core::intersection::planar_scalar::Relation::Points(points))
            if points.len() == 1 && points[0].multiplicity == 1
    );
    if unique {
        Ok(())
    } else {
        Err(refused(
            "Normalization normal-probe uniqueness is unresolved",
        ))
    }
}

/// Subdivide `other` until every piece's control hull is separated from the
/// probe segment along a coordinate axis or the probe tangent.
fn separate_probe(
    other: &Curve,
    endpoints: &[[f64; 2]; 2],
    probe: &NormalProbe,
    tolerance: f64,
    work: &mut usize,
) -> Result<()> {
    let tangent = [probe.d[0] / probe.length, probe.d[1] / probe.length];
    let mut pending = vec![(other.clone(), 0usize)];
    while let Some((part, depth)) = pending.pop() {
        *work += 1;
        if *work > PROBE_WORK_BUDGET || depth > PROBE_SPLIT_DEPTH {
            return Err(refused("Normalization probe separation budget exceeded"));
        }
        let hull = controls(&part)?;
        if [[1., 0.], [0., 1.], tangent]
            .into_iter()
            .any(|axis| hull_separated_along(&hull, endpoints, axis, tolerance))
        {
            continue;
        }
        let [a, b] = part.domain();
        let [left, right] = part.split(a * 0.5 + b * 0.5)?;
        pending.push((right, depth + 1));
        pending.push((left, depth + 1));
    }
    Ok(())
}

fn hull_separated_along(
    hull: &[[f64; 2]],
    endpoints: &[[f64; 2]; 2],
    axis: [f64; 2],
    tolerance: f64,
) -> bool {
    let bounds = |points: &[[f64; 2]]| {
        points
            .iter()
            .fold([f64::INFINITY, f64::NEG_INFINITY], |r, p| {
                let v = p[0] * axis[0] + p[1] * axis[1];
                [r[0].min(v), r[1].max(v)]
            })
    };
    let a = bounds(hull);
    let b = bounds(endpoints);
    let round = PROBE_ROUNDOFF
        * f64::EPSILON
        * hull
            .iter()
            .flatten()
            .chain(endpoints.iter().flatten())
            .map(|v| v.abs())
            .fold(1., f64::max);
    let gap = arrangement::HULL_GAP * tolerance;
    a[1] + gap + round < b[0] || b[1] + gap + round < a[0]
}
