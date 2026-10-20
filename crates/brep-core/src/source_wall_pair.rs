//! Continuous wall-pair exclusion over retained original chart products.
//! Each product needs an outside, normal-direction or distance proof.
use crate::{source_contour_winding, source_volume::Body};
use nurbs_core::{
    Error, Result,
    surface::Surface,
    surface_distance::{enclosure_distance, rectangle_bounds},
    trim_domain::Location,
};
use std::collections::HashMap;
type Product = [[[f64; 2]; 2]; 2];
pub struct Limits {
    pub normal_spans: usize,
    pub max_sine_squared: f64,
    pub cells: usize,
    pub spans: usize,
    pub domain_cells: usize,
    pub domain_cells_per_query: usize,
    pub tolerance_uv: f64,
}
pub struct Certificate<'a,E = crate::source_shared_edge::SharedEdge> {
    body: &'a Body<E>,
    faces: [usize; 2],
    lower_mm: f64,
    max_sine_squared: f64,
}
impl<'a,E: crate::source_shared_chord_map::ChordCarrier> Certificate<'a,E> {
    pub fn body(&self) -> &'a Body<E> {
        self.body
    }
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn max_sine_squared(&self) -> f64 {
        self.max_sine_squared
    }
    pub fn lower_mm(&self) -> f64 {
        self.lower_mm
    }
}
pub struct Report<'a,E = crate::source_shared_edge::SharedEdge> {
    pub certificate: Option<Certificate<'a,E>>,
    pub normal_spans: usize,
    pub normal_products: usize,
    pub factored_queries: usize,
    pub cells: usize,
    pub spans: usize,
    pub domain_cells: usize,
    pub domain_queries: usize,
    pub endpoint_queries: usize,
    pub outside_products: usize,
    pub unresolved_products: Vec<Product>,
    pub reason: &'static str,
}
fn domain(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
fn spans(s: &Surface, r: [[f64; 2]; 2]) -> usize {
    let count = |k: &[f64], p: usize, n: usize, d: [f64; 2]| {
        (p..n)
            .filter(|&i| k[i] < k[i + 1] && k[i] <= d[1] && k[i + 1] >= d[0])
            .count()
    };
    count(&s.knots_u, s.degree_u, s.control_points.len(), r[0])
        * count(&s.knots_v, s.degree_v, s.control_points[0].len(), r[1])
}
/// A certificate requires complete coverage of the unchanged domain product.
/// Proximity, boundary membership and exhausted work never create a certificate.
pub fn qualify<'a,E: crate::source_shared_chord_map::ChordCarrier>(
    body: &'a Body<E>,
    faces: [usize; 2],
    minimum_mm: f64,
    limits: Limits,
) -> Result<Report<'a,E>> {
    qualify_with_factors(body,faces,minimum_mm,limits,[None,None],0)
}
/// Reuse only source-bound factors already prepared by the whole-wall gate.
/// Their directions at poles do not establish regular tangent planes.
pub(crate) fn qualify_with_factors<'a,E: crate::source_shared_chord_map::ChordCarrier>(
    body: &'a Body<E>, faces: [usize;2], minimum_mm:f64, limits:Limits,
    factors:[Option<&nurbs_core::factored_surface_normals::Certificate<'_>>;2],
    query_budget:usize,
) -> Result<Report<'a,E>> {
    qualify_with_prepared(body,faces,minimum_mm,limits,factors,query_budget,None)
}
/// Reuse exact original anchors whose preparation work was charged by the caller.
/// Full coverage is still required for a pair certificate.
pub fn qualify_with_anchors<'a,E: crate::source_shared_chord_map::ChordCarrier>(body:&'a Body<E>,faces:[usize;2],minimum_mm:f64,
    limits:Limits,anchors:&crate::source_wall_anchor::PreparedPair<'a,E>)->Result<Report<'a,E>> {
    qualify_with_prepared(body,faces,minimum_mm,limits,[None,None],0,Some(anchors))
}
pub(crate) fn qualify_with_prepared<'a,E: crate::source_shared_chord_map::ChordCarrier>(body:&'a Body<E>,faces:[usize;2],minimum_mm:f64,
    limits:Limits,factors:[Option<&nurbs_core::factored_surface_normals::Certificate<'_>>;2],
    query_budget:usize,anchors:Option<&crate::source_wall_anchor::PreparedPair<'a,E>>)->Result<Report<'a,E>> {
    if anchors.is_some_and(|a|!a.matches(body,faces)) {
        return Err(nurbs_core::Error::new("NURBS_INVALID_INPUT","Use anchors prepared for this original body and ordered face pair"));
    }
    let shell = body.geometry().shell();
    if faces[0] == faces[1]
        || faces.iter().any(|&f| f >= shell.faces().len())
        || !minimum_mm.is_finite()
        || minimum_mm <= 0.
        || [
            limits.cells,
            limits.spans,
            limits.domain_cells,
            limits.domain_cells_per_query,
        ]
        .iter()
        .any(|v| !(1..=100000).contains(v))
        || !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
    {
        return Err(Error::new(
            "BREP_SOURCE_WALL_PAIR",
            "Choose disjoint owned faces, a positive gap and bounded domain work",
        ));
    }
    if !(1..=100000).contains(&limits.normal_spans)
        || !limits.max_sine_squared.is_finite()
        || !(0. ..1.).contains(&limits.max_sine_squared)
    {
        return Err(Error::new(
            "BREP_SOURCE_WALL_PAIR",
            "Choose bounded normal work and an angle below one",
        ));
    }
    let threshold = crate::material_wall_coverage::necessary_normal_sine(limits.max_sine_squared)?;
    let regions = shell.regions().ok_or_else(|| {
        Error::new(
            "BREP_SOURCE_WALL_PAIR",
            "Original retained regions are required",
        )
    })?;
    // Reuse the exact retained-region sources that own prepared normal facts.
    let surfaces = faces.map(|f| regions[f].loops()[0][0].surface());
    let natural = surfaces.map(domain);
    let shared_boundary=crate::source_shared_chord_map::prepare_for_chords(body,faces,100000)?;
    let [mut first_normals, mut second_normals] =
        surfaces.map(nurbs_core::normal_alignment::OriginalNormalCache::new);
    let mut queue = vec![natural];
    let mut out = Report {
        certificate: None,
        normal_spans: 0,
        normal_products: 0,
        factored_queries: 0,
        cells: 0,
        spans: 0,
        domain_cells: 0,
        domain_queries: 0,
        endpoint_queries: 0,
        outside_products: 0,
        unresolved_products: Vec::new(),
        reason: "source-wall-pair-work-limit",
    };
    // The unchanged body and per-call tolerance bind every cached classification.
    // Splitting the other side often repeats exactly the same original rectangle.
    let mut domains: HashMap<(usize, [u64; 4]), (usize, Location)> = HashMap::new();
    while let Some(uv) = queue.pop() {
        if out.cells == limits.cells {
            out.unresolved_products.push(uv);
            out.unresolved_products.extend(queue);
            return Ok(out);
        }
        out.cells += 1;
        let mut normal_components=[None;2];
        if let Some(sine) = threshold {
            let (normals,queries) = match first_normals.inspect_other_with_factors(
                &mut second_normals,
                uv,
                sine,
                limits.normal_spans - out.normal_spans,
                if query_budget-out.factored_queries>=2 {factors} else {[None,None]},
                query_budget-out.factored_queries,
            ) {
                Ok(report) => report,
                Err(error) if error.code == "NURBS_NUMERIC_ERROR" => {
                    // Failed interval work supplies neither a direction proof nor
                    // a work count. Reserve its remaining allowance conservatively.
                    out.normal_spans = limits.normal_spans;
                    out.unresolved_products.push(uv);
                    out.unresolved_products.extend(queue);
                    out.reason = "source-wall-pair-numeric-limit";
                    return Ok(out);
                }
                Err(error) => return Err(error),
            };
            out.normal_spans += normals.spans;
            out.factored_queries += queries;
            normal_components=normals.normal_components;
            if normals.aligned == Some(false) {
                out.normal_products += 1;
                continue;
            }
        }
        let mut outside = false;
        for side in 0..2 {
            let key = (
                side,
                [
                    uv[side][0][0].to_bits(),
                    uv[side][0][1].to_bits(),
                    uv[side][1][0].to_bits(),
                    uv[side][1][1].to_bits(),
                ],
            );
            let allowance =
                (limits.domain_cells - out.domain_cells).min(limits.domain_cells_per_query);
            if let Some(&(paid, location)) = domains.get(&key) {
                // Uncertain results remain conservative; larger budgets may retry.
                if location != Location::Unresolved || allowance <= paid {
                    if location == Location::Outside {
                        outside = true;
                        break;
                    }
                    continue;
                }
            }
            if allowance == 0 {
                break;
            }
            let classification = source_contour_winding::classify(
                regions[faces[side]].loops(),
                uv[side],
                limits.tolerance_uv,
                allowance,
            )?;
            domains.insert(key, (allowance, classification.location));
            out.domain_cells += classification.cells;
            out.domain_queries += 1;
            out.endpoint_queries += classification.endpoint_queries;
            if classification.location == Location::Outside {
                outside = true;
                break;
            }
        }
        if outside {
            out.outside_products += 1;
            continue;
        }
        let work = spans(surfaces[0], uv[0]) + spans(surfaces[1], uv[1]);
        if work > limits.spans - out.spans {
            out.unresolved_products.push(uv);
            out.unresolved_products.extend(queue);
            return Ok(out);
        }
        out.spans += work;
        let boxes = [
            rectangle_bounds(surfaces[0], uv[0])?,
            rectangle_bounds(surfaces[1], uv[1])?,
        ];
        if enclosure_distance(&boxes[0], &boxes[1])?.0 >= minimum_mm {
            continue;
        }
        // Positional enclosures and paid normal boxes refer to this same
        // unchanged chart product. Oblique chords cannot witness wall thickness.
        let points=std::array::from_fn(|side|std::array::from_fn(|axis|boxes[side][axis]));
        if nurbs_core::normal_alignment::endpoint_chord_excluded(points,normal_components,limits.max_sine_squared)? {
            out.normal_products+=1;
            continue;
        }
        let chosen = (0..2)
            .flat_map(|side| (0..2).map(move |axis| (side, axis)))
            .filter(|&(side, axis)| {
                let r = uv[side][axis];
                let t = r[0] * 0.5 + r[1] * 0.5;
                r[1] - r[0] > limits.tolerance_uv && r[0] < t && t < r[1]
            })
            .max_by(|&(s, a), &(t, b)| {
                let width = |(s, a): (usize, usize)| {
                    (uv[s][a][1] - uv[s][a][0]) / (natural[s][a][1] - natural[s][a][0])
                };
                width((s, a)).total_cmp(&width((t, b)))
            });
        let Some((side, axis)) = chosen else {
            // At original chart corners the two point boxes lose their common
            // anchor dependency. Integrate source derivatives instead; every
            // proposed projection is checked by a source-bound cone proof.
            let mut corners=[[false;2];2];let mut anchored=true;
            for s in 0..2 {for a in 0..2 {
                if uv[s][a][0]==natural[s][a][0] {corners[s][a]=false;}
                else if uv[s][a][1]==natural[s][a][1] {corners[s][a]=true;}
                else {anchored=false;}
            }}
            let mut excluded=false;
            if let Some(shared)=shared_boundary.as_ref() {
                if (0..2).all(|side|shared.canonical_bounds(side,uv[side]).is_ok()) {
                    let geometric=(limits.spans-out.spans).min(64);
                    let normals=(limits.normal_spans-out.normal_spans).min(64);
                    match shared.exclude_normal_chords(uv,limits.max_sine_squared,geometric,normals) {
                        Ok((proof,used,normal_used))=>{out.spans+=used;out.normal_spans+=normal_used;excluded=proof;}
                        Err(e) if e.code=="NURBS_NUMERIC_ERROR"=>{out.spans+=geometric;out.normal_spans+=normals;}
                        Err(e)=>return Err(e),
                    }
                }
            }
            if excluded {out.normal_products+=1;continue;}

            if anchored {
                'projections:for x in -1..=1 {for y in -1..=1 {for z in -1..=1 {
                    if x==0&&y==0&&z==0 {continue;}
                    let allowance=(limits.spans-out.spans).min(64);
                    if allowance==0 {break 'projections;}
                    match nurbs_core::surface_self_chord::qualify_corner_chords(
                        surfaces,corners,uv,[x as f64,y as f64,z as f64],limits.max_sine_squared,allowance) {
                        Ok((proof,used))=>{out.spans+=used;if proof.is_some(){excluded=true;break 'projections;}}
                        Err(e) if e.code=="NURBS_INVALID_INPUT"=>{break 'projections;}
                        Err(e) if e.code=="NURBS_NUMERIC_ERROR"=>{
                            out.spans+=allowance;break 'projections;
                        }
                        Err(e)=>return Err(e),
                    }
                }}}
            }
            if !excluded {if let Some(paths)=anchors {
                let projections:Vec<_>=(-1..=1).flat_map(|x|(-1..=1).flat_map(move|y|(-1..=1).map(move|z|[x as f64,y as f64,z as f64])))
                    .filter(|p|*p!=[0.;3]).collect();
                let allowance=(limits.spans-out.spans).min(64);
                if allowance>0 {
                    // Conservative query charge includes supplied factors even if
                    // a path refusal prevents querying one of the normals.
                    let count=factors.iter().filter(|f|f.is_some()).count();
                    let normals=if count<=query_budget-out.factored_queries {out.factored_queries+=count;factors}else{[None,None]};
                    let query=paths.qualify_product_projections_with_normals(uv,&projections,limits.max_sine_squared,allowance,normals);
                    let query=match query {
                        Err(e) if e.code=="NURBS_INVALID_INPUT" && normals.iter().any(Option::is_some)=>
                            paths.qualify_product_projections(uv,&projections,limits.max_sine_squared,allowance),
                        other=>other,
                    };
                    match query {
                        Ok((proof,used))=>{out.spans+=used;excluded=proof.is_some();}
                        Err(e) if e.code=="NURBS_INVALID_INPUT"=>{}
                        Err(e) if e.code=="NURBS_NUMERIC_ERROR"=>{out.spans+=allowance;}
                        Err(e)=>return Err(e),
                    }
                }
            }}
            if excluded {out.normal_products+=1;continue;}
            out.unresolved_products.push(uv);
            continue;
        };
        let t = uv[side][axis][0] * 0.5 + uv[side][axis][1] * 0.5;
        let mut left = uv;
        let mut right = uv;
        left[side][axis][1] = t;
        right[side][axis][0] = t;
        queue.push(right);
        queue.push(left);
    }
    if out.unresolved_products.is_empty() {
        out.certificate = Some(Certificate {
            body,
            faces,
            lower_mm: minimum_mm,
            max_sine_squared: limits.max_sine_squared,
        });
        out.reason = "source-wall-pair-qualified";
    } else {
        out.reason = "source-wall-pair-resolution-limit";
    }
    Ok(out)
}
