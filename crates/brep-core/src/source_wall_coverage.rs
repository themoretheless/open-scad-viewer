//! Continuous lower qualification over every original source face pair.
//! Curved self pairs require a continuous intrinsic exclusion; otherwise they remain unresolved.
use crate::{material_wall_coverage as geometry, source_face_gap, source_volume::Body};
use nurbs_core::{Error, Result};
pub struct Limits {
    pub pairs: usize,
    pub plane_controls: usize,
    pub normal_spans: usize,
    pub gap: source_face_gap::Limits,
    pub max_sine_squared: f64,
}
pub struct Pair {
    pub faces: [usize; 2],
    pub reason: &'static str,
    pub proven: bool,
}
pub struct NormalProductLimits {
    pub cells: usize,
    pub cells_per_pair: usize,
}
/// Optional exact boundary factors; all work is separate from regular normals.
pub struct FactoredNormalLimits {
    pub exact_work: u64,
    pub queries: usize,
    pub cells: usize,
    pub cells_per_pair: usize,
}
#[derive(Clone, Copy)]
pub struct RetainedGapLimits {
    pub cells:usize,pub spans:usize,pub domain_cells:usize,
    pub cells_per_pair:usize,pub spans_per_pair:usize,pub domain_cells_per_pair:usize,
    pub domain_cells_per_query:usize,pub tolerance_uv:f64,
}
#[derive(Clone, Copy)]
pub struct MixedPairLimits {
    pub domains: RetainedGapLimits,
    pub normal_spans: usize,
    pub normal_spans_per_pair: usize,
}
pub struct MixedPair {
    pub retained: RetainedPair,
    pub normal_spans: usize,
    pub normal_products: usize,
}
pub struct MixedReport {
    pub cells: usize, pub spans: usize, pub domain_cells: usize,
    pub normal_spans: usize, pub pairs: Vec<MixedPair>,
}
pub struct RetainedPair {
    pub faces:[usize;2],pub qualified:bool,pub reason:&'static str,
    pub cells:usize,pub spans:usize,pub domain_cells:usize,
    pub domain_queries:usize,pub endpoint_queries:usize,pub outside_products:usize,
    pub unresolved_products:Vec<[[[f64;2];2];2]>,
}
pub struct RetainedReport {
    pub cells:usize,pub spans:usize,pub domain_cells:usize,
    pub domain_queries:usize,pub endpoint_queries:usize,pub pairs:Vec<RetainedPair>,
}
pub struct Certificate<'a> {
    body: &'a Body,
    minimum_mm: f64,
    max_sine_squared: f64,
    gaps: Vec<source_face_gap::Certificate<'a>>,
    retained_gaps: Vec<crate::source_retained_face_gap::Certificate<'a>>,
    mixed_pairs: Vec<crate::source_wall_pair::Certificate<'a>>,
    self_chords: Vec<nurbs_core::surface_self_chord::Certificate<'a>>,
    adaptive_self: Vec<crate::source_wall_self_coverage::FaceCertificate<'a>>,
}
impl<'a> Certificate<'a> {
    pub fn body(&self) -> &'a Body {
        self.body
    }
    pub fn minimum_mm(&self) -> f64 {
        self.minimum_mm
    }
    pub fn max_sine_squared(&self) -> f64 {
        self.max_sine_squared
    }
    pub fn adaptive_self_certificates(
        &self,
    ) -> &[crate::source_wall_self_coverage::FaceCertificate<'a>] {
        &self.adaptive_self
    }
    pub fn self_chord_certificates(&self) -> &[nurbs_core::surface_self_chord::Certificate<'a>] {
        &self.self_chords
    }
    pub fn retained_gap_certificates(&self) -> &[crate::source_retained_face_gap::Certificate<'a>] { &self.retained_gaps }
    pub fn mixed_pair_certificates(&self) -> &[crate::source_wall_pair::Certificate<'a>] { &self.mixed_pairs }
    pub fn gap_certificates(&self) -> &[source_face_gap::Certificate<'a>] {
        &self.gaps
    }
}
pub struct Report<'a> {
    pub retained_gap:Option<RetainedReport>,
    pub mixed:Option<MixedReport>,
    pub certificate: Option<Certificate<'a>>,
    pub pairs: Vec<Pair>,
    pub total_pairs: usize,
    pub enumeration_complete: bool,
    pub plane_controls: usize,
    pub normal_spans: usize,
    pub normal_product_cells: usize,
    pub factored_exact_work: u64,
    pub factored_queries: usize,
    pub factored_product_cells: usize,
    pub cells: usize,
    pub spans: usize,
    pub reason: &'static str,
}
fn domain(s: &nurbs_core::surface::Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
/// Proves a lower threshold for aligned material chords, including unselected
/// walls, cavities, same-face pairs and root-ended trim regions. Exclusions and
/// distance bounds use complete original charts as conservative supersets.
pub fn qualify<'a>(body: &'a Body, minimum_mm: f64, limits: Limits) -> Result<Report<'a>> {
    qualify_with_self_facts(body, minimum_mm, limits, Vec::new())
}
/// Reuses only owned private self-face facts for this immutable Body. Public
/// reports cannot supply proof. Higher thresholds or broader angle requests
/// cannot be covered by a stale or narrower certificate.
pub fn qualify_with_self_facts<'a>(
    body: &'a Body,
    minimum_mm: f64,
    limits: Limits,
    facts: Vec<crate::source_wall_self_coverage::FaceCertificate<'a>>,
) -> Result<Report<'a>> {
    qualify_with_self_facts_and_normal_products(body,minimum_mm,limits,facts,
        NormalProductLimits{cells:0,cells_per_pair:0})
}
/// Optional bounded full-chart normal exclusions. Zero cells preserves the
/// original scheduling; only a completely excluded product proves a pair.
pub fn qualify_with_self_facts_and_normal_products<'a>(
    body:&'a Body, minimum_mm:f64, limits:Limits,
    facts:Vec<crate::source_wall_self_coverage::FaceCertificate<'a>>,
    products:NormalProductLimits,
) -> Result<Report<'a>> {
    qualify_with_self_facts_and_factored_normals(body,minimum_mm,limits,facts,products,
        FactoredNormalLimits{exact_work:0,queries:0,cells:0,cells_per_pair:0})
}
/// Reuses exact limiting directions only for their immutable original face.
/// Regular-normal proofs retain their scheduling and are never replaced.
pub fn qualify_with_self_facts_and_factored_normals<'a>(
    body:&'a Body, minimum_mm:f64, limits:Limits,
    facts:Vec<crate::source_wall_self_coverage::FaceCertificate<'a>>,
    products:NormalProductLimits, factors:FactoredNormalLimits,
)->Result<Report<'a>> {
    qualify_with_retained_gap_facts(body,minimum_mm,limits,facts,products,factors,Vec::new())
}
/// Add private original retained-domain certificates without replacing earlier proofs.
pub fn qualify_with_retained_gap_facts<'a>(
    body:&'a Body,minimum_mm:f64,limits:Limits,
    facts:Vec<crate::source_wall_self_coverage::FaceCertificate<'a>>,
    products:NormalProductLimits,factors:FactoredNormalLimits,
    retained_gaps:Vec<crate::source_retained_face_gap::Certificate<'a>>,
)->Result<Report<'a>> {
    qualify_with_retained_gap_pass(body,minimum_mm,limits,facts,products,factors,retained_gaps,None)
}
/// Additional fair bounded pass over unresolved original distinct-face products.
pub fn qualify_with_retained_gap_pass<'a>(
    body:&'a Body,minimum_mm:f64,limits:Limits,
    facts:Vec<crate::source_wall_self_coverage::FaceCertificate<'a>>,
    products:NormalProductLimits,factors:FactoredNormalLimits,
    retained_gaps:Vec<crate::source_retained_face_gap::Certificate<'a>>,
    retained:Option<RetainedGapLimits>,
)->Result<Report<'a>> {
    qualify_with_mixed_pair_facts(body,minimum_mm,limits,facts,products,factors,retained_gaps,retained,Vec::new())
}
/// Reuse complete retained product proofs bound to this original immutable body.
/// Certificates cover the requested threshold and normal-angle criterion.
pub fn qualify_with_mixed_pair_facts<'a>(
    body:&'a Body,minimum_mm:f64,limits:Limits,
    facts:Vec<crate::source_wall_self_coverage::FaceCertificate<'a>>,
    products:NormalProductLimits,factors:FactoredNormalLimits,
    retained_gaps:Vec<crate::source_retained_face_gap::Certificate<'a>>,
    retained:Option<RetainedGapLimits>,
    mixed_pairs:Vec<crate::source_wall_pair::Certificate<'a>>,
)->Result<Report<'a>> {
    qualify_with_mixed_pair_pass(body,minimum_mm,limits,facts,products,factors,retained_gaps,retained,mixed_pairs,None)
}
/// Additional bounded mixed criterion over the remaining original products.
pub fn qualify_with_mixed_pair_pass<'a>(
    body:&'a Body,minimum_mm:f64,limits:Limits,
    facts:Vec<crate::source_wall_self_coverage::FaceCertificate<'a>>,
    products:NormalProductLimits,factors:FactoredNormalLimits,
    mut retained_gaps:Vec<crate::source_retained_face_gap::Certificate<'a>>,
    retained:Option<RetainedGapLimits>,
    mut mixed_pairs:Vec<crate::source_wall_pair::Certificate<'a>>,
    mixed:Option<MixedPairLimits>,
)->Result<Report<'a>> {
    if let Some(m)=mixed {
        let r=m.domains;
        if [m.normal_spans,m.normal_spans_per_pair,r.cells,r.spans,r.cells_per_pair,r.spans_per_pair,r.domain_cells_per_pair,r.domain_cells_per_query].iter().any(|n|!(1..=100000).contains(n))
            ||!(1..=1000000).contains(&r.domain_cells)||!r.tolerance_uv.is_finite()||r.tolerance_uv<=0. {
            return Err(Error::new("BREP_SOURCE_MIXED_WALL_LIMIT","Choose bounded total and per-pair mixed proof work"));
        }
    }
    let mut mixed_faces=std::collections::HashSet::new();
    for proof in &mixed_pairs {
        let mut faces=proof.faces();faces.sort();
        if !std::ptr::eq(proof.body(),body)||proof.lower_mm()<minimum_mm
            ||proof.max_sine_squared()<limits.max_sine_squared||!mixed_faces.insert(faces) {
            return Err(Error::new("BREP_SOURCE_MIXED_WALL_FACT","Use unique original body proofs covering the requested threshold and angle"));
        }
    }
    if let Some(r)=retained {
        if [r.cells,r.spans,r.cells_per_pair,r.spans_per_pair,r.domain_cells_per_pair,r.domain_cells_per_query].iter().any(|n|!(1..=100000).contains(n))
            ||!(1..=1000000).contains(&r.domain_cells)||!r.tolerance_uv.is_finite()||r.tolerance_uv<=0. {
            return Err(Error::new("BREP_SOURCE_RETAINED_GAP_LIMIT","Choose positive bounded total and per-pair retained domain work"));
        }
    }
    let mut retained_pairs=std::collections::HashSet::new();
    for gap in &retained_gaps {
        let mut faces=gap.faces();faces.sort();
        if !std::ptr::eq(gap.body(),body)||gap.lower_mm()<minimum_mm
            ||!retained_pairs.insert(faces) {
            return Err(Error::new("BREP_SOURCE_RETAINED_WALL_FACT","Use unique owned retained face facts covering the requested threshold"));
        }
    }
    let enabled=factors.exact_work>0;
    if factors.exact_work>100_000_000 || factors.queries>100000 || factors.cells>100000
        || factors.cells_per_pair>100000 || [factors.queries,factors.cells,factors.cells_per_pair]
            .iter().any(|n|(*n>0)!=enabled) {
        return Err(Error::new("BREP_SOURCE_WALL_FACTORED_LIMIT","Choose bounded exact, query and product work"));
    }
    if products.cells>100000 || products.cells_per_pair>100000
        || (products.cells==0)!=(products.cells_per_pair==0) {
        return Err(Error::new("BREP_SOURCE_WALL_NORMAL_PRODUCT_LIMIT",
            "Choose bounded total and per-pair normal product work"));
    }
    if !minimum_mm.is_finite()
        || minimum_mm <= 0.
        || !(1..=100000).contains(&limits.pairs)
        || !(1..=1000000).contains(&limits.plane_controls)
        || !(1..=100000).contains(&limits.normal_spans)
        || !(1..=100000).contains(&limits.gap.cells)
        || !(1..=100000).contains(&limits.gap.spans)
        || !limits.max_sine_squared.is_finite()
        || !(0. ..1.).contains(&limits.max_sine_squared)
    {
        return Err(Error::new(
            "BREP_SOURCE_WALL_COVERAGE_INPUT",
            "Choose positive whole-wall threshold and bounded continuous work",
        ));
    }
    let regions = body.geometry().shell().regions().unwrap();
    let mut owned = std::collections::BTreeSet::new();
    for fact in &facts {
        if !std::ptr::eq(fact.body(), body)
            || fact.face() >= regions.len()
            || !owned.insert(fact.face())
            || fact.minimum_mm() < minimum_mm
            || fact.max_sine_squared() < limits.max_sine_squared
        {
            return Err(Error::new(
                "BREP_SOURCE_SELF_WALL_FACT",
                "Use unique owned self-face facts covering the requested threshold and angle",
            ));
        }
    }
    let surfaces: Vec<_> = regions.iter().map(|r| r.loops()[0][0].surface()).collect();
    let total_pairs = surfaces
        .len()
        .checked_mul(surfaces.len() + 1)
        .and_then(|n| n.checked_div(2))
        .ok_or_else(|| {
            Error::new(
                "BREP_SOURCE_WALL_COVERAGE_LIMIT",
                "Original face pair count overflow",
            )
        })?;
    let mut out = Report {
        certificate: None,
        retained_gap:None,
        mixed:None,
        pairs: Vec::with_capacity(total_pairs.min(limits.pairs)),
        total_pairs,
        enumeration_complete: false,
        plane_controls: 0,
        normal_spans: 0,
        normal_product_cells: 0,
        factored_exact_work:0, factored_queries:0, factored_product_cells:0,
        cells: 0,
        spans: 0,
        reason: "source-wall-coverage-unproven",
    };
    let planes: Vec<_> = surfaces
        .iter()
        .map(|s| {
            let (p, used) = geometry::plane(s, limits.plane_controls - out.plane_controls);
            out.plane_controls += used;
            p
        })
        .collect();
    let adjacency: std::collections::BTreeSet<_> = body.geometry().shell().uses().iter()
        .map(|uses| {
            let [a, b] = uses.map(|u| u.face);
            [a.min(b), a.max(b)]
        }).collect();
    let necessary = geometry::necessary_normal_sine(limits.max_sine_squared)?;
    let mut normal_caches: Vec<_> = surfaces.iter().map(|s|
        nurbs_core::normal_alignment::OriginalNormalCache::new(s)).collect();
    let mut prepared=Vec::with_capacity(surfaces.len());
    for surface in &surfaces {
        let mut certificate=None;
        if enabled {
            for end in [false,true] {
                if out.factored_exact_work==factors.exact_work {break;}
                // Unsupported charts provide no fact and no exclusion.
                if let Ok(r)=nurbs_core::factored_surface_normals::prepare(surface,end,
                    factors.exact_work-out.factored_exact_work) {
                    out.factored_exact_work+=r.work;
                    if let Some(c)=r.certificate {certificate=Some(c);break;}
                }
            }
        }
        prepared.push(certificate);
    }
    let mut gaps = Vec::new();
    let mut self_chords = Vec::new();
    'pairs: for a in 0..surfaces.len() {
        for b in a..surfaces.len() {
            if out.pairs.len() == limits.pairs {
                break 'pairs;
            }
            let mut pair = Pair {
                faces: [a, b],
                reason: "source-wall-pair-unproven",
                proven: false,
            };
            if a == b {
                if owned.contains(&a) {
                    pair.reason = "source-wall-self-certified";
                    pair.proven = true;
                } else if planes[a].is_some() && limits.max_sine_squared < 1. {
                    pair.reason = "source-wall-planar-self-excluded";
                    pair.proven = true;
                } else {
                    pair.reason = "source-wall-curved-self-unproven";
                    if out.normal_spans < limits.normal_spans {
                        let r = nurbs_core::surface_self_chord::qualify(
                            surfaces[a],
                            limits.max_sine_squared,
                            limits.normal_spans - out.normal_spans,
                        )?;
                        out.normal_spans += r.spans;
                        if let Some(certificate) = r.certificate {
                            pair.reason = "source-wall-curved-self-excluded";
                            pair.proven = true;
                            self_chords.push(certificate);
                        }
                    }
                }
            } else if matches!((planes[a],planes[b]),(Some(x),Some(y)) if geometry::same_plane(x,y))
            {
                pair.reason = "source-wall-coplanar-excluded";
                pair.proven = true;
            } else {
                if let Some(threshold) =
                    necessary
                {
                    let (before,after)=normal_caches.split_at_mut(b);
                    let r = before[a].inspect_other(&mut after[0],
                        [domain(surfaces[a]), domain(surfaces[b])], threshold,
                        limits.normal_spans - out.normal_spans)?;
                    out.normal_spans += r.spans;
                    if r.aligned == Some(false) {
                        pair.reason = "source-wall-normal-pair-excluded";
                        pair.proven = true;
                    }
                    if !pair.proven && out.normal_product_cells<products.cells {
                        let r=before[a].inspect_product(&mut after[0],
                            [domain(surfaces[a]),domain(surfaces[b])],threshold,
                            products.cells_per_pair.min(products.cells-out.normal_product_cells),
                            limits.normal_spans-out.normal_spans)?;
                        out.normal_product_cells+=r.cells;out.normal_spans+=r.spans;
                        if r.excluded {
                            pair.reason="source-wall-normal-pair-excluded";
                            pair.proven=true;
                        }
                    }
                }
                // An admitted shared boundary prevents full-chart clearance from
                // establishing a positive wall threshold for this pair. Keep it
                // unresolved unless normal exclusion already succeeded; reserve
                // gap subdivision for distinct faces without shared edge uses.
                // This scheduling exclusion is never a thickness certificate.
                let adjacent = adjacency.contains(&[a, b]);
                if !pair.proven && !adjacent && out.cells < limits.gap.cells && out.spans < limits.gap.spans {
                    let r = source_face_gap::qualify(
                        body,
                        [&[a], &[b]],
                        minimum_mm,
                        source_face_gap::Limits {
                            cells: limits.gap.cells - out.cells,
                            spans: limits.gap.spans - out.spans,
                        },
                    )?;
                    out.cells += r.cells;
                    out.spans += r.spans;
                    pair.reason = r.reason;
                    if let Some(c) = r.certificate {
                        pair.proven = true;
                        gaps.push(c);
                    }
                }
            }
            out.pairs.push(pair);
        }
    }
    // A second pass preserves every regular proof and its original scheduling.
    for pair in &mut out.pairs {
        let [a,b]=pair.faces;
        if a!=b && !pair.proven && enabled && out.factored_product_cells<factors.cells
            && (prepared[a].is_some() || prepared[b].is_some()) {
            if let Some(threshold)=necessary {
                let (before,after)=normal_caches.split_at_mut(b);
                let r=before[a].inspect_product_with_factors(&mut after[0],
                    [domain(surfaces[a]),domain(surfaces[b])],threshold,
                    factors.cells_per_pair.min(factors.cells-out.factored_product_cells),
                    limits.normal_spans-out.normal_spans,
                    [prepared[a].as_ref(),prepared[b].as_ref()],
                    factors.queries-out.factored_queries)?;
                out.factored_product_cells+=r.cells;out.factored_queries+=r.factored_queries;
                out.normal_spans+=r.spans;
                if r.excluded {pair.reason="source-wall-factored-normal-pair-excluded";pair.proven=true;}
            }
        }
    }
    for pair in &mut out.pairs {
        if !pair.proven && mixed_faces.contains(&pair.faces) {
            pair.proven=true;
            pair.reason="source-wall-mixed-product-qualified";
        }
        if !pair.proven && retained_pairs.contains(&pair.faces) {
            pair.proven=true;
            pair.reason="source-wall-retained-gap-qualified";
        }
    }
    if let Some(work)=retained {
        let mut diagnostics=RetainedReport{cells:0,spans:0,domain_cells:0,domain_queries:0,endpoint_queries:0,pairs:Vec::new()};
        for pair in out.pairs.iter_mut().filter(|p|!p.proven&&p.faces[0]!=p.faces[1]) {
            if diagnostics.cells==work.cells||diagnostics.spans==work.spans||diagnostics.domain_cells==work.domain_cells {break;}
            let r=crate::source_retained_face_gap::qualify(body,pair.faces,minimum_mm,crate::source_retained_face_gap::Limits{
                cells:(work.cells-diagnostics.cells).min(work.cells_per_pair),
                spans:(work.spans-diagnostics.spans).min(work.spans_per_pair),
                domain_cells:(work.domain_cells-diagnostics.domain_cells).min(work.domain_cells_per_pair),
                domain_cells_per_query:work.domain_cells_per_query,tolerance_uv:work.tolerance_uv})?;
            diagnostics.cells+=r.cells;diagnostics.spans+=r.spans;diagnostics.domain_cells+=r.domain_cells;
            diagnostics.domain_queries+=r.domain_queries;diagnostics.endpoint_queries+=r.endpoint_queries;
            let qualified=r.certificate.is_some();
            if let Some(c)=r.certificate {retained_gaps.push(c);pair.proven=true;pair.reason="source-wall-retained-gap-qualified";}
            diagnostics.pairs.push(RetainedPair{faces:pair.faces,qualified,reason:r.reason,cells:r.cells,spans:r.spans,domain_cells:r.domain_cells,domain_queries:r.domain_queries,endpoint_queries:r.endpoint_queries,outside_products:r.outside_products,unresolved_products:r.unresolved_products});
        }
        out.retained_gap=Some(diagnostics);
    }
    if let Some(work)=mixed {
        let d=work.domains;
        let mut diagnostics=MixedReport{cells:0,spans:0,domain_cells:0,normal_spans:0,pairs:Vec::new()};
        for pair in out.pairs.iter_mut().filter(|p|!p.proven&&p.faces[0]!=p.faces[1]) {
            if diagnostics.cells==d.cells||diagnostics.spans==d.spans
                ||diagnostics.domain_cells==d.domain_cells||diagnostics.normal_spans==work.normal_spans {break;}
            let r=crate::source_wall_pair::qualify(body,pair.faces,minimum_mm,crate::source_wall_pair::Limits{
                cells:(d.cells-diagnostics.cells).min(d.cells_per_pair),
                spans:(d.spans-diagnostics.spans).min(d.spans_per_pair),
                domain_cells:(d.domain_cells-diagnostics.domain_cells).min(d.domain_cells_per_pair),
                domain_cells_per_query:d.domain_cells_per_query,tolerance_uv:d.tolerance_uv,
                normal_spans:(work.normal_spans-diagnostics.normal_spans).min(work.normal_spans_per_pair),
                max_sine_squared:limits.max_sine_squared})?;
            diagnostics.cells+=r.cells;diagnostics.spans+=r.spans;
            diagnostics.domain_cells+=r.domain_cells;diagnostics.normal_spans+=r.normal_spans;
            let qualified=r.certificate.is_some();
            if let Some(c)=r.certificate {mixed_pairs.push(c);pair.proven=true;pair.reason="source-wall-mixed-product-qualified";}
            diagnostics.pairs.push(MixedPair{normal_spans:r.normal_spans,normal_products:r.normal_products,
                retained:RetainedPair{faces:pair.faces,qualified,reason:r.reason,cells:r.cells,spans:r.spans,
                    domain_cells:r.domain_cells,domain_queries:r.domain_queries,endpoint_queries:r.endpoint_queries,
                    outside_products:r.outside_products,unresolved_products:r.unresolved_products}});
        }
        out.mixed=Some(diagnostics);
    }
    out.enumeration_complete = out.pairs.len() == total_pairs;
    if out.enumeration_complete && out.pairs.iter().all(|p| p.proven) {
        out.certificate = Some(Certificate {
            body,
            minimum_mm,
            max_sine_squared: limits.max_sine_squared,
            gaps,
            retained_gaps,
            mixed_pairs,
            self_chords,
            adaptive_self: facts,
        });
        out.reason = "source-whole-wall-lower-qualified";
    } else if !out.enumeration_complete {
        out.reason = "source-wall-coverage-pair-limit";
    }
    Ok(out)
}
