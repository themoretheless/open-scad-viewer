//! Complete adaptive coverage of original same-surface endpoint pairs.
use crate::{
    check, distance_bounds::Interval as I, normal_alignment, surface::Surface, surface_distance,
    surface_self_chord, Result,
};
type Rectangle = [[f64; 2]; 2];
type Pair = [Rectangle; 2];
/// On A×A, closed halves L/R yield LL, LR, RR plus the swapped LR.
/// Normal alignment uses squared angles and distance uses chord length, so
/// reversing the chord and exchanging its two endpoint tests changes neither.
fn diagonal_children(rect: Rectangle, axis: usize, mid: f64) -> [Pair; 3] {
    let mut left=rect; let mut right=rect;
    left[axis][1]=mid; right[axis][0]=mid;
    [[left,left],[left,right],[right,right]]
}
/// Per-call cache owns one immutable source and one admission tolerance.
/// Only complete derivative/continuity coverage can be reused for free.
struct RectangleCache<'a> {
    surface: &'a Surface,
    sine: f64,
    reports: std::collections::HashMap<[[u64;2];2],surface_self_chord::Report<'a>>,
}
impl<'a> RectangleCache<'a> {
    fn new(surface:&'a Surface,sine:f64)->Self {
        Self{surface,sine,reports:std::collections::HashMap::new()}
    }
    fn qualify(&mut self,domain:Rectangle,budget:usize)
        ->Result<(surface_self_chord::Report<'a>,bool)> {
        let key=domain.map(|axis|axis.map(f64::to_bits));
        if let Some(report)=self.reports.get(&key) {
            let mut report=report.clone();report.spans=0;
            return Ok((report,true));
        }
        let report=surface_self_chord::qualify_rectangle(self.surface,domain,self.sine,budget)?;
        if report.complete {self.reports.insert(key,report.clone());}
        Ok((report,false))
    }
}
pub struct Limits {
    pub cells: usize,
    pub spans: usize,
}
pub struct Certificate<'a> {
    surface: &'a Surface,
    minimum_mm: f64,
    max_sine_squared: f64,
    intrinsic: Vec<surface_self_chord::Certificate<'a>>,
}
impl<'a> Certificate<'a> {
    pub fn surface(&self) -> &'a Surface {
        self.surface
    }
    pub fn minimum_mm(&self) -> f64 {
        self.minimum_mm
    }
    pub fn max_sine_squared(&self) -> f64 {
        self.max_sine_squared
    }
    pub fn intrinsic_certificates(&self) -> &[surface_self_chord::Certificate<'a>] {
        &self.intrinsic
    }
}
pub struct Report<'a> {
    pub certificate: Option<Certificate<'a>>,
    pub cells: usize,
    pub spans: usize,
    pub intrinsic_exclusions: usize,
    pub polar_exclusions: usize,
    pub coordinate_path_exclusions: usize,
    pub coordinate_path_attempts: usize,
    pub coordinate_path_refusals: usize,
    pub local_coordinate_path_exclusions: usize,
    /// Qualified local paths by proven margin: three, one, quarter width.
    pub local_margin_exclusions: [usize; 3],
    pub local_coordinate_path_attempts: usize,
    pub local_coordinate_path_refusals: usize,
    pub polar_factor_refusals: usize,
    pub angular_exclusions: usize,
    pub factored_angular_exclusions: usize,
    pub direction_exclusions: usize,
    /// Subset of direction exclusions proved by the original mean matrix.
    pub mean_matrix_exclusions: usize,
    pub complete_rectangle_cache_hits: usize,
    pub original_normal_rectangle_cache_hits: usize,
    pub distance_exclusions: usize,
    pub pending: usize,
    pub uncertain: Option<Pair>,
    /// Remaining closed pair representatives in original parameter coordinates.
    /// Each also represents the pair with its endpoints exchanged.
    /// These are unresolved obligations, never asserted geometric defects.
    pub unresolved_pairs: Vec<Pair>,
    pub reason: &'static str,
}
fn span_count(s: &Surface, domain: Rectangle) -> usize {
    let count = |degree: usize, knots: &[f64], n: usize, d: [f64; 2]| {
        (degree..n)
            .filter(|&i| knots[i] < knots[i + 1] && knots[i] <= d[1] && knots[i + 1] >= d[0])
            .count()
    };
    count(s.degree_u, &s.knots_u, s.control_points.len(), domain[0])
        * count(s.degree_v, &s.knots_v, s.control_points[0].len(), domain[1])
}
fn squared_norm(v: [I; 3]) -> Result<I> {
    let mut sum = I::point(0.);
    for x in v {
        let lo = if x.lo > 0. {
            x.lo
        } else if x.hi < 0. {
            -x.hi
        } else {
            0.
        };
        let hi = x.lo.abs().max(x.hi.abs());
        sum = sum.add(I::new((lo * lo).next_down().max(0.), (hi * hi).next_up())?)?;
    }
    I::new(sum.lo.max(0.), sum.hi)
}
/// Original position boxes enclose every secant; original normal boxes enclose
/// both endpoint normals. One endpoint strictly oblique excludes the whole pair.
fn secant_exclusion(
    s: &Surface,
    pair: Pair,
    boxes: [&Vec<[f64; 2]>; 2],
    sine: f64,
    max_spans: usize,
    factored: Option<&crate::factored_surface_normals::Certificate<'_>>,
) -> Result<(bool, usize)> {
    secant_exclusion_cached(s, pair, boxes, sine, max_spans, factored, None, None)
}
fn secant_exclusion_cached(
    s: &Surface, pair: Pair, boxes: [&Vec<[f64; 2]>; 2], sine: f64,
    max_spans: usize,
    factored: Option<&crate::factored_surface_normals::Certificate<'_>>,
    cached: Option<&crate::factored_surface_normals::PairBounds<'_>>,
    original: Option<&normal_alignment::OriginalPairBounds<'_>>,
) -> Result<(bool, usize)> {
    let components = if let Some(cache) = cached {
        let proof = factored.ok_or(crate::input("Missing original normal proof"))?;
        crate::check(std::ptr::eq(s, proof.surface()), "Use original normal source")?;
        Some(cache.components(proof, pair)?)
    } else if let Some(cache)=original {
        Some(cache.components(s,pair)?)
    } else { None };
    let mut d = [I::point(0.); 3];
    for k in 0..3 {
        d[k] =
            I::new(boxes[1][k][0], boxes[1][k][1])?.sub(I::new(boxes[0][k][0], boxes[0][k][1])?)?;
    }
    let scale = d
        .iter()
        .map(|x| x.lo.abs().max(x.hi.abs()))
        .fold(0_f64, f64::max);
    if scale < f64::MIN_POSITIVE {
        return Ok((false, 0));
    }
    for x in &mut d {
        *x = x.div(I::point(scale))?;
    }
    let length = squared_norm(d)?;
    if length.lo <= 0. {
        return Ok((false, 0));
    }
    let threshold = I::point(1.).sub(I::point(sine))?;
    let mut spans = 0;
    for (index, rect) in pair.into_iter().enumerate() {
        if components.is_none() && spans == max_spans {
            break;
        }
        let (bounds, used) = if let Some(values) = components {
            (Some(values[index]), 0)
        } else if let Some(proof) = factored {
            // A prepared fact covers one original Bezier span. Its exact
            // preparation budget is separate from this bounded coverage call.
            (Some(proof.bounds(rect)?), 1)
        } else {
            normal_alignment::normal_bounds(s, rect, max_spans - spans)?
        };
        spans += used;
        let Some(bounds) = bounds else {
            continue;
        };
        let scale = bounds
            .iter()
            .flatten()
            .map(|x| x.abs())
            .fold(0_f64, f64::max);
        if scale < f64::MIN_POSITIVE {
            continue;
        }
        let mut n = [I::point(0.); 3];
        for k in 0..3 {
            n[k] = I::new(bounds[k][0], bounds[k][1])?.div(I::point(scale))?;
        }
        let denominator = length.mul(squared_norm(n)?)?;
        if denominator.lo <= 0. {
            continue;
        }
        let mut dot = I::point(0.);
        for k in 0..3 {
            dot = dot.add(n[k].mul(d[k])?)?;
        }
        let upper = dot.lo.abs().max(dot.hi.abs());
        let numerator = I::point(upper).mul(I::point(upper))?;
        if numerator.div(denominator)?.hi < threshold.lo {
            return Ok((true, spans));
        }
    }
    Ok((false, spans))
}

