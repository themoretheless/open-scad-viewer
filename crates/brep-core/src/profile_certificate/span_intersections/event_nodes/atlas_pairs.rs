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
        let b = nurbs_core::surface_distance::rectangle_bounds(surface, chart.domain)?;
        if b.len() != 3
            || b.iter()
                .any(|a| a.iter().any(|v| !v.is_finite()) || a[0] > a[1])
        {
            return Err(refused(
                "Original chart image enclosure is incomplete or nonfinite",
            ));
        }
        bounds.push([b[0], b[1], b[2]]);
    }
    let mut pairs = Vec::with_capacity(count);
    let mut work = 0;
    let mut subdivision_cells = 0;
    for i in 0..n {
        for j in i + 1..n {
            let proof = prove_pair(
                surface,
                [charts[i].domain, charts[j].domain],
                [bounds[i], bounds[j]],
                natural,
                wrapped,
                limits,
                &mut work,
                &mut subdivision_cells,
                0,
            )?;
            pairs.push(OriginalAtlasPair {
                charts: [i, j],
                proof,
            });
        }
    }
    Ok((bounds, pairs, work, subdivision_cells))
}

fn prove_pair(
    surface: &nurbs_core::surface::Surface,
    domains: [[[f64; 2]; 2]; 2],
    bounds: [[[f64; 2]; 3]; 2],
    natural: [[f64; 2]; 2],
    wrapped: [bool; 2],
    limits: OriginalAtlasPairLimits,
    total_work: &mut usize,
    created: &mut usize,
    depth: usize,
) -> Result<OriginalAtlasPairProof> {
    let mut work = *total_work;
    let mut proof = None;
    let pair_start_work = work;
    for axis in 0..3 {
        if bounds[0][axis][1] < bounds[1][axis][0] {
            proof = Some(OriginalAtlasPairProof::SeparatedAxis {
                axis,
                first_before_second: true,
            });
            break;
        }
        if bounds[1][axis][1] < bounds[0][axis][0] {
            proof = Some(OriginalAtlasPairProof::SeparatedAxis {
                axis,
                first_before_second: false,
            });
            break;
        }
    }
    if proof.is_none() {
        if let Some(lower) = nurbs_core::surface_distance::verify_rectangle_separation_gap(
            surface,
            surface,
            [domains[0], domains[1]],
        )? {
            if lower.is_finite() && lower > 0. {
                proof = Some(OriginalAtlasPairProof::SeparatedControlGap { lower });
            }
        }
    }
    if proof.is_none() && work < limits.max_span_work {
        let a = domains[0];
        let b = domains[1];
        let domain =
            std::array::from_fn(|axis| [a[axis][0].min(b[axis][0]), a[axis][1].max(b[axis][1])]);
        let remaining = (limits.max_span_work - work).min(100000);
        let report = nurbs_core::surface_injectivity::certify_on_refined(
            surface,
            domain,
            remaining,
            limits.refinement_depth,
        )?;
        if report.spans > remaining {
            return Err(refused(
                "Original chart pair exceeds shared actual span work",
            ));
        }
        work += report.spans;
        if report.proven {
            let projection = report
                .projection
                .ok_or_else(|| refused("Original pair common projection omitted"))?;
            let contraction = report
                .contraction_upper
                .ok_or_else(|| refused("Original pair contraction omitted"))?;
            if !contraction.is_finite() || !(0. ..1.).contains(&contraction) || report.spans == 0 {
                return Err(refused("Original pair common injectivity proof is invalid"));
            }
            proof = Some(OriginalAtlasPairProof::CommonOriginalRectangle {
                domain,
                projection,
                contraction_upper: contraction,
                span_work: report.spans,
            });
        }
    }
    if proof.is_none() && work < limits.max_span_work {
        let a = domains[0];
        let b = domains[1];
        let direct: [Vec<[f64; 2]>; 2] = std::array::from_fn(|axis| {
            vec![[a[axis][0].min(b[axis][0]), a[axis][1].max(b[axis][1])]]
        });
        let options: [Option<Vec<[f64; 2]>>; 2] = std::array::from_fn(|axis| {
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
            if work == limits.max_span_work {
                break;
            }
            if (0..2).any(|axis| mask & (1 << axis) != 0 && options[axis].is_none()) {
                continue;
            }
            let sections = std::array::from_fn(|axis| {
                if mask & (1 << axis) != 0 {
                    options[axis].as_ref().unwrap().clone()
                } else {
                    direct[axis].clone()
                }
            });
            let remaining = (limits.max_span_work - work).min(100000);
            let report = nurbs_core::surface_injectivity::certify_seam_glued_rectangle(
                surface,
                sections.clone(),
                remaining,
                limits.refinement_depth,
            )?;
            if report.spans > remaining {
                return Err(refused(
                    "Original glued pair exceeds shared actual span work",
                ));
            }
            work += report.spans;
            if report.proven {
                let projection = report
                    .projection
                    .ok_or_else(|| refused("Original glued pair common projection omitted"))?;
                let contraction = report
                    .contraction_upper
                    .ok_or_else(|| refused("Original glued pair contraction omitted"))?;
                if !contraction.is_finite()
                    || !(0. ..1.).contains(&contraction)
                    || report.spans == 0
                {
                    return Err(refused("Original glued pair contraction is invalid"));
                }
                proof = Some(OriginalAtlasPairProof::SeamGluedRectangle {
                    sections,
                    projection,
                    contraction_upper: contraction,
                    span_work: report.spans,
                    parameter_scale: report.parameter_scale,
                });
                break;
            }
        }
    }

    *total_work = work;
    if let Some(proof) = proof {
        return Ok(proof);
    }
    if depth < limits.max_pair_depth
        && *created + 2 <= limits.max_subdivision_cells
        && work < limits.max_span_work
    {
        let side = depth % 2;
        let width = std::array::from_fn::<_, 2, _>(|axis| {
            (domains[side][axis][1] - domains[side][axis][0])
                / (natural[axis][1] - natural[axis][0])
        });
        let preferred = if width[0] >= width[1] { 0 } else { 1 };
        let mut split = None;
        for axis in [preferred, 1 - preferred] {
            let d = domains[side][axis];
            let cut = d[0] * 0.5 + d[1] * 0.5;
            if d[0] < cut && cut < d[1] {
                split = Some((axis, cut));
                break;
            }
        }
        if let Some((axis, cut)) = split {
            *created += 2;
            let mut first = domains;
            let mut second = domains;
            first[side][axis][1] = cut;
            second[side][axis][0] = cut;
            let child = |section: [[[f64; 2]; 2]; 2],
                         total_work: &mut usize,
                         created: &mut usize|
             -> Result<OriginalAtlasPairProof> {
                let mut boxes = [[[0.; 2]; 3]; 2];
                for k in 0..2 {
                    let values =
                        nurbs_core::surface_distance::rectangle_bounds(surface, section[k])?;
                    if values.len() != 3
                        || values
                            .iter()
                            .any(|a| a.iter().any(|v| !v.is_finite()) || a[0] > a[1])
                    {
                        return Err(refused(
                            "Original subdivided pair has an invalid source enclosure",
                        ));
                    }
                    boxes[k] = [values[0], values[1], values[2]];
                }
                prove_pair(
                    surface,
                    section,
                    boxes,
                    natural,
                    wrapped,
                    limits,
                    total_work,
                    created,
                    depth + 1,
                )
            };
            let a = child(first, total_work, created)?;
            let b = child(second, total_work, created)?;
            return Ok(OriginalAtlasPairProof::Subdivided {
                domains,
                side,
                axis,
                cut,
                children: Box::new([a, b]),
                span_work: *total_work - pair_start_work,
            });
        }
    }
    Ok(OriginalAtlasPairProof::Unresolved {
        span_work: work - pair_start_work,
    })
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
        let mut work = 0;
        let mut cells = 0;
        let flat = prove_pair(
            &surface,
            domains,
            bounds,
            natural,
            [true, true],
            OriginalAtlasPairLimits {
                max_pair_depth: 0,
                ..limits
            },
            &mut work,
            &mut cells,
            0,
        )
        .unwrap();
        assert!(!flat.is_proven());
        assert_eq!(cells, 0);
        let mut work = 0;
        let mut cells = 0;
        let proof = prove_pair(
            &surface,
            domains,
            bounds,
            natural,
            [true, true],
            limits,
            &mut work,
            &mut cells,
            0,
        )
        .unwrap();
        assert!(proof.is_proven());
        assert!(cells > 0);
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
