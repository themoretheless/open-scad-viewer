//! Complete original-domain cover by independently injective source charts.
use super::*;
use std::sync::Arc;
#[derive(Clone, Copy, Debug)]
pub struct OriginalInjectiveAtlasLimits {
    /// All created rectangles, including split parents and the original grid.
    pub max_cells: usize,
    /// Shared actual original span evaluations across successful/failed trials.
    pub max_span_work: usize,
    pub max_depth: usize,
    pub proof_refinement_depth: usize,
}
impl Default for OriginalInjectiveAtlasLimits {
    fn default() -> Self {
        Self {
            max_cells: 4096,
            max_span_work: 100000,
            max_depth: 16,
            proof_refinement_depth: 2,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct OriginalInjectiveChart {
    pub domain: [[f64; 2]; 2],
    pub projection: [usize; 2],
    pub contraction_upper: f64,
    pub span_work: usize,
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalInjectiveChartAtlas {
    regularity: Arc<OwnedOriginalRegularTrimSurface>,
    limits: OriginalInjectiveAtlasLimits,
    charts: Vec<OriginalInjectiveChart>,
    created_cells: usize,
    span_work: usize,
}
fn derive(
    regularity: &OwnedOriginalRegularTrimSurface,
    limits: OriginalInjectiveAtlasLimits,
) -> Result<(Vec<OriginalInjectiveChart>, usize, usize)> {
    regularity.reverify_source()?;
    cover_surface(regularity.coverage().topology().seams().surface(), limits)
}
// Caller owns the original supporting surface; the cover always includes its
// complete original knot domain, never a sampled or fitted replacement.
pub(super) fn cover_surface(
    surface: &nurbs_core::surface::Surface,
    limits: OriginalInjectiveAtlasLimits,
) -> Result<(Vec<OriginalInjectiveChart>, usize, usize)> {
    surface.validate()?;
    if !(1..=100000).contains(&limits.max_cells)
        || !(1..=100000).contains(&limits.max_span_work)
        || limits.max_depth > 32
        || limits.proof_refinement_depth > 6
    {
        return Err(refused("Invalid original injective atlas work allowance"));
    }
    let mut pending = Vec::new();
    for u in surface.degree_u..surface.control_points.len() {
        if surface.knots_u[u] >= surface.knots_u[u + 1] {
            continue;
        }
        for v in surface.degree_v..surface.control_points[0].len() {
            if surface.knots_v[v] < surface.knots_v[v + 1] {
                pending.push((
                    [
                        [surface.knots_u[u], surface.knots_u[u + 1]],
                        [surface.knots_v[v], surface.knots_v[v + 1]],
                    ],
                    0usize,
                ));
            }
        }
    }
    let mut created = pending.len();
    let mut work = 0;
    let mut charts = Vec::new();
    if created == 0 || created > limits.max_cells {
        return Err(refused(
            "Original injective atlas initial source grid exceeds allowance",
        ));
    }
    while let Some((domain, depth)) = pending.pop() {
        let remaining = limits.max_span_work - work;
        if remaining == 0 {
            return Err(refused(
                "Original injective atlas shared source-span work exhausted",
            ));
        }
        let report = nurbs_core::surface_injectivity::certify_on_refined(
            surface,
            domain,
            remaining,
            limits.proof_refinement_depth,
        )?;
        if report.spans > remaining {
            return Err(refused(
                "Original injective chart exceeds shared actual span work",
            ));
        }
        work += report.spans;
        if report.proven {
            let projection = report.projection.ok_or_else(|| {
                refused("Original injective chart has no proved common projection")
            })?;
            let contraction = report
                .contraction_upper
                .ok_or_else(|| refused("Original injective chart has no contraction bound"))?;
            if !contraction.is_finite() || !(0. ..1.).contains(&contraction) || report.spans == 0 {
                return Err(refused(
                    "Original injective chart has an invalid common contraction proof",
                ));
            }
            charts.push(OriginalInjectiveChart {
                domain,
                projection,
                contraction_upper: contraction,
                span_work: report.spans,
            });
            continue;
        }
        if depth == limits.max_depth
            || created + 2 > limits.max_cells
            || work == limits.max_span_work
        {
            return Err(refused(
                "Original injective chart remains unresolved within atlas allowance",
            ));
        }
        let mut split = None;
        for axis in [depth % 2, 1 - depth % 2] {
            let midpoint = domain[axis][0] * 0.5 + domain[axis][1] * 0.5;
            if domain[axis][0] < midpoint && midpoint < domain[axis][1] {
                split = Some((axis, midpoint));
                break;
            }
        }
        let (axis, midpoint) = split.ok_or_else(|| {
            refused("Original injective atlas has no strict representable subdivision")
        })?;
        let mut first = domain;
        let mut second = domain;
        first[axis][1] = midpoint;
        second[axis][0] = midpoint;
        pending.push((second, depth + 1));
        pending.push((first, depth + 1));
        created += 2;
    }
    charts.sort_by(|a, b| {
        a.domain
            .iter()
            .flatten()
            .zip(b.domain.iter().flatten())
            .find_map(|(x, y)| {
                let order = x.partial_cmp(y).unwrap();
                (order != std::cmp::Ordering::Equal).then_some(order)
            })
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok((charts, created, work))
}
impl OwnedOriginalInjectiveChartAtlas {
    pub fn regularity(&self) -> &Arc<OwnedOriginalRegularTrimSurface> {
        &self.regularity
    }
    pub fn charts(&self) -> &[OriginalInjectiveChart] {
        &self.charts
    }
    pub fn created_cells(&self) -> usize {
        self.created_cells
    }
    pub fn span_work(&self) -> usize {
        self.span_work
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (charts, created, work) = derive(&self.regularity, self.limits)?;
        if charts != self.charts || created != self.created_cells || work != self.span_work {
            return Err(refused(
                "Original injective atlas source cover or work receipt changed",
            ));
        }
        Ok(())
    }
}
/// Cover EVERY original source knot rectangle by dyadic parameter subdivision
/// and a common-projection injectivity proof per accepted closed chart. Actual
/// source surface remains unchanged; no rounded extracted patch is authority.
/// Failed trials consume shared work. This is LOCAL injectivity plus complete
/// parameter coverage; cross-chart images may still intersect. Global periodic
/// quotient embedding requires separate patch-pair agreement/separation proofs.
pub fn own_original_injective_chart_atlas(
    regularity: Arc<OwnedOriginalRegularTrimSurface>,
    limits: OriginalInjectiveAtlasLimits,
) -> Result<OwnedOriginalInjectiveChartAtlas> {
    let (charts, created_cells, span_work) = derive(&regularity, limits)?;
    Ok(OwnedOriginalInjectiveChartAtlas {
        regularity,
        limits,
        charts,
        created_cells,
        span_work,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn injective_atlas_replay_rejects_missing_cover_projection_bounds_and_work() {
        let v = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
        let wire = v
            .windows(2)
            .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
            .collect();
        let surface = nurbs_core::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            weights: vec![vec![1.; 2]; 2],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            periodic_u: false,
            periodic_v: false,
        };
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
        let regularity = Arc::new(own_original_regular_trim_surface(coverage, 1).unwrap());
        let a = own_original_injective_chart_atlas(regularity, Default::default()).unwrap();
        a.reverify_source().unwrap();
        let mut bad = a.clone();
        bad.charts.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = a.clone();
        bad.charts[0].domain[0][1] = 0.5;
        assert!(bad.reverify_source().is_err());
        let mut bad = a.clone();
        bad.charts[0].projection = [0, 0];
        assert!(bad.reverify_source().is_err());
        let mut bad = a.clone();
        bad.charts[0].contraction_upper = 1.;
        assert!(bad.reverify_source().is_err());
        let mut bad = a.clone();
        bad.created_cells += 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = a.clone();
        bad.span_work += 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = a;
        bad.limits.max_span_work = 0;
        assert!(bad.reverify_source().is_err());
    }
}