/// Covers the Cartesian product of the full original UV chart with itself.
/// Every split preserves both closed child rectangles, including the diagonal.
/// A local intrinsic certificate covers both endpoints through their convex UV
/// hull. Otherwise original normal separation or original Cartesian distance
/// must exclude the pair. Missing work never constructs a global certificate.
pub fn qualify<'a>(
    s: &'a Surface,
    minimum_mm: f64,
    max_sine_squared: f64,
    limits: Limits,
) -> Result<Report<'a>> {
    qualify_impl(s, minimum_mm, max_sine_squared, limits, None, None)
}
/// Reuse only a sealed exact boundary-factor fact from this same immutable
/// source. Original exact preparation work is paid before coverage begins.
pub fn qualify_with_factored_normals<'a>(
    s: &'a Surface,
    minimum_mm: f64,
    max_sine_squared: f64,
    limits: Limits,
    proof: &crate::factored_surface_normals::Certificate<'a>,
) -> Result<Report<'a>> {
    check(
        std::ptr::eq(s, proof.surface()),
        "Use factored normals from the same original surface",
    )?;
    qualify_impl(s, minimum_mm, max_sine_squared, limits, Some(proof), None)
}
/// Reuses exact differential and normal facts from the same source and pole.
pub fn qualify_with_rational_pole<'a>(
    s: &'a Surface,
    minimum_mm: f64,
    max_sine_squared: f64,
    limits: Limits,
    normals: &crate::factored_surface_normals::Certificate<'a>,
    differential: &crate::rational_pole_differential::Certificate<'a>,
) -> Result<Report<'a>> {
    check(
        std::ptr::eq(s, normals.surface())
            && std::ptr::eq(s, differential.surface())
            && normals.end() == differential.end(),
        "Use pole facts from the same original surface and end",
    )?;
    qualify_impl(
        s,
        minimum_mm,
        max_sine_squared,
        limits,
        Some(normals),
        Some(differential),
    )
}
fn qualify_impl<'a>(
    s: &'a Surface,
    minimum_mm: f64,
    max_sine_squared: f64,
    limits: Limits,
    factored: Option<&crate::factored_surface_normals::Certificate<'a>>,
    differential: Option<&crate::rational_pole_differential::Certificate<'a>>,
) -> Result<Report<'a>> {
    s.validate()?;
    check(
        minimum_mm.is_finite()
            && minimum_mm > 0.
            && max_sine_squared.is_finite()
            && (0. ..1.).contains(&max_sine_squared)
            && (1..=100000).contains(&limits.cells)
            && (1..=100000).contains(&limits.spans),
        "Bound same-face coverage work and choose positive threshold",
    )?;
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let necessary = if max_sine_squared < 0.5 {
        let sine = I::point(max_sine_squared);
        let upper = I::point(4.).mul(sine)?.mul(I::point(1.).sub(sine)?)?.hi;
        (upper < 1.).then_some(upper)
    } else {
        None
    };
    let mut out = Report {
        certificate: None,
        cells: 0,
        spans: 0,
        intrinsic_exclusions: 0,
        polar_exclusions: 0,
        coordinate_path_exclusions: 0,
        coordinate_path_attempts: 0,
        coordinate_path_refusals: 0,
        local_coordinate_path_exclusions: 0,
        local_margin_exclusions: [0; 3],
        local_coordinate_path_attempts: 0,
        local_coordinate_path_refusals: 0,
        polar_factor_refusals: 0,
        angular_exclusions: 0,
        factored_angular_exclusions: 0,
        direction_exclusions: 0,
        mean_matrix_exclusions: 0,
        complete_rectangle_cache_hits: 0,
        original_normal_rectangle_cache_hits: 0,
        distance_exclusions: 0,
        pending: 1,
        uncertain: None,
        unresolved_pairs: Vec::new(),
        reason: "self-coverage-work-limit",
    };
    // Visit each subdivision level before refining one singular corner.
    let mut pending = std::collections::VecDeque::from([[domain, domain]]);
    let mut intrinsic = Vec::new();
    let mut rectangles=RectangleCache::new(s,max_sine_squared);
    let mut normal_rectangles=normal_alignment::OriginalNormalCache::new(s);
    while let Some(pair) = pending.pop_front() {
        if out.cells == limits.cells || out.spans == limits.spans {
            pending.push_back(pair);
            break;
        }
        out.cells += 1;
        let mut pair_normals = None;
        let mut original_normals = None;
        let union = std::array::from_fn(|axis| {
            [
                pair[0][axis][0].min(pair[1][axis][0]),
                pair[0][axis][1].max(pair[1][axis][1]),
            ]
        });
        let (r,reused)=rectangles.qualify(union,limits.spans-out.spans)?;
        out.complete_rectangle_cache_hits+=usize::from(reused);
        out.spans += r.spans;
        if let Some(c) = r.certificate {
            out.intrinsic_exclusions += 1;
            intrinsic.push(c);
            continue;
        }
        let ordinary_matrix = r.mean_matrix;
        if let Some(proof) = differential.filter(|_| out.spans < limits.spans) {
            let r = surface_self_chord::qualify_rational_polar_rectangle(
                s,
                union,
                proof,
                max_sine_squared,
                limits.spans - out.spans,
            )?;
            out.spans += r.spans;
            if let Some(c) = r.certificate {
                out.intrinsic_exclusions += 1;
                out.polar_exclusions += 1;
                intrinsic.push(c);
                continue;
            }
        }
        if let Some(threshold) = necessary.filter(|_| out.spans < limits.spans) {
            if let Some(proof) = factored {
                if limits.spans - out.spans >= 2 {
                    let cache = proof.pair_bounds(pair)?;
                    let components = cache.components(proof, pair)?;
                    pair_normals = Some(cache);
                    out.spans += 2;
                    let r = normal_alignment::inspect_pair_bounds(components, threshold)?;
                    if r.aligned == Some(false) {
                        out.angular_exclusions += 1;
                        out.factored_angular_exclusions += 1;
                        continue;
                    }
                }
            } else {
                let (r,cache,hits) = normal_rectangles.inspect_pair(
                    pair,
                    threshold,
                    limits.spans - out.spans,
                )?;
                out.original_normal_rectangle_cache_hits+=hits;
                original_normals=cache;
                out.spans += r.spans;
                if r.aligned == Some(false) {
                    out.angular_exclusions += 1;
                    continue;
                }
            }
        }
        // Cheap original position/secant bounds often exclude separated cells
        // before constructing any of the three pole mean matrices.
        let work = span_count(s, pair[0]) + span_count(s, pair[1]);
        if work <= limits.spans - out.spans {
            let a = surface_distance::rectangle_bounds(s, pair[0])?;
            let b = surface_distance::rectangle_bounds(s, pair[1])?;
            out.spans += work;
            if surface_distance::enclosure_distance(&a, &b)?.0 >= minimum_mm {
                out.distance_exclusions += 1;
                continue;
            }
            if out.spans < limits.spans || pair_normals.is_some() || original_normals.is_some() {
                let (excluded, used) = secant_exclusion_cached(
                    s,
                    pair,
                    [&a, &b],
                    max_sine_squared,
                    limits.spans - out.spans,
                    factored,
                    pair_normals.as_ref(),
                    original_normals.as_ref(),
                )?;
                out.spans += used;
                if excluded {
                    out.direction_exclusions += 1;
                    continue;
                }
            }
        }
        if let Some(matrix) = ordinary_matrix.as_ref() {
            if out.spans < limits.spans || pair_normals.is_some() || original_normals.is_some() {
                let cached = factored.zip(pair_normals.as_ref());
                let (excluded,used) = matrix.exclude_pair(pair,max_sine_squared,
                    limits.spans-out.spans,cached,original_normals.as_ref())?;
                out.spans += used;
                if excluded {
                    out.direction_exclusions += 1;
                    out.mean_matrix_exclusions += 1;
                    continue;
                }
            }
        }
        if let Some(matrix)=ordinary_matrix.as_ref() {
            // Both orders retain all endpoints; a successful path excludes
            // the complete original pair, including its exchanged order.
            let mut excluded=false;
            for endpoints in [pair,[pair[1],pair[0]]] {
                if out.spans==limits.spans {break;}
                let (u,reused)=rectangles.qualify([union[0],endpoints[0][1]],limits.spans-out.spans)?;
                out.spans+=u.spans;out.complete_rectangle_cache_hits+=usize::from(reused);
                let Some(u)=u.mean_matrix else {continue};
                if out.spans==limits.spans {break;}
                let (v,reused)=rectangles.qualify([endpoints[1][0],union[1]],limits.spans-out.spans)?;
                out.spans+=v.spans;out.complete_rectangle_cache_hits+=usize::from(reused);
                let Some(v)=v.mean_matrix else {continue};
                let path=matrix.coordinate_path(endpoints,&u,&v)?;
                // Normal caches are indexed by the original endpoint order.
                let (done,used)=path.exclude_pair(pair,max_sine_squared,
                    limits.spans-out.spans,factored.zip(pair_normals.as_ref()),original_normals.as_ref())?;
                out.spans+=used;
                if done {excluded=true;break;}
                if !done {
                    let chord=path.chord_bounds()?;
                    let zero=vec![[0.,0.];3];
                    let (done,used)=secant_exclusion_cached(s,pair,[&zero,&chord],max_sine_squared,
                        limits.spans-out.spans,factored,pair_normals.as_ref(),original_normals.as_ref())?;
                    out.spans+=used;
                    if done {excluded=true;break;}
                }

            }
            if excluded {out.direction_exclusions+=1;continue;}
        }
        if let (Some(differential), Some(normals)) = (differential, factored) {
            if out.spans < limits.spans {
                let r = surface_self_chord::rational_pole_pair_report_cached(
                    s,
                    pair,
                    differential,
                    normals,
                    max_sine_squared,
                    limits.spans - out.spans,
                    pair_normals.as_ref(),
                )?;
                out.spans += r.spans;
                out.coordinate_path_attempts += usize::from(r.coordinate_path_attempted);
                out.coordinate_path_refusals +=
                    usize::from(r.coordinate_path_attempted && !r.coordinate_path_available);
                out.coordinate_path_exclusions += usize::from(r.coordinate_path_excluded);
                out.local_coordinate_path_exclusions += usize::from(r.local_path_excluded);
                if let Some(margin) = r.local_path_margin {
                    let index = [3., 1., 0.25].iter().position(|m| *m == margin)
                        .expect("Only original proven local path margins are reported");
                    out.local_margin_exclusions[index] += 1;
                }
                out.local_coordinate_path_attempts += usize::from(r.local_path_attempted);
                out.local_coordinate_path_refusals +=
                    usize::from(r.local_path_attempted && !r.local_path_available);
                if r.excluded {
                    out.direction_exclusions += 1;
                    continue;
                }
            }
        }
        if let Some(proof) = factored.filter(|_| out.spans < limits.spans) {
            let r = surface_self_chord::qualify_polar_rectangle(
                s,
                union,
                proof.end(),
                max_sine_squared,
                limits.spans - out.spans,
            )?;
            out.spans += r.spans;
            if r.reason == "polar-chord-factor-unproven" {
                out.polar_factor_refusals += 1;
            }
            if let Some(c) = r.certificate {
                out.intrinsic_exclusions += 1;
                out.polar_exclusions += 1;
                intrinsic.push(c);
                continue;
            }
        }
        if out.spans == limits.spans {
            pending.push_back(pair);
            break;
        }
        let mut selected = (0, 0);
        let mut largest = 0.;
        for side in 0..2 {
            for axis in 0..2 {
                let width = (pair[side][axis][1] - pair[side][axis][0])
                    / (domain[axis][1] - domain[axis][0]);
                if width > largest {
                    largest = width;
                    selected = (side, axis)
                }
            }
        }
        let (side, axis) = selected;
        let d = pair[side][axis];
        let mid = d[0] * 0.5 + d[1] * 0.5;
        if mid <= d[0] || mid >= d[1] {
            pending.push_back(pair);
            out.reason = "self-coverage-resolution-limit";
            break;
        }
        if pair[0]==pair[1] {
            let [ll,lr,rr]=diagonal_children(pair[0],axis,mid);
            pending.push_back(rr); pending.push_back(lr); pending.push_back(ll);
            continue;
        }
        let mut left = pair;
        let mut right = pair;
        left[side][axis][1] = mid;
        right[side][axis][0] = mid;
        pending.push_back(right);
        pending.push_back(left);
    }
    out.pending = pending.len();
    out.uncertain = pending.back().copied();
    out.unresolved_pairs = pending.iter().copied().collect();
    if pending.is_empty() {
        out.reason = "self-coverage-qualified";
        out.certificate = Some(Certificate {
            surface: s,
            minimum_mm,
            max_sine_squared,
            intrinsic,
        });
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagonal_split_covers_closed_product_up_to_endpoint_exchange() {
        let rect=[[0.,1.],[-2.,2.]];
        for axis in 0..2 {
            let mid=(rect[axis][0]+rect[axis][1])*0.5;
            let children=diagonal_children(rect,axis,mid);
            let points:Vec<_>=[0.,0.25,0.5,0.75,1.].into_iter().flat_map(|u|
                [-2.,-1.,0.,1.,2.].into_iter().map(move|v|[u,v])).collect();
            let contains=|r:Rectangle,p:[f64;2]| (0..2).all(|a|r[a][0]<=p[a] && p[a]<=r[a][1]);
            for &a in &points {for &b in &points {
                assert!(children.iter().any(|c|
                    contains(c[0],a)&&contains(c[1],b) || contains(c[0],b)&&contains(c[1],a)));
            }}
            assert_eq!(children[0][0][axis][1],mid);
            assert_eq!(children[2][0][axis][0],mid);
        }
    }
    #[test]
    fn budget_refusal_preserves_all_closed_pairs_including_exchanged_endpoints() {
        let mut s=graph();
        for row in &mut s.control_points {for point in row {point.fill(0.);}}
        let points:Vec<_>=[0.,0.25,0.5,0.75,1.].into_iter().flat_map(|u|
            [0.,0.25,0.5,0.75,1.].into_iter().map(move|v|[u,v])).collect();
        let contains=|r:Rectangle,p:[f64;2]| (0..2).all(|a|r[a][0]<=p[a] && p[a]<=r[a][1]);
        for cells in 1..=12 {
            let report=qualify(&s,0.1,1e-4,Limits{cells,spans:1000}).unwrap();
            assert!(report.certificate.is_none());
            assert_eq!(report.cells,cells);
            assert_eq!(report.pending,report.unresolved_pairs.len());
            assert_eq!(report.uncertain,report.unresolved_pairs.last().copied());
            for &a in &points {for &b in &points {
                assert!(report.unresolved_pairs.iter().any(|c|
                    contains(c[0],a)&&contains(c[1],b) || contains(c[0],b)&&contains(c[1],a)),
                    "Missing original endpoint obligation after {cells} cells: {a:?}, {b:?}");
            }}
        }
    }
    #[test]
    fn unresolved_singular_corner_does_not_starve_other_original_pairs() {
        let mut s=graph();
        for row in &mut s.control_points {for point in row {point.fill(0.);}}
        let report=qualify(&s,0.1,1e-4,Limits{cells:64,spans:10000}).unwrap();
        assert!(report.certificate.is_none());
        assert_eq!(report.cells,64);
        assert_eq!(report.pending,report.unresolved_pairs.len());
        // A depth-first traversal spends all cells shrinking one corner;
        // level-order coverage keeps all unresolved parameter boxes useful.
        assert!(report.unresolved_pairs.iter().flatten().flatten()
            .all(|range| range[1]-range[0] >= 1./64.));
    }
    #[test]
    fn rectangle_cache_reuses_only_complete_original_coverage() {
        let s=graph();
        let mut cache=RectangleCache::new(&s,1e-6);
        let domain=[[0.,1.];2];
        assert!(cache.qualify(domain,0).is_err());
        let (paid,reused)=cache.qualify(domain,1).unwrap();
        assert!(paid.complete && !reused && paid.spans==1);
        let (free,reused)=cache.qualify(domain,0).unwrap();
        assert!(free.complete && reused && free.spans==0);
        assert_eq!(paid.cosine_squared_upper,free.cosine_squared_upper);
        assert!(free.mean_matrix.is_some());
        assert!(cache.qualify([[0.,0.5],[0.,1.]],0).is_err());
        let mut multiple=s.clone();
        multiple.control_points.push(multiple.control_points[2].clone());
        multiple.weights.push(vec![1.;2]);
        multiple.knots_u=vec![0.,0.,0.,0.5,1.,1.,1.];
        let mut partial=RectangleCache::new(&multiple,1e-6);
        for _ in 0..2 {
            let (report,reused)=partial.qualify(domain,1).unwrap();
            assert!(!report.complete && !reused && report.spans==1);
        }
        let (report,reused)=partial.qualify(domain,2).unwrap();
        assert!(report.complete && !reused && report.spans==2);
        assert!(partial.qualify(domain,0).unwrap().1);
    }
    #[test]
    fn original_normal_cache_retains_complete_rectangles_but_not_partial_pairs() {
        let s=graph();
        let pair=[[[0.,0.1],[0.,0.1]],[[0.8,0.9],[0.8,0.9]]];
        let mut cache=normal_alignment::OriginalNormalCache::new(&s);
        let (first,proof,hits)=cache.inspect_pair(pair,1e-6,1).unwrap();
        assert_eq!((first.spans,proof.is_some(),hits),(1,false,0));
        let (missing,proof,hits)=cache.inspect_pair(pair,1e-6,0).unwrap();
        assert_eq!((missing.spans,proof.is_some(),hits),(0,false,1));
        let (paid,proof,hits)=cache.inspect_pair(pair,1e-6,1).unwrap();
        assert_eq!((paid.spans,proof.is_some(),hits),(1,true,1));
        let (free,proof,hits)=cache.inspect_pair([pair[1],pair[0]],1e-6,0).unwrap();
        assert_eq!((free.spans,proof.is_some(),hits),(0,true,2));
        assert_eq!(paid.sine_squared_interval,free.sine_squared_interval);
        assert!(cache.inspect_pair([[[0.,1.01],[0.,1.]],pair[1]],1e-6,0).is_err());
        assert!(cache.inspect_pair(pair,f64::NAN,0).is_err());
        let foreign=s.clone();
        assert!(proof.unwrap().components(&foreign,[pair[1],pair[0]]).is_err());
        let (_,foreign_proof,hits)=normal_alignment::OriginalNormalCache::new(&foreign)
            .inspect_pair(pair,1e-6,0).unwrap();
        assert!(foreign_proof.is_none() && hits==0);
    }
    fn graph() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..2)
                        .map(|v| vec![i as f64 / 2., v as f64, if i == 2 { 2. } else { 0. }])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn factored_original_pole_normals_exclude_oblique_secants_but_refuse_foreign_source() {
        let s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.]; 2],
                vec![vec![0.5, 0., 0.]; 2],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let prepared = crate::factored_surface_normals::prepare(&s, false, 1_000_000).unwrap();
        let proof = prepared.certificate.unwrap();
        let pair = [[[0., 0.], [0., 1.]], [[0.5, 0.6], [0., 1.]]];
        let a = surface_distance::rectangle_bounds(&s, pair[0]).unwrap();
        let b = surface_distance::rectangle_bounds(&s, pair[1]).unwrap();
        assert!(
            !secant_exclusion(&s, pair, [&a, &b], 1e-6, 1, None)
                .unwrap()
                .0
        );
        let (excluded, used) = secant_exclusion(&s, pair, [&a, &b], 1e-6, 1, Some(&proof)).unwrap();
        assert!(excluded && used == 1);
        let cache = proof.pair_bounds(pair).unwrap();
        assert_eq!(secant_exclusion_cached(&s, pair, [&a, &b], 1e-6, 0,
            Some(&proof), Some(&cache), None).unwrap(), (true, 0));
        assert!(cache.components(&proof, [pair[0]; 2]).is_err());
        let other = s.clone();
        let other_proof = crate::factored_surface_normals::prepare(&other, false, 1_000_000)
            .unwrap().certificate.unwrap();
        assert!(cache.components(&other_proof, pair).is_err());
        let other_end = crate::factored_surface_normals::prepare(&s, true, 1_000_000)
            .unwrap();
        if let Some(other_end) = other_end.certificate {
            assert!(cache.components(&other_end, pair).is_err());
        }

        let foreign = s.clone();
        assert!(qualify_with_factored_normals(
            &foreign,
            0.1,
            1e-6,
            Limits {
                cells: 10,
                spans: 100
            },
            &proof
        )
        .is_err());
        let r = qualify_with_factored_normals(&s, 0.1, 1e-6, Limits { cells: 1, spans: 1 }, &proof)
            .unwrap();
        assert!(r.certificate.is_none() && r.pending > 0 && r.spans <= 1);
    }
    #[test]
    fn original_secant_direction_excludes_oblique_chords_and_refuses_overlap_or_no_work() {
        let s = graph();
        let pair = [[[0., 0.01], [0., 0.01]], [[0.1, 0.11], [0., 0.01]]];
        let a = surface_distance::rectangle_bounds(&s, pair[0]).unwrap();
        let b = surface_distance::rectangle_bounds(&s, pair[1]).unwrap();
        let (excluded, used) = secant_exclusion(&s, pair, [&a, &b], 1e-6, 100, None).unwrap();
        assert!(excluded);
        assert!(used > 0 && used <= 100);
        let (paid,cache)=normal_alignment::inspect_original_pair_cached(&s,pair,1e-6,100).unwrap();
        assert!(paid.spans>0);
        let cache=cache.unwrap();
        assert_eq!(secant_exclusion_cached(&s,pair,[&a,&b],1e-6,0,None,None,
            Some(&cache)).unwrap(),(true,0));
        let foreign=s.clone();
        assert!(secant_exclusion_cached(&foreign,pair,[&a,&b],1e-6,0,None,None,
            Some(&cache)).is_err());
        assert!(secant_exclusion_cached(&s,[pair[0];2],[&a,&a],1e-6,0,None,None,
            Some(&cache)).is_err());

        assert_eq!(
            secant_exclusion(&s, pair, [&a, &b], 1e-6, 0, None).unwrap(),
            (false, 0)
        );
        assert_eq!(
            secant_exclusion(&s, [pair[0]; 2], [&a, &a], 1e-6, 100, None).unwrap(),
            (false, 0)
        );
    }
    #[test]
    fn complete_subdivision_proves_strongly_curved_graph_and_missing_work_refuses() {
        let s = graph();
        assert!(surface_self_chord::qualify(&s, 1e-6, 1)
            .unwrap()
            .certificate
            .is_none());
        let r = qualify(
            &s,
            0.1,
            1e-6,
            Limits {
                cells: 10000,
                spans: 100000,
            },
        )
        .unwrap();
        eprintln!(
            "self adaptive {} cells={} spans={} intrinsic={} angular={} distance={} pending={}",
            r.reason,
            r.cells,
            r.spans,
            r.intrinsic_exclusions,
            r.angular_exclusions,
            r.distance_exclusions,
            r.pending
        );
        let c = r
            .certificate
            .expect("all original endpoint pairs must be covered");
        assert!(std::ptr::eq(c.surface(), &s));
        assert_eq!(c.minimum_mm(), 0.1);
        assert_eq!(c.max_sine_squared(), 1e-6);
        assert!(r.intrinsic_exclusions > 1);
        assert_eq!(r.pending, 0);
        assert!(r.uncertain.is_none());
        assert!(r.unresolved_pairs.is_empty());
        assert!(c
            .intrinsic_certificates()
            .iter()
            .all(|p| std::ptr::eq(p.surface(), &s)));
        for limits in [
            Limits {
                cells: 1,
                spans: 100000,
            },
            Limits {
                cells: 10000,
                spans: 1,
            },
        ] {
            let r = qualify(&s, 0.1, 1e-6, limits).unwrap();
            assert!(r.certificate.is_none());
            assert!(r.pending > 0);
            assert!(r.uncertain.is_some());
            assert_eq!(r.unresolved_pairs.len(), r.pending);
            assert_eq!(r.unresolved_pairs.last().copied(), r.uncertain);
            for pair in &r.unresolved_pairs {
                for rect in pair {
                    for axis in 0..2 {
                        assert!(rect[axis][0] >= 0. && rect[axis][0] <= rect[axis][1]
                            && rect[axis][1] <= 1.);
                    }
                }
            }
        }
    }
    #[test]
    fn rational_half_cylinder_with_short_exact_normal_chord_cannot_qualify() {
        let points = [[1., 0.], [1., 1.], [0., 1.], [-1., 1.], [-1., 0.]];
        let w = 0.5_f64.sqrt();
        let s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: points
                .iter()
                .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
                .collect(),
            weights: [1., w, 1., w, 1.].iter().map(|w| vec![*w; 2]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        // Original endpoints at U=0/1 have distance exactly 2 and normals
        // parallel to their connecting line, independent of rounded arc weight.
        let pair = [[[0., 0.001], [0.4, 0.401]], [[0.999, 1.], [0.4, 0.401]]];
        let a = surface_distance::rectangle_bounds(&s, pair[0]).unwrap();
        let b = surface_distance::rectangle_bounds(&s, pair[1]).unwrap();
        let (excluded, used) = secant_exclusion(&s, pair, [&a, &b], 1e-6, 100, None).unwrap();
        assert!(!excluded, "the exact diameter must remain a candidate");
        assert!(used > 0 && used <= 100);
        let r = qualify(
            &s,
            2.1,
            1e-6,
            Limits {
                cells: 2000,
                spans: 10000,
            },
        )
        .unwrap();
        eprintln!(
            "short original normal chord refusal {} cells={} spans={} pending={}",
            r.reason, r.cells, r.spans, r.pending
        );
        assert!(r.certificate.is_none());
        assert!(r.pending > 0);
        assert!(r.uncertain.is_some());
        assert!(r.cells <= 2000 && r.spans <= 10000);
        assert!(qualify(
            &s,
            0.,
            1e-6,
            Limits {
                cells: 2000,
                spans: 10000
            }
        )
        .is_err());
    }
}
