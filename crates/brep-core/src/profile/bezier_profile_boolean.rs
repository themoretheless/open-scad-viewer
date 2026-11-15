//! Native transverse polynomial profile Boolean. Trims retain source controls;
//! only shared endpoint roundoff is conformed within the requested tolerance.
use super::arrangement_ops as arrangement;
use super::{Result, controls, point_inside, refused};
use crate::Curve;
use nurbs_core::intersection::{
    ContactClass, CurveCurveComponentKind, intersect_curve_curve_report,
};
fn material(op: &str, a: bool, b: bool) -> bool {
    match op {
        "union" => a || b,
        "intersection" => a && b,
        "difference" => a && !b,
        "xor" => a != b,
        _ => false,
    }
}
pub(super) fn lift(c: &Curve) -> Curve {
    let mut c = c.clone();
    for p in &mut c.control_points {
        p.push(0.);
    }
    c
}
/// Complete checked event census, projected to numerical trim parameters only.
/// This does not certify the rounded trims or the resulting solid.
pub(super) fn checked_cuts(a: &Curve,b: &Curve,self_pair:bool)->Option<(Vec<[f64;2]>,Vec<f64>)>{
    use crate::profile_certificate as proof;
    if let Ok(c)=proof::certify_curve_intersections(a,b,self_pair)
        && let Ok(r)=proof::verify_curve_intersections(a,b,&c){
        return Some((r.events().to_vec(),r.singular_parameters().to_vec()));
    }
    let c=proof::certify_algebraic_intersections(a,b,self_pair,1e-14).ok()?;
    let r=proof::verify_algebraic_intersections(a,b,&c).ok()?;
    let midpoint=|p:[f64;2]|p[0]+(p[1]-p[0])*0.5;
    Some((r.events().iter().map(|p|[midpoint(p[0]),midpoint(p[1])]).collect(),
        r.singular_parameters().iter().map(|p|midpoint(*p)).collect()))
}
fn same(a: &Curve, b: &Curve, tolerance:f64) -> Option<bool> {
    if a.degree == b.degree && a.weights == b.weights {
        if a.control_points == b.control_points {
            return Some(true);
        }
        if a.control_points.iter().eq(b.control_points.iter().rev()) {
            return Some(false);
        }
    }
    for reverse in [false, true] {
        let d = b.domain();
        let domain = if reverse { [d[1], d[0]] } else { d };
        if crate::profile_certificate::certify_curve_agreement(a, b, a.domain(), domain).is_ok()
            || crate::profile_certificate::certify_nonlinear_curve_agreement(
                a,
                b,
                a.domain(),
                domain,
            )
            .is_ok()
        {
            return Some(!reverse);
        }
        let endpoints=if reverse{a.control_points.first()==b.control_points.last()&&a.control_points.last()==b.control_points.first()}else{a.control_points.first()==b.control_points.first()&&a.control_points.last()==b.control_points.last()};
        if endpoints&&super::injective(a,tolerance)&&super::injective(b,tolerance)
            &&crate::profile_certificate::certify_curve_image_agreement(a,b,a.domain(),domain).is_ok(){return Some(!reverse);}
    }
    None
}

fn contains(loops: &[Vec<Curve>], p: [f64; 2], tol: f64) -> Result<bool> {
    let mut inside = false;
    for wire in loops {
        inside ^= point_inside(wire, p, tol)?;
    }
    Ok(inside)
}
/// Pair work allowed before the Boolean refuses.
const PAIR_BUDGET: usize = arrangement::PAIR_BUDGET;
/// Relative coefficient roundoff admitted for a proposed overlap carrier.
const OVERLAP_ROUNDOFF: f64 = 256.;
/// Relative roundoff admitted when comparing retained fragment controls.
const FRAGMENT_ROUNDOFF: f64 = 256.;

/// `(a span, b span, same orientation, a range, b range)` of a shared carrier.
type SharedRange = (usize, usize, bool, [f64; 2], [f64; 2]);

