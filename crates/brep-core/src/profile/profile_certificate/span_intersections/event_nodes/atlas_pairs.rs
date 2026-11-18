//! Complete original-chart pair census with explicit unresolved obligations.
use super::*;
use std::sync::Arc;
#[derive(Clone, Copy, Debug)]
pub struct OriginalAtlasPairLimits {
    pub max_pairs: usize,
    pub max_span_work: usize,
    pub refinement_depth: usize,
    pub max_pair_depth: usize,
    pub max_subdivision_cells: usize,
}
impl Default for OriginalAtlasPairLimits {
    fn default() -> Self {
        Self {
            max_pairs: 1000000,
            max_span_work: 100000,
            refinement_depth: 2,
            max_pair_depth: 8,
            max_subdivision_cells: 100000,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum OriginalAtlasPairProof {
    SeparatedAxis {
        axis: usize,
        first_before_second: bool,
    },
    SeparatedControlGap {
        lower: f64,
    },
    CommonOriginalRectangle {
        domain: [[f64; 2]; 2],
        projection: [usize; 2],
        contraction_upper: f64,
        span_work: usize,
    },
    SeamGluedRectangle {
        sections: [Vec<[f64; 2]>; 2],
        projection: [[f64; 3]; 2],
        contraction_upper: f64,
        span_work: usize,
        parameter_scale: [f64; 2],
    },
    Subdivided {
        domains: [[[f64; 2]; 2]; 2],
        side: usize,
        axis: usize,
        cut: f64,
        children: Box<[OriginalAtlasPairProof; 2]>,
        span_work: usize,
    },
    Unresolved {
        span_work: usize,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct OriginalAtlasPair {
    pub charts: [usize; 2],
    pub proof: OriginalAtlasPairProof,
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalAtlasPairs {
    atlas: Arc<OwnedOriginalInjectiveChartAtlas>,
    limits: OriginalAtlasPairLimits,
    bounds: Vec<[[f64; 2]; 3]>,
    pairs: Vec<OriginalAtlasPair>,
    span_work: usize,
    subdivision_cells: usize,
}
fn derive(
    atlas: &OwnedOriginalInjectiveChartAtlas,
    limits: OriginalAtlasPairLimits,
) -> Result<(Vec<[[f64; 2]; 3]>, Vec<OriginalAtlasPair>, usize, usize)> {
    atlas.reverify_source()?;
    let chart=atlas.regularity().coverage().topology().seams().dictionary().chart();
    cover_source_pairs(atlas.regularity().coverage().topology().seams().surface(),
        atlas.charts(),chart.chart(),chart.wrapped(),limits)
}
// The caller owns a complete independently injective source-chart cover and
// actual supporting-surface seam proofs for every advertised wrapped axis.
pub(super) fn cover_source_pairs(
    surface:&nurbs_core::surface::Surface, charts:&[OriginalInjectiveChart],
    natural:[[f64;2];2],wrapped:[bool;2],limits:OriginalAtlasPairLimits,
) -> Result<(Vec<[[f64;2];3]>,Vec<OriginalAtlasPair>,usize,usize)> {
    surface.validate()?;
    if limits.max_pairs > 2000000
        || limits.max_span_work > 10000000
        || limits.refinement_depth > 6
        || limits.max_pair_depth > 16
        || limits.max_subdivision_cells > 1000000
    {
        return Err(refused("Invalid complete original chart-pair allowance"));
    }
    let n = charts.len();
    let count = n
        .checked_mul(n.saturating_sub(1))
        .and_then(|v| v.checked_div(2))
        .ok_or_else(|| refused("Original chart pair census overflow"))?;
    if count > limits.max_pairs {
        return Err(refused(
            "Complete original unordered chart-pair census exceeds allowance",
        ));
    }
    let mut bounds = Vec::new();
    for chart in charts {
        bounds.push(image_bounds(
            surface,
            chart.domain,
            "Original chart image enclosure is incomplete or nonfinite",
        )?);
    }
    let mut pairs = Vec::with_capacity(count);
    let mut prover = PairProver {
        atlas: AtlasSurface {
            surface,
            natural,
            wrapped,
        },
        limits,
        work: 0,
        created: 0,
    };
    for i in 0..n {
        for j in i + 1..n {
            let proof = prover.prove(
                [charts[i].domain, charts[j].domain],
                [bounds[i], bounds[j]],
                0,
            )?;
            pairs.push(OriginalAtlasPair {
                charts: [i, j],
                proof,
            });
        }
    }
    let (work, subdivision_cells) = (prover.work, prover.created);
    Ok((bounds, pairs, work, subdivision_cells))
}

/// The surface whose chart pairs are proved, with its natural parameter
/// domain and which directions wrap periodically.
#[derive(Clone, Copy)]
struct AtlasSurface<'a> {
    surface: &'a nurbs_core::surface::Surface,
    natural: [[f64; 2]; 2],
    wrapped: [bool; 2],
}
/// Parameter rectangles of the two charts of one pair.
type PairDomains = [[[f64; 2]; 2]; 2];
/// World bounding boxes of the two chart images of one pair.
type PairBounds = [[[f64; 2]; 3]; 2];
/// Span work one injectivity attempt may spend from the shared allowance.
const ATTEMPT_SPAN_WORK: usize = 100000;
/// Finite world bounding box of the surface image of one parameter rectangle.
fn image_bounds(
    surface: &nurbs_core::surface::Surface,
    domain: [[f64; 2]; 2],
    invalid: &'static str,
) -> Result<[[f64; 2]; 3]> {
    let b = nurbs_core::surface_distance::rectangle_bounds(surface, domain)?;
    if b.len() != 3
        || b.iter()
            .any(|a| a.iter().any(|v| !v.is_finite()) || a[0] > a[1])
    {
        return Err(refused(invalid));
    }
    Ok([b[0], b[1], b[2]])
}
/// The two image boxes are strictly apart along one world axis.
fn separated_axis(bounds: PairBounds) -> Option<OriginalAtlasPairProof> {
    for axis in 0..3 {
        if bounds[0][axis][1] < bounds[1][axis][0] {
            return Some(OriginalAtlasPairProof::SeparatedAxis {
                axis,
                first_before_second: true,
            });
        }
        if bounds[1][axis][1] < bounds[0][axis][0] {
            return Some(OriginalAtlasPairProof::SeparatedAxis {
                axis,
                first_before_second: false,
            });
        }
    }
    None
}
/// Smallest rectangle containing both chart domains.
fn common_domain(domains: PairDomains) -> [[f64; 2]; 2] {
    let [a, b] = domains;
    std::array::from_fn(|axis| [a[axis][0].min(b[axis][0]), a[axis][1].max(b[axis][1])])
}
/// Projection and contraction of a proved injectivity report.
fn injective_frame<P>(
    projection: Option<P>,
    contraction_upper: Option<f64>,
    spans: usize,
    [no_projection, no_contraction, invalid]: [&'static str; 3],
) -> Result<(P, f64)> {
    let projection = projection.ok_or_else(|| refused(no_projection))?;
    let contraction = contraction_upper.ok_or_else(|| refused(no_contraction))?;
    if !contraction.is_finite() || !(0. ..1.).contains(&contraction) || spans == 0 {
        return Err(refused(invalid));
    }
    Ok((projection, contraction))
}
/// Proves chart pairs against one surface, sharing the span work and the
/// subdivision cell allowance across all pairs and all recursion levels.
struct PairProver<'a> {
    atlas: AtlasSurface<'a>,
    limits: OriginalAtlasPairLimits,
    work: usize,
    created: usize,
}
impl PairProver<'_> {
    fn exhausted(&self) -> bool {
        self.work >= self.limits.max_span_work
    }
    /// The control nets of the two rectangles are a positive distance apart.
    fn control_gap(&self, domains: PairDomains) -> Result<Option<OriginalAtlasPairProof>> {
        let surface = self.atlas.surface;
        let lower = nurbs_core::surface_distance::verify_rectangle_separation_gap(
            surface, surface, domains,
        )?;
        Ok(lower
            .filter(|lower| lower.is_finite() && *lower > 0.)
            .map(|lower| OriginalAtlasPairProof::SeparatedControlGap { lower }))
    }
    /// The surface is injective on one rectangle that contains both charts.
    fn common_rectangle(&mut self, domains: PairDomains) -> Result<Option<OriginalAtlasPairProof>> {
        let domain = common_domain(domains);
        let remaining = (self.limits.max_span_work - self.work).min(ATTEMPT_SPAN_WORK);
        let report = nurbs_core::surface_injectivity::certify_on_refined(
            self.atlas.surface,
            domain,
            remaining,
            self.limits.refinement_depth,
        )?;
        if report.spans > remaining {
            return Err(refused(
                "Original chart pair exceeds shared actual span work",
            ));
        }
        self.work += report.spans;
        if !report.proven {
            return Ok(None);
        }
        let (projection, contraction_upper) = injective_frame(
            report.projection,
            report.contraction_upper,
            report.spans,
            [
                "Original pair common projection omitted",
                "Original pair contraction omitted",
                "Original pair common injectivity proof is invalid",
            ],
        )?;
        Ok(Some(OriginalAtlasPairProof::CommonOriginalRectangle {
            domain,
            projection,
            contraction_upper,
            span_work: report.spans,
        }))
    }
    /// The surface is injective on a rectangle that joins the two charts
    /// across a wrapped seam on one or both axes.
    fn seam_glued(&mut self, domains: PairDomains) -> Result<Option<OriginalAtlasPairProof>> {
        let AtlasSurface {
            surface,
            natural,
            wrapped,
        } = self.atlas;
        let [a, b] = domains;
        let direct = common_domain(domains);
        let across: [Option<Vec<[f64; 2]>>; 2] = std::array::from_fn(|axis| {
            if !wrapped[axis] {
                return None;
            }
            let (low, high) = if a[axis][1] < b[axis][0] {
                (a[axis], b[axis])
            } else if b[axis][1] < a[axis][0] {
                (b[axis], a[axis])
            } else {
                return None;
            };
            Some(vec![
                [high[0], natural[axis][1]],
                [natural[axis][0], low[1]],
            ])
        });
        for mask in 0..4 {
            if self.work == self.limits.max_span_work {
                break;
            }
            let mut sections = [vec![direct[0]], vec![direct[1]]];
            let mut available = true;
            for axis in (0..2).filter(|axis| mask & (1 << axis) != 0) {
                match &across[axis] {
                    Some(parts) => sections[axis] = parts.clone(),
                    None => available = false,
                }
            }
            if !available {
                continue;
            }
            let remaining = (self.limits.max_span_work - self.work).min(ATTEMPT_SPAN_WORK);
            let report = nurbs_core::surface_injectivity::certify_seam_glued_rectangle(
                surface,
                sections.clone(),
                remaining,
                self.limits.refinement_depth,
            )?;
            if report.spans > remaining {
                return Err(refused(
                    "Original glued pair exceeds shared actual span work",
                ));
            }
            self.work += report.spans;
            if !report.proven {
                continue;
            }
            let (projection, contraction_upper) = injective_frame(
                report.projection,
                report.contraction_upper,
                report.spans,
                [
                    "Original glued pair common projection omitted",
                    "Original glued pair contraction omitted",
                    "Original glued pair contraction is invalid",
                ],
            )?;
            return Ok(Some(OriginalAtlasPairProof::SeamGluedRectangle {
                sections,
                projection,
                contraction_upper,
                span_work: report.spans,
                parameter_scale: report.parameter_scale,
            }));
        }
        Ok(None)
    }
    /// The first proof that settles the pair without subdividing it.
    fn direct(
        &mut self,
        domains: PairDomains,
        bounds: PairBounds,
    ) -> Result<Option<OriginalAtlasPairProof>> {
        if let Some(proof) = separated_axis(bounds) {
            return Ok(Some(proof));
        }
        if let Some(proof) = self.control_gap(domains)? {
            return Ok(Some(proof));
        }
        if !self.exhausted()
            && let Some(proof) = self.common_rectangle(domains)?
        {
            return Ok(Some(proof));
        }
        if self.exhausted() {
            return Ok(None);
        }
        self.seam_glued(domains)
    }
    /// Where to halve one chart: the relatively wider axis first, if its
    /// midpoint is strictly inside.
    fn split(&self, domain: [[f64; 2]; 2]) -> Option<(usize, f64)> {
        let natural = self.atlas.natural;
        let width = std::array::from_fn::<_, 2, _>(|axis| {
            (domain[axis][1] - domain[axis][0]) / (natural[axis][1] - natural[axis][0])
        });
        let preferred = if width[0] >= width[1] { 0 } else { 1 };
        [preferred, 1 - preferred].into_iter().find_map(|axis| {
            let d = domain[axis];
            let cut = d[0] * 0.5 + d[1] * 0.5;
            (d[0] < cut && cut < d[1]).then_some((axis, cut))
        })
    }
    /// Halves one chart (alternating by depth) and proves both halves.
    fn subdivide(
        &mut self,
        domains: PairDomains,
        depth: usize,
        start_work: usize,
    ) -> Result<Option<OriginalAtlasPairProof>> {
        if depth >= self.limits.max_pair_depth
            || self.created + 2 > self.limits.max_subdivision_cells
            || self.exhausted()
        {
            return Ok(None);
        }
        let side = depth % 2;
        let Some((axis, cut)) = self.split(domains[side]) else {
            return Ok(None);
        };
        self.created += 2;
        let mut halves = [domains, domains];
        halves[0][side][axis][1] = cut;
        halves[1][side][axis][0] = cut;
        let mut children = Vec::with_capacity(2);
        for section in halves {
            let mut boxes = [[[0.; 2]; 3]; 2];
            for (k, bounds) in boxes.iter_mut().enumerate() {
                *bounds = image_bounds(
                    self.atlas.surface,
                    section[k],
                    "Original subdivided pair has an invalid source enclosure",
                )?;
            }
            children.push(self.prove(section, boxes, depth + 1)?);
        }
        let [Some(second), Some(first)] = [children.pop(), children.pop()] else {
            return Ok(None);
        };
        Ok(Some(OriginalAtlasPairProof::Subdivided {
            domains,
            side,
            axis,
            cut,
            children: Box::new([first, second]),
            span_work: self.work - start_work,
        }))
    }
    fn prove(
        &mut self,
        domains: PairDomains,
        bounds: PairBounds,
        depth: usize,
    ) -> Result<OriginalAtlasPairProof> {
        let start_work = self.work;
        if let Some(proof) = self.direct(domains, bounds)? {
            return Ok(proof);
        }
        if let Some(proof) = self.subdivide(domains, depth, start_work)? {
            return Ok(proof);
        }
        Ok(OriginalAtlasPairProof::Unresolved {
            span_work: self.work - start_work,
        })
    }
}
impl OriginalAtlasPairProof {
    pub fn is_proven(&self) -> bool {
        match self {
            Self::Unresolved { .. } => false,
            Self::Subdivided { children, .. } => children.iter().all(Self::is_proven),
            _ => true,
        }
    }
}
impl OwnedOriginalAtlasPairs {
    pub fn atlas(&self) -> &Arc<OwnedOriginalInjectiveChartAtlas> {
        &self.atlas
    }
    pub fn bounds(&self) -> &[[[f64; 2]; 3]] {
        &self.bounds
    }
    pub fn pairs(&self) -> &[OriginalAtlasPair] {
        &self.pairs
    }
    /// Union-rectangle Jacobian spans; enclosure/control queries are separately
    /// bounded by the complete chart and pair census.
    pub fn span_work(&self) -> usize {
        self.span_work
    }
    pub fn subdivision_cells(&self) -> usize {
        self.subdivision_cells
    }
    pub fn unresolved_pairs(&self) -> usize {
        self.pairs.iter().filter(|p| !p.proof.is_proven()).count()
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (bounds, pairs, work, subdivision_cells) = derive(&self.atlas, self.limits)?;
        if bounds != self.bounds
            || pairs != self.pairs
            || work != self.span_work
            || subdivision_cells != self.subdivision_cells
        {
            return Err(refused(
                "Complete original chart pair census or source proof changed",
            ));
        }
        Ok(())
    }
}
/// Census every unordered original chart pair, including contacts. Strict
/// image separation or injectivity of one complete original union rectangle
/// proves its obligation. Other cases, including unproved seam gluing, remain
/// explicit Unresolved; work exhaustion never deletes pairs or grants success.
pub fn own_original_atlas_pairs(
    atlas: Arc<OwnedOriginalInjectiveChartAtlas>,
    limits: OriginalAtlasPairLimits,
) -> Result<OwnedOriginalAtlasPairs> {
    let (bounds, pairs, span_work, subdivision_cells) = derive(&atlas, limits)?;
    Ok(OwnedOriginalAtlasPairs {
        atlas,
        limits,
        bounds,
        pairs,
        span_work,
        subdivision_cells,
    })
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalGlobalChartEmbedding {
    pairs: Arc<OwnedOriginalAtlasPairs>,
}
impl OwnedOriginalGlobalChartEmbedding {
    pub fn pairs(&self) -> &Arc<OwnedOriginalAtlasPairs> {
        &self.pairs
    }
    pub fn reverify_source(&self) -> Result<()> {
        self.pairs.reverify_source()?;
        if self.pairs.unresolved_pairs() != 0 {
            return Err(refused(
                "Original global chart embedding has unresolved patch-pair obligations",
            ));
        }
        Ok(())
    }
}
/// Admit global original-chart embedding only after the complete local cover
/// and EVERY unordered patch pair are freshly proved. Current sufficient pair
/// methods may leave periodic seams unresolved; that remains refusal, never
/// permission inferred from locally injective charts or declared periodicity.
pub fn own_original_global_chart_embedding(
    pairs: Arc<OwnedOriginalAtlasPairs>,
) -> Result<OwnedOriginalGlobalChartEmbedding> {
    let owner = OwnedOriginalGlobalChartEmbedding { pairs };
    owner.reverify_source()?;
    Ok(owner)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pair_census_replay_rejects_omission_fake_separation_bounds_and_work() {
        let surface = nurbs_core::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 0.5, 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            weights: vec![vec![1.; 2]; 3],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            periodic_u: false,
            periodic_v: false,
        };
        let v = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
        let wire = v
            .windows(2)
            .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
            .collect();
        let regions = Arc::new(
            assemble_original_filled_regions(
                &[wire],
                &[1],
                1e-12,
                Default::default(),
                Default::default(),
            )
            .unwrap(),
        );
        let chart = Arc::new(
            own_original_chart_vertices(
                regions.owner().cycles().graph().clone(),
                [[0., 1.], [0., 1.]],
                [false, false],
                Default::default(),
            )
            .unwrap(),
        );
        let dictionary =
            Arc::new(own_original_seam_dictionary(regions, chart, Default::default()).unwrap());
        let seams = Arc::new(bind_original_seams_to_surface(dictionary, &surface).unwrap());
        let topology = Arc::new(own_original_periodic_topology(seams).unwrap());
        let coverage =
            Arc::new(own_original_trim_chart_coverage(topology, Default::default()).unwrap());
        let regularity = Arc::new(own_original_regular_trim_surface(coverage, 64).unwrap());
        let atlas =
            Arc::new(own_original_injective_chart_atlas(regularity, Default::default()).unwrap());
        let c = own_original_atlas_pairs(atlas, Default::default()).unwrap();
        c.reverify_source().unwrap();
        let mut bad = c.clone();
        bad.pairs.clear();
        assert!(bad.reverify_source().is_err());
        assert!(own_original_global_chart_embedding(Arc::new(bad)).is_err());
        let mut bad = c.clone();
        bad.pairs[0].proof = OriginalAtlasPairProof::SeparatedAxis {
            axis: 2,
            first_before_second: true,
        };
        assert!(bad.reverify_source().is_err());
        let mut bad = c.clone();
        bad.bounds[0][0][1] = 99.;
        assert!(bad.reverify_source().is_err());
        let mut bad = c.clone();
        bad.span_work += 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = c;
        bad.limits.max_span_work = 0;
        assert!(bad.reverify_source().is_err());
    }
}
#[cfg(test)]
mod subdivision_tests {
    use super::*;
    #[test]
    fn pair_subdivision_covers_both_actual_domains_and_never_drops_a_failed_child() {
        let surface = nurbs_core::primitives::torus([0.; 3], 3., 1., 1.).unwrap();
        let domains = [
            [[0.125, 0.1875], [0.25, 0.5]],
            [[0.1875, 0.25], [0.5, 0.75]],
        ];
        let natural = [[0., 1.], [0., 4.]];
        let bounds = domains.map(|d| {
            let b = nurbs_core::surface_distance::rectangle_bounds(&surface, d).unwrap();
            [b[0], b[1], b[2]]
        });
        let limits = OriginalAtlasPairLimits {
            max_span_work: 1000000,
            refinement_depth: 4,
            ..Default::default()
        };
        let prover = |limits| PairProver {
            atlas: AtlasSurface {
                surface: &surface,
                natural,
                wrapped: [true, true],
            },
            limits,
            work: 0,
            created: 0,
        };
        let mut flat_prover = prover(OriginalAtlasPairLimits {
            max_pair_depth: 0,
            ..limits
        });
        let flat = flat_prover.prove(domains, bounds, 0).unwrap();
        assert!(!flat.is_proven());
        assert_eq!(flat_prover.created, 0);
        let mut full_prover = prover(limits);
        let proof = full_prover.prove(domains, bounds, 0).unwrap();
        assert!(proof.is_proven());
        assert!(full_prover.created > 0);
        fn cover(p: &OriginalAtlasPairProof, expected: [[[f64; 2]; 2]; 2]) {
            if let OriginalAtlasPairProof::Subdivided {
                domains,
                side,
                axis,
                cut,
                children,
                ..
            } = p
            {
                assert_eq!(*domains, expected);
                assert!(*side < 2 && *axis < 2);
                assert!(expected[*side][*axis][0] < *cut && *cut < expected[*side][*axis][1]);
                let mut a = expected;
                let mut b = expected;
                a[*side][*axis][1] = *cut;
                b[*side][*axis][0] = *cut;
                cover(&children[0], a);
                cover(&children[1], b);
            } else {
                assert!(p.is_proven());
            }
        }
        cover(&proof, domains);
        let mut missing = proof.clone();
        if let OriginalAtlasPairProof::Subdivided { children, .. } = &mut missing {
            children[0] = OriginalAtlasPairProof::Unresolved { span_work: 0 };
        }
        assert!(!missing.is_proven());
    }
}