pub fn boolean(
    a: &[Vec<Curve>],
    b: &[Vec<Curve>],
    op: &str,
    tolerance: f64,
) -> Result<Vec<Vec<Curve>>> {
    if !matches!(op, "union" | "intersection" | "difference" | "xor") {
        return Err(refused("Invalid Bézier Boolean operation"));
    }
    let a = super::orient_even_odd(a, tolerance)?;
    let b = super::orient_even_odd(b, tolerance)?;
    let av: Vec<_> = a.iter().flatten().cloned().collect();
    let bv: Vec<_> = b.iter().flatten().cloned().collect();
    if av.len() + bv.len() > arrangement::MAX_ARRANGEMENT_SPANS {
        return Err(refused("Bézier Boolean exceeds 1022 source spans"));
    }
    let mut census = BooleanCensus::new(&av, &bv, tolerance);
    for i in 0..av.len() {
        for j in 0..bv.len() {
            census.pair(i, j)?;
        }
    }
    let BooleanCensus {
        mut cuts,
        aliases,
        shared,
        linear,
        mut work,
        ..
    } = census;
    let sources: Vec<_> = av.iter().chain(&bv).cloned().collect();
    super::coincidence_ops::synchronize(&sources, &aliases, &mut cuts, tolerance, &mut work)?;
    let mut selector = FragmentSelector {
        op,
        tolerance,
        aliases: &aliases,
        shared: &shared,
        linear: &linear,
        selected: Vec::new(),
    };
    for (index, source) in sources.iter().enumerate() {
        let from_a = index < av.len();
        let side = SourceSide {
            from_a,
            local: if from_a { index } else { index - av.len() },
            other: if from_a { &b } else { &a },
        };
        super::coincidence_ops::coalesce(source, &mut cuts[index], tolerance, &mut work)?;
        selector.select_source(source, index, &side, &cuts[index])?;
    }
    assemble(selector.selected, tolerance, op == "union" || op == "difference")
}

/// Pairwise event census of the two operands' source spans.
struct BooleanCensus<'a> {
    a_spans: &'a [Curve],
    b_spans: &'a [Curve],
    tolerance: f64,
    cuts: Vec<Vec<f64>>,
    aliases: Vec<usize>,
    shared: Vec<SharedRange>,
    linear: Vec<bool>,
    work: usize,
}

impl<'a> BooleanCensus<'a> {
    fn new(a_spans: &'a [Curve], b_spans: &'a [Curve], tolerance: f64) -> Self {
        let cuts: Vec<Vec<f64>> = a_spans
            .iter()
            .chain(b_spans)
            .map(|c| c.domain().to_vec())
            .collect();
        let count = cuts.len();
        Self {
            a_spans,
            b_spans,
            tolerance,
            cuts,
            aliases: (0..count).collect(),
            shared: Vec::new(),
            linear: vec![false; count],
            work: 0,
        }
    }

    fn pair(&mut self, i: usize, j: usize) -> Result<()> {
        let (a_spans, b_spans) = (self.a_spans, self.b_spans);
        let (ca, cb) = (&a_spans[i], &b_spans[j]);
        if let Some(forward) = same(ca, cb, self.tolerance) {
            self.record_shared(i, j, forward, ca.domain(), cb.domain());
            return Ok(());
        }
        if self.rejected_by_hulls(ca, cb)? {
            return Ok(());
        }
        self.work += 1;
        if self.work > PAIR_BUDGET {
            return Err(refused("Bézier Boolean pair budget exceeded"));
        }
        if self.planar_relation(i, j, ca, cb)? {
            return Ok(());
        }
        self.report_events(i, j, ca, cb)
    }

    /// Control hulls that cannot meet, by box gap or supporting-plane proof.
    fn rejected_by_hulls(&self, ca: &Curve, cb: &Curve) -> Result<bool> {
        let pa = controls(ca)?;
        let pb = controls(cb)?;
        let tolerance = self.tolerance;
        if (0..2).any(|k| {
            let min = |p: &Vec<[f64; 2]>| p.iter().map(|q| q[k]).fold(f64::INFINITY, f64::min);
            let max = |p: &Vec<[f64; 2]>| p.iter().map(|q| q[k]).fold(f64::NEG_INFINITY, f64::max);
            max(&pa) + arrangement::HULL_GAP * tolerance < min(&pb)
                || max(&pb) + arrangement::HULL_GAP * tolerance < min(&pa)
        }) {
            return Ok(true);
        }
        // A supporting coordinate plane can contain endpoint tangencies.
        // Positive Bernstein weights and one strictly interior control on
        // each side prove that the open spans never reach that plane.
        if super::supporting_hulls_separate(&pa, &pb) {
            return Ok(true);
        }
        let (a_first, a_last) = arrangement::control_ends(&pa)?;
        let (b_first, b_last) = arrangement::control_ends(&pb)?;
        let shared_endpoint = [*a_first, *a_last]
            .into_iter()
            .find(|p| p == b_first || p == b_last);
        Ok(shared_endpoint.is_some_and(|p| super::shared_hulls_separate(&pa, &pb, p)))
    }

    fn push_event(&mut self, i: usize, j: usize, first: f64, second: f64) {
        self.cuts[i].push(first);
        self.cuts[self.a_spans.len() + j].push(second);
    }

    /// Merge the carriers of `a[i]` and `b[j]`; returns the previous class ids.
    fn record_shared(
        &mut self,
        i: usize,
        j: usize,
        forward: bool,
        a_range: [f64; 2],
        b_range: [f64; 2],
    ) -> (usize, usize) {
        let classes = arrangement::merge_alias(&mut self.aliases, i, self.a_spans.len() + j);
        self.shared.push((i, j, forward, a_range, b_range));
        classes
    }

    /// Exact planar scalar relation; `false` when it does not apply.
    fn planar_relation(&mut self, i: usize, j: usize, ca: &Curve, cb: &Curve) -> Result<bool> {
        use nurbs_core::intersection::planar_scalar::{Relation, intersect};
        let Some(relation) = intersect(ca, cb, self.tolerance)? else {
            return Ok(false);
        };
        let (a_range, b_range, reversed, straight) = match relation {
            Relation::Points(points) => {
                for point in points {
                    self.push_event(i, j, point.first, point.second);
                }
                return Ok(true);
            }
            Relation::Coincident {
                reversed,
                linear: straight,
            } => (ca.domain(), cb.domain(), reversed, straight),
            Relation::Overlap {
                points,
                linear,
                first,
                second,
                reversed,
            } => {
                for point in points {
                    self.push_event(i, j, point[0], point[1]);
                }
                (first, second, reversed, linear)
            }
        };
        self.cuts[i].extend(a_range);
        self.cuts[self.a_spans.len() + j].extend(b_range);
        let (old, new) = self.record_shared(i, j, !reversed, a_range, b_range);
        self.linear[new] = straight || self.linear[old] || self.linear[new];
        Ok(true)
    }

    /// Interval intersection report, with the checked census as fallback.
    fn report_events(&mut self, i: usize, j: usize, ca: &Curve, cb: &Curve) -> Result<()> {
        let report = intersect_curve_curve_report(&lift(ca), &lift(cb), None)?;
        if !report.report.unresolved.is_empty() {
            if let Some((events, _)) = checked_cuts(ca, cb, false) {
                for p in events {
                    self.push_event(i, j, p[0], p[1]);
                }
                return Ok(());
            }
            return Err(refused("Bézier Boolean intersection is unresolved"));
        }
        for event in report.report.components {
            if event.kind == CurveCurveComponentKind::Overlap {
                let x = ca.trim(event.first_interval[0], event.first_interval[1])?;
                let y = cb.trim(event.second_interval[0], event.second_interval[1])?;
                let forward = same(&x, &y, self.tolerance)
                    .or_else(|| coefficient_orientation(&x, &y))
                    .ok_or_else(|| refused("Bézier overlap carrier identity is unresolved"))?;
                self.cuts[i].extend(event.first_interval);
                self.cuts[self.a_spans.len() + j].extend(event.second_interval);
                self.record_shared(i, j, forward, event.first_interval, event.second_interval);
                continue;
            }
            if !matches!(
                event.contact,
                ContactClass::Transverse
                    | ContactClass::Boundary
                    | ContactClass::OddTangency
                    | ContactClass::EvenTangency
            ) {
                return Err(refused("Bézier Boolean contact is unresolved"));
            }
            if event.residual > self.tolerance {
                return Err(refused("Bézier Boolean event residual exceeds tolerance"));
            }
            self.push_event(i, j, event.first, event.second);
        }
        Ok(())
    }
}

/// Interval intersection proposes an overlap; retained polynomial Bernstein
/// coefficients independently admit it, in one of the two orientations.
fn coefficient_orientation(x: &Curve, y: &Curve) -> Option<bool> {
    if x.degree != y.degree || x.weights != y.weights {
        return None;
    }
    for forward in [true, false] {
        let yy = if forward { y.clone() } else { y.reverse().ok()? };
        if x.control_points.iter().zip(&yy.control_points).all(|(p, q)| {
            p.iter().zip(q).all(|(a, b)| {
                (a - b).abs() <= OVERLAP_ROUNDOFF * f64::EPSILON * a.abs().max(b.abs()).max(1.)
            })
        }) {
            return Some(forward);
        }
    }
    None
}

/// Which operand a source span comes from, and the other operand's loops.
struct SourceSide<'a> {
    from_a: bool,
    local: usize,
    other: &'a [Vec<Curve>],
}

/// Material classification and deduplication of trimmed source fragments.
struct FragmentSelector<'a> {
    op: &'a str,
    tolerance: f64,
    aliases: &'a [usize],
    shared: &'a [SharedRange],
    linear: &'a [bool],
    selected: Vec<(Curve, usize)>,
}

impl FragmentSelector<'_> {
    fn select_source(
        &mut self,
        source: &Curve,
        index: usize,
        side: &SourceSide,
        cuts: &[f64],
    ) -> Result<()> {
        for interval in cuts.windows(2) {
            if interval[1] <= interval[0] {
                continue;
            }
            let mut piece = source.trim(interval[0], interval[1])?;
            let p = piece.evaluate(interval[0] * 0.5 + interval[1] * 0.5)?.point;
            let mid = interval[0] * 0.5 + interval[1] * 0.5;
            let (ol, or) = if let Some(forward) = self.shared_orientation(side, mid) {
                (forward, !forward)
            } else {
                let inside = contains(side.other, [p[0], p[1]], self.tolerance)?;
                (inside, inside)
            };
            let (left, right) = if side.from_a {
                (material(self.op, true, ol), material(self.op, false, or))
            } else {
                (material(self.op, ol, true), material(self.op, or, false))
            };
            if left == right {
                continue;
            }
            if !left {
                piece = piece.reverse()?;
            }
            let alias = self.aliases[index];
            if self.coincides_with_selected(&piece, alias)?
                || self.duplicates_selected(&piece, alias)?
            {
                continue;
            }
            self.selected.push((piece, alias));
            if self.selected.len() > arrangement::FRAGMENT_BUDGET {
                return Err(refused("Bézier Boolean fragment budget exceeded"));
            }
        }
        Ok(())
    }

    /// Orientation of a shared carrier range strictly containing `mid`.
    fn shared_orientation(&self, side: &SourceSide, mid: f64) -> Option<bool> {
        self.shared.iter().find_map(|(i, j, same, ar, br)| {
            let range = if side.from_a { ar } else { br };
            let owner = if side.from_a { *i } else { *j };
            (owner == side.local && range[0] < mid && mid < range[1]).then_some(*same)
        })
    }

    fn coincides_with_selected(&self, piece: &Curve, alias: usize) -> Result<bool> {
        for (c, id) in &self.selected {
            if *id == alias && arrangement::fragments_agree(c, piece, self.tolerance)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn duplicates_selected(&self, piece: &Curve, alias: usize) -> Result<bool> {
        let tolerance = self.tolerance;
        for (c, id) in &self.selected {
            // Endpoint agreement is sufficient only for source carriers
            // independently proved straight by the exact scalar reduction.
            if *id == alias && self.linear[alias] {
                let (c_first, c_last) = arrangement::control_ends(&c.control_points)?;
                let (p_first, p_last) = arrangement::control_ends(&piece.control_points)?;
                if [(c_first, p_first), (c_last, p_last)].iter().all(|(p, q)| {
                    (p[0] - q[0]).hypot(p[1] - q[1]) <= arrangement::HULL_GAP * tolerance
                }) {
                    return Ok(true);
                }
            }
            let control_limit = |x: f64, y: f64| {
                if *id == alias {
                    arrangement::HULL_GAP * tolerance
                } else {
                    FRAGMENT_ROUNDOFF * f64::EPSILON * x.abs().max(y.abs()).max(1.)
                }
            };
            if c.degree == piece.degree
                && c.control_points.len() == piece.control_points.len()
                && c.weights.iter().zip(&piece.weights).all(|(x, y)| {
                    (x / c.weights[0] - y / piece.weights[0]).abs()
                        <= FRAGMENT_ROUNDOFF * f64::EPSILON
                })
                && c.control_points.iter().zip(&piece.control_points).all(|(p, q)| {
                    p.iter()
                        .zip(q)
                        .all(|(x, y)| (x - y).abs() <= control_limit(*x, *y))
                })
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
/// Departure angles closer than this are treated as tied.
const ANGLE_TIE: f64 = 1e-10;

pub(super) fn assemble(
    mut selected: Vec<(Curve, usize)>,
    tolerance: f64,
    contacts: bool,
) -> Result<Vec<Vec<Curve>>> {
    let (vertex_count, links) = cluster_vertices(&mut selected, tolerance)?;
    let outgoing = outgoing_links(&links, vertex_count, contacts)?;
    let next = successors(&selected, &links, &outgoing)?;
    let loops = trace_loops(&selected, &next, tolerance)?;
    if !loops.is_empty() {
        super::classify_components(&loops, tolerance)?;
    }
    Ok(loops)
}

/// Snap fragment endpoints to unique vertex clusters; returns the vertex
/// count and each fragment's `[start, end]` vertex.
fn cluster_vertices(
    selected: &mut [(Curve, usize)],
    tolerance: f64,
) -> Result<(usize, Vec<[usize; 2]>)> {
    let mut vertices: Vec<Vec<f64>> = Vec::new();
    let mut links = Vec::new();
    for (curve, _) in selected.iter_mut() {
        let mut pair = [0usize; 2];
        let last = curve
            .control_points
            .len()
            .checked_sub(1)
            .ok_or_else(|| refused("Bézier span has no control points"))?;
        for (side, index) in [0, last].into_iter().enumerate() {
            let p = &curve.control_points[index];
            let nearby: Vec<_> = vertices
                .iter()
                .enumerate()
                .filter(|(_, q)| {
                    (p[0] - q[0]).hypot(p[1] - q[1]) <= arrangement::HULL_GAP * tolerance
                })
                .map(|(i, _)| i)
                .collect();
            if nearby.len() > 1 {
                return Err(refused("Bézier Boolean vertex clustering is ambiguous"));
            }
            pair[side] = if let Some(i) = nearby.first() {
                *i
            } else {
                vertices.push(p.clone());
                vertices.len() - 1
            };
            curve.control_points[index] = vertices[pair[side]].clone();
        }
        if pair[0] == pair[1] {
            return Err(refused(
                "Bézier Boolean fragment collapses at the vertex tolerance",
            ));
        }
        links.push(pair);
    }
    Ok((vertices.len(), links))
}

/// Outgoing fragments per vertex, refusing unbalanced or forbidden contacts.
fn outgoing_links(
    links: &[[usize; 2]],
    vertex_count: usize,
    contacts: bool,
) -> Result<Vec<Vec<usize>>> {
    let mut outgoing = vec![Vec::new(); vertex_count];
    let mut incoming = vec![0usize; vertex_count];
    for (i, p) in links.iter().enumerate() {
        outgoing[p[0]].push(i);
        incoming[p[1]] += 1;
    }
    if incoming
        .iter()
        .zip(&outgoing)
        .any(|(n, out)| *n != out.len() || (!contacts && *n != 1))
    {
        return Err(refused("Bézier Boolean has a nonmanifold boundary contact"));
    }
    Ok(outgoing)
}

/// Successor of every fragment by the turning order at its end vertex.
fn successors(
    selected: &[(Curve, usize)],
    links: &[[usize; 2]],
    outgoing: &[Vec<usize>],
) -> Result<Vec<usize>> {
    let mut next = vec![0usize; selected.len()];
    let mut predecessor = vec![0usize; selected.len()];
    let mut order_work = 0;
    for (i, link) in links.iter().enumerate() {
        next[i] = successor(selected, &selected[i].0, &outgoing[link[1]], &mut order_work)?;
        predecessor[next[i]] += 1;
    }
    if predecessor.iter().any(|n| *n != 1) {
        return Err(refused("Boundary traversal is nonmanifold"));
    }
    Ok(next)
}

fn successor(
    selected: &[(Curve, usize)],
    curve: &Curve,
    outgoing: &[usize],
    order_work: &mut usize,
) -> Result<usize> {
    let reverse = curve.reverse()?;
    let d = super::departure_ops::direction(&reverse)
        .ok_or_else(|| refused("Boundary direction is unresolved"))?;
    let back = d[1].atan2(d[0]);
    let mut candidates = Vec::new();
    for &j in outgoing {
        let d = super::departure_ops::direction(&selected[j].0)
            .ok_or_else(|| refused("Boundary cusp has no nonzero jet"))?;
        candidates.push((
            (back - d[1].atan2(d[0])).rem_euclid(std::f64::consts::TAU),
            j,
        ));
    }
    if candidates.len() > 1 && angles_tied(&candidates) {
        let branches: Vec<_> = outgoing.iter().map(|j| (*j, &selected[*j].0)).collect();
        return super::departure_ops::continuation(curve, &branches, order_work)?
            .ok_or_else(|| refused("Boundary tangent departure ordering is unresolved"));
    }
    if candidates.is_empty() {
        return Err(refused(
            "Boundary has only zero-angle tangent continuations",
        ));
    }
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
    if candidates.len() > 1 && (candidates[0].0 - candidates[1].0).abs() < ANGLE_TIE {
        return Err(refused("Boundary tangent ordering is ambiguous"));
    }
    Ok(candidates[0].1)
}

/// A zero turning angle, or two candidates with the same turning angle.
fn angles_tied(candidates: &[(f64, usize)]) -> bool {
    candidates.iter().any(|a| a.0 < ANGLE_TIE)
        || candidates.iter().enumerate().any(|(i, a)| {
            candidates
                .iter()
                .skip(i + 1)
                .any(|b| (a.0 - b.0).abs() < ANGLE_TIE)
        })
}

fn trace_loops(
    selected: &[(Curve, usize)],
    next: &[usize],
    tolerance: f64,
) -> Result<Vec<Vec<Curve>>> {
    let mut used = vec![false; selected.len()];
    let mut loops = Vec::new();
    let mut emitted = 0;
    for first in 0..selected.len() {
        if used[first] {
            continue;
        }
        let mut wire = Vec::new();
        let mut current = first;
        loop {
            if used[current] {
                if current != first {
                    return Err(refused("Bézier Boolean boundary traversal is ambiguous"));
                }
                break;
            }
            used[current] = true;
            wire.push(selected[current].0.clone());
            current = next[current];
        }
        if wire.len() == 2 {
            split_lens(&mut wire, tolerance)?;
        }
        emitted += wire.len();
        if emitted > arrangement::FRAGMENT_BUDGET {
            return Err(refused("Bézier output fragment budget exceeded"));
        }
        loops.push(wire);
    }
    Ok(loops)
}

/// A valid curved lens may have only two boundary carriers. Preserve both and
/// split one retained curve to satisfy the cap builder's three-span wire
/// convention; no chord or fitted carrier is introduced.
fn split_lens(wire: &mut Vec<Curve>, tolerance: f64) -> Result<()> {
    let index = usize::from(wire[1].degree > wire[0].degree);
    let original = &wire[index];
    let [a, b] = original.domain();
    let mut parts = original.split(a * 0.5 + b * 0.5)?;
    let (start, end) = arrangement::control_ends(&original.control_points)?;
    let endpoints = [start.clone(), end.clone()];
    let middle = arrangement::control_ends(&parts[0].control_points)?.1.clone();
    let last = parts[1].control_points.len() - 1;
    for (part, index, p) in [
        (0, 0, endpoints[0].clone()),
        (1, 0, middle),
        (1, last, endpoints[1].clone()),
    ] {
        let q = &parts[part].control_points[index];
        if (q[0] - p[0]).hypot(q[1] - p[1]) > tolerance {
            return Err(refused("Lens trim endpoint conformance exceeds tolerance"));
        }
        parts[part].control_points[index] = p;
    }
    wire.splice(index..index + 1, parts);
    Ok(())
}

#[cfg(test)]mod algebraic_cuts_tests{
    use super::*;
    #[test]fn nondyadic_fallback_returns_complete_numerical_cuts(){
        let a=Curve{degree:4,knots:[vec![0.;5],vec![1.;5]].concat(),control_points:vec![vec![0.,1.],vec![1.,-1.],vec![2.,1.],vec![3.,-1.],vec![4.,1.]],weights:vec![1.;5],periodic:false};
        let b=crate::sketch::polygon_wire(vec![[0.,0.25],[4.,0.25],[4.,1.]]).unwrap().remove(0);
        let r=checked_cuts(&a,&b,false).unwrap();assert_eq!(r.0.len(),2);
    }
}
