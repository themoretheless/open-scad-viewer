//! Retained 2D rational profiles. No temporary solids or mesh construction.
use super::{Result, Value, encode, field, input};
use brep_core::planar_trim;
use nurbs_core::curve::Curve;
use value_codec::json;

fn active_spans(loops:&[Vec<Curve>],limit:usize)->Result<Vec<Vec<Curve>>>{
    let mut out=Vec::with_capacity(loops.len());let mut count=0;
    for wire in loops {let mut spans=Vec::with_capacity(wire.len());for c in wire{
        c.validate()?;let d=c.domain();let mut knots:Vec<_>=c.knots.iter().copied().filter(|v|*v>=d[0]&&*v<=d[1]).collect();knots.dedup();
        for p in knots.windows(2){count+=1;if count>limit{return Err(input("Retained profile active-span budget exceeded"));}
            spans.push(if knots.len()==2&&c.control_points.len()==c.degree+1&&!c.periodic{c.clone()}else{c.trim(p[0],p[1])?});
        }
    }out.push(spans);}Ok(out)
}
fn orient_general(loops:&[Vec<Curve>],tolerance:f64)->Result<Vec<Vec<Curve>>>{
    let spans=active_spans(loops,4096)?;
    let oriented=brep_core::bezier_profile::orient_even_odd(&spans,tolerance)?;
    loops.iter().zip(spans.iter().zip(&oriented)).map(|(wire,(before,after))|{
        let a=brep_core::bezier_profile::signed_area(before,tolerance)?;
        let b=brep_core::bezier_profile::signed_area(after,tolerance)?;
        if (a>0.)==(b>0.){Ok(wire.clone())}else{wire.iter().rev().map(|c|c.reverse()).collect()}
    }).collect()
}
pub(crate) fn profile(loops: Vec<Vec<Curve>>, tolerance: f64) -> Result<Value> {
    for curve in loops.iter().flatten() {
        curve.validate()?;
        if curve.control_points.iter().any(|p|p.len()!=2){return Err(input("Retained profiles require 2D control points"));}
    }
    let area = if loops.iter().flatten().any(|c| brep_core::bezier_profile::requires_general(c,tolerance)) {
        // Validate each active retained span while preserving the authored NURBS
        // definition in the returned profile, as the prism builder already does.
        let spans=active_spans(&loops,4096)?;
        // This owner proves the retained Bezier topology, including exact cusp
        // and algebraic endpoint cases unsupported by generic trim subdivision.
        // The Green integral remains a numerical area, not a volume certificate.
        brep_core::bezier_profile::validate_profile(&spans,tolerance).ok()
    } else {
        planar_trim::validate(&loops, tolerance)?;
        let area=loops.iter().try_fold(0., |sum, wire| {
            Ok::<_, super::Error>(sum + planar_trim::signed_area(wire, tolerance)?)
        })?;
        return Ok(json!({"kind":"brep-profile", "loops":loops, "areaMm2":area,
            "toleranceMm":tolerance, "geometryStatus":"numerical_uncertified"}));
    };
    let report = match brep_core::profile_region::inspect(&loops,tolerance,true) {
        Ok(report)=>report,
        Err(error)=>return match area {
            Some(area)=>Ok(json!({"kind":"brep-profile","loops":loops,"areaMm2":area,
                "toleranceMm":tolerance,"geometryStatus":"numerical_uncertified"})),
            None=>Err(error),
        },
    };
    let mut interval = [0_f64, 0_f64];
    for area in report.areas_mm2 {
        interval[0] = (interval[0] + area[0]).next_down();
        interval[1] = (interval[1] + area[1]).next_up();
    }
    if interval[0] <= 0. {
        return Err(input("Profile material area is unproven"));
    }
    Ok(json!({"kind":"brep-profile", "loops":loops,
        "areaMm2":area.unwrap_or(interval[0]*0.5+interval[1]*0.5),"areaIntervalMm2":interval,
        "toleranceMm":tolerance, "geometryStatus":"numerical_uncertified"}))
}

/// The private verified region already owns global winding and rounding proofs.
/// Summarize its area numerically without reapplying the unrelated analytic
/// arrangement's smaller span limit. Fresh downstream solid audits remain required.
fn verified_offset_profiles(regions:&brep_core::profile_certificate::VerifiedOffsetRegions,tolerance:f64)->Result<Vec<Value>> {
    regions.components().iter().map(|loops| {
        let area=loops.iter().try_fold(0.,|sum,wire|Ok::<_,super::Error>(sum+brep_core::bezier_profile::signed_area(wire,tolerance)?))?;
        if !area.is_finite()||area<=0. {return Err(input("Verified offset region numerical area is invalid"));}
        Ok(json!({"kind":"brep-profile","loops":loops,"areaMm2":area,"toleranceMm":tolerance,"geometryStatus":"numerical_uncertified"}))
    }).collect()
}

fn minkowski_result(c:brep_core::profile_certificate::MinkowskiRegionCertificate,r:brep_core::profile_certificate::VerifiedMinkowskiRegion,e:f64,tolerance:f64)->Result<Value>{
    let converted=r.components(e).and_then(|components|components.into_iter().map(|loops|profile(loops,tolerance)).collect::<Result<Vec<_>>>());
    let (components,refusal)=match converted{Ok(p)=>(p,None),Err(err)=>(Vec::new(),Some(err.to_string()))};
    Ok(json!({"components":components,"nominalExactRings":r.nominal_exact(),"innerExactRings":r.inner_exact(),"outerExactRings":r.outer_exact(),"certificate":c,"exactSetResult":r.exact_set_result(),"geometryStatus":"bounded_approximation","regionStatus":"inclusion_verified","modelingStatus":if refusal.is_some(){"refused"}else{"validated"},"modelingRefusal":refusal}))
}

pub fn dispatch(v: Value) -> Result<Value> {
    let tolerance = match v.get("toleranceMm") {
        Some(_) => field::<f64>(&v, "toleranceMm")?,
        None => 1e-7,
    };
    match v["op"].as_str() {
        Some("brep_profile_intersections") => brep_core::profile_intersections::inspect(
            &field::<Vec<Vec<Curve>>>(&v, "loops")?,
            tolerance,
            if v.get("maxPairs").is_some() {
                field(&v, "maxPairs")?
            } else {
                128
            },
            if v.get("maxBoxes").is_some() {
                field(&v, "maxBoxes")?
            } else {
                32768
            },
        ),
        Some("brep_curve_certify_image_agreement") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;
            encode(brep_core::profile_certificate::certify_curve_image_agreement(&a,&b,field(&v,"firstDomain")?,field(&v,"secondDomain")?)?)
        }
        Some("brep_curve_verify_image_agreement_certificate") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;let c=field(&v,"certificate")?;
            let r=brep_core::profile_certificate::verify_curve_image_agreement(&a,&b,field(&v,"firstDomain")?,field(&v,"secondDomain")?,&c)?;
            Ok(json!({"status":"verified","scope":"represented-curve-image-set-agreement","sourceCells":r.source_cells,"coverageSections":r.coverage_sections}))
        }
        Some("brep_profile_minkowski_sum") => {
            let a:Vec<Vec<Curve>>=field(&v,"a")?;let b:Vec<Vec<Curve>>=field(&v,"b")?;let e:f64=field(&v,"maximumSourceDeviationMm")?;
            let(c,r)=brep_core::profile_certificate::certify_minkowski_sum(&a,&b,e)?;minkowski_result(c,r,e,tolerance)
        }
        Some("brep_profile_minkowski_offset") => {
            let loops:Vec<Vec<Curve>>=field(&v,"loops")?;let e:f64=field(&v,"maximumSourceDeviationMm")?;
            let(c,r)=brep_core::profile_certificate::certify_minkowski_disk_offset(&loops,field(&v,"distanceMm")?,e)?;minkowski_result(c,r,e,tolerance)
        }
        Some("brep_profile_verify_minkowski_certificate") => {
            let a:Vec<Vec<Curve>>=field(&v,"a")?;let b:Vec<Vec<Curve>>=field(&v,"b")?;let c=field(&v,"certificate")?;
            let r=brep_core::profile_certificate::verify_minkowski_region(&a,&b,&c)?;
            Ok(json!({"status":"verified","scope":"represented-filled-minkowski-region-inclusions","exactSetResult":r.exact_set_result(),"nominalExactRings":r.nominal_exact(),"innerExactRings":r.inner_exact(),"outerExactRings":r.outer_exact()}))
        }
        Some("brep_curve_certify_span_intersections") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;
            encode(brep_core::profile_certificate::certify_span_intersections(&a,&b,field(&v,"selfPair")?,field(&v,"maximumParameterWidth")?)?)
        }
        Some("brep_curve_verify_span_intersections_certificate") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;let c=field(&v,"certificate")?;
            let r=brep_core::profile_certificate::verify_span_intersections(&a,&b,&c)?;
            Ok(json!({"status":"verified","scope":"represented-multispan-curve-intersection-components","events":r.events(),"singularParameters":r.singular_parameters(),"overlaps":r.overlaps(),"algebraicLoci":r.algebraic_loci(),"stationarySpans":r.stationary_spans(),"branchDecompositions":r.branch_decompositions(),"finiteEventsExhaustive":r.algebraic_loci().is_empty()}))
        }
        Some("brep_curve_certify_algebraic_intersections") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;
            encode(brep_core::profile_certificate::certify_algebraic_intersections(&a,&b,field(&v,"selfPair")?,field(&v,"maximumParameterWidth")?)?)
        }
        Some("brep_curve_verify_algebraic_intersections_certificate") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;let c=field(&v,"certificate")?;
            let r=brep_core::profile_certificate::verify_algebraic_intersections(&a,&b,&c)?;
            Ok(json!({"status":"verified","scope":"represented-algebraic-curve-intersection-set","events":r.events(),"singularParameters":r.singular_parameters(),"stationaryCensusComplete":r.stationary_census_complete()}))
        }
        Some("brep_curve_certify_multispan_agreement") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;
            encode(brep_core::profile_certificate::certify_multispan_agreement(&a,&b,field(&v,"firstDomain")?,field(&v,"secondDomain")?)?)
        }
        Some("brep_curve_verify_multispan_agreement_certificate") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;let c=field(&v,"certificate")?;
            let r=brep_core::profile_certificate::verify_multispan_agreement(&a,&b,field(&v,"firstDomain")?,field(&v,"secondDomain")?,&c)?;
            Ok(json!({"status":"verified","scope":"represented-multispan-curve-image-agreement","commonSpans":r.common_spans()}))
        }
        Some("brep_profile_offset_regions_certified") => {
            let loops:Vec<Vec<Curve>>=field(&v,"loops")?;let d:f64=field(&v,"distanceMm")?;let e:f64=field(&v,"maximumDeviationMm")?;let l:f64=field(&v,"miterLimitMm")?;
            let c=brep_core::profile_certificate::certify_offset_regions(&loops,d,e,&field::<String>(&v,"joinStyle")?,l)?;
            let r=brep_core::profile_certificate::verify_offset_regions(&loops,d,e,l,&c)?;
            let components=verified_offset_profiles(&r,tolerance)?;
            Ok(json!({"components":components,"certificate":c,"distanceMm":d,"maximumDeviationMm":e,"geometryStatus":"bounded_approximation","regularizationStatus":"verified"}))
        }
        Some("brep_profile_verify_offset_regions_certificate") => {
            let loops:Vec<Vec<Curve>>=field(&v,"loops")?;let c=field(&v,"certificate")?;
            let r=brep_core::profile_certificate::verify_offset_regions(&loops,field(&v,"distanceMm")?,field(&v,"maximumDeviationMm")?,field(&v,"miterLimitMm")?,&c)?;
            Ok(json!({"status":"verified","scope":"represented-offset-positive-region-regularization","components":r.components().len(),"rings":r.components().iter().map(Vec::len).sum::<usize>()}))
        }
        Some("brep_curve_certify_nonlinear_agreement") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;
            encode(brep_core::profile_certificate::certify_nonlinear_curve_agreement(&a,&b,field(&v,"firstDomain")?,field(&v,"secondDomain")?)?)
        }
        Some("brep_curve_verify_nonlinear_agreement_certificate") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;let c=field(&v,"certificate")?;
            let r=brep_core::profile_certificate::verify_nonlinear_curve_agreement(&a,&b,field(&v,"firstDomain")?,field(&v,"secondDomain")?,&c)?;
            Ok(json!({"status":"verified","scope":"represented-curve-nonlinear-image-agreement","commonSpans":r.common_spans()}))
        }
        Some("brep_curve_certify_intersections") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;
            encode(brep_core::profile_certificate::certify_curve_intersections(&a,&b,field(&v,"selfPair")?)?)
        }
        Some("brep_curve_verify_intersections_certificate") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;let c=field(&v,"certificate")?;
            let r=brep_core::profile_certificate::verify_curve_intersections(&a,&b,&c)?;
            Ok(json!({"status":"verified","scope":"represented-isolated-curve-intersection-set","events":r.events(),"singularParameters":r.singular_parameters()}))
        }
        Some("brep_profile_offset_wire_certified") => {
            let curves:Vec<Curve>=field(&v,"curves")?;let distance:f64=field(&v,"distanceMm")?;let deviation:f64=field(&v,"maximumDeviationMm")?;
            let c=brep_core::profile_certificate::certify_offset_wire(&curves,distance,deviation,field(&v,"closed")?,&field::<String>(&v,"joinStyle")?,field(&v,"miterLimitMm")?)?;
            Ok(json!({"curves":c.curves()?,"certificate":c,"distanceMm":distance,"maximumDeviationMm":deviation,"geometryStatus":"bounded_approximation"}))
        }
        Some("brep_profile_verify_offset_wire_certificate") => {
            let curves:Vec<Curve>=field(&v,"curves")?;let c=field(&v,"certificate")?;
            let r=brep_core::profile_certificate::verify_offset_wire(&curves,field(&v,"distanceMm")?,field(&v,"maximumDeviationMm")?,field(&v,"miterLimitMm")?,&c)?;
            Ok(json!({"status":"verified","scope":"represented-joined-normal-offset-wire-approximation","segments":r.segments(),"maximumDeviationMm":r.maximum_deviation_mm()}))
        }
        Some("brep_curve_certify_projective_agreement") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;
            encode(brep_core::profile_certificate::certify_projective_curve_agreement(&a,&b,field(&v,"firstDomain")?,field(&v,"secondDomain")?,field(&v,"parameterScale")?)?)
        }
        Some("brep_curve_verify_projective_agreement_certificate") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;let c=field(&v,"certificate")?;
            let verified=brep_core::profile_certificate::verify_projective_curve_agreement(&a,&b,field(&v,"firstDomain")?,field(&v,"secondDomain")?,field(&v,"parameterScale")?,&c)?;
            Ok(json!({"status":"verified","scope":"represented-curve-projective-parameter-agreement","commonSpans":verified.common_spans()}))
        }
        Some("brep_curve_certify_endpoint_contacts") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;
            encode(brep_core::profile_certificate::certify_endpoint_contacts(&a,&b)?)
        }
        Some("brep_curve_verify_endpoint_contacts_certificate") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;let c=field(&v,"certificate")?;
            let verified=brep_core::profile_certificate::verify_endpoint_contacts(&a,&b,&c)?;
            Ok(json!({"status":"verified","scope":"represented-endpoint-intersection-set","commonEndpoints":verified.common_endpoints()}))
        }
        Some("brep_curve_offset_certified") => {
            let curve:Curve=field(&v,"curve")?;let distance:f64=field(&v,"distanceMm")?;let deviation:f64=field(&v,"maximumDeviationMm")?;
            let c=brep_core::profile_certificate::certify_offset_curve(&curve,distance,deviation)?;
            Ok(json!({"curves":c.curves()?,"certificate":c,"distanceMm":distance,"maximumDeviationMm":deviation,"geometryStatus":"bounded_approximation"}))
        }
        Some("brep_curve_verify_offset_certificate") => {
            let curve:Curve=field(&v,"curve")?;let c=field(&v,"certificate")?;
            let verified=brep_core::profile_certificate::verify_offset_curve(&curve,field(&v,"distanceMm")?,field(&v,"maximumDeviationMm")?,&c)?;
            Ok(json!({"status":"verified","scope":"represented-normal-offset-curve-approximation","segments":verified.segments(),"maximumDeviationMm":verified.maximum_deviation_mm()}))
        }
        Some("brep_curve_certify_agreement") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;
            encode(brep_core::profile_certificate::certify_curve_agreement(&a,&b,field(&v,"firstDomain")?,field(&v,"secondDomain")?)?)
        }
        Some("brep_curve_verify_agreement_certificate") => {
            let a:Curve=field(&v,"first")?;let b:Curve=field(&v,"second")?;
            let c=field(&v,"certificate")?;
            let verified=brep_core::profile_certificate::verify_curve_agreement(&a,&b,field(&v,"firstDomain")?,field(&v,"secondDomain")?,&c)?;
            Ok(json!({"status":"verified","scope":"represented-curve-affine-parameter-agreement","commonSpans":verified.common_spans()}))
        }
        Some("brep_profile_certify_measure") => {
            let loops: Vec<Vec<Curve>> = field(&v,"loops")?;
            let width: f64 = field(&v,"maximumAreaWidthMm2")?;
            encode(brep_core::profile_certificate::certify_measure(&loops,tolerance,width)?)
        }
        Some("brep_profile_verify_measure_certificate") => {
            let loops: Vec<Vec<Curve>> = field(&v,"loops")?;
            let certificate: brep_core::profile_certificate::ProfileMeasureCertificate=field(&v,"certificate")?;
            let verified=brep_core::profile_certificate::verify_measure(&loops,tolerance,&certificate)?;
            Ok(json!({"status":"verified","scope":"represented-profile-topology-and-area",
                "spans":verified.profile().spans(),"pieces":verified.profile().pieces(),
                "nestingDepth":verified.profile().nesting_depth(),"areaMm2":verified.area_mm2()}))
        }
        Some("brep_profile_certify") => {
            let loops: Vec<Vec<Curve>> = field(&v,"loops")?;
            encode(brep_core::profile_certificate::certify(&loops,tolerance)?)
        }
        Some("brep_profile_verify_certificate") => {
            let loops: Vec<Vec<Curve>> = field(&v,"loops")?;
            let certificate: brep_core::profile_certificate::ProfileCertificate=field(&v,"certificate")?;
            let verified=brep_core::profile_certificate::verify(&loops,tolerance,&certificate)?;
            Ok(json!({"status":"verified","scope":"represented-profile-topology",
                "spans":verified.spans(),"pieces":verified.pieces(),"nestingDepth":verified.nesting_depth()}))
        }
        Some("brep_profile_normalize") => {
            let loops:Vec<Vec<Curve>>=field(&v,"loops")?;
            profile(brep_core::bezier_profile::normalize(&active_spans(&loops,1022)?,tolerance)?,tolerance)
        },
        Some("brep_profile_cleanup") => {
            let loops: Vec<Vec<Curve>> = field(&v, "loops")?;
            let deviation: f64 = field(&v, "deviation")?;
            if !deviation.is_finite()
                || !(0.0001..=10.).contains(&deviation)
                || loops.is_empty()
                || loops.len() > 32
                || loops.iter().map(Vec::len).sum::<usize>() > 1022
            {
                return Err(input("Invalid retained cleanup options"));
            }
            let mut rings = Vec::with_capacity(loops.len());
            let mut total = 0;
            for wire in loops {
                let mut ring = Vec::with_capacity(wire.len() * 8);
                let mut endpoint: Option<[f64; 2]> = None;
                for curve in wire {
                    if curve.control_points.iter().any(|p| p.len() != 2) {
                        return Err(input("Cleanup requires planar profile curves"));
                    }
                    let report = nurbs_core::tessellation::curve_tessellation::tessellate(
                        &curve,
                        deviation * 0.5,
                        8192,
                    )?;
                    if !report.within_tolerance {
                        return Err(input(
                            "Profile cleanup tessellation did not meet its tolerance",
                        ));
                    }
                    for segment in report.segments {
                        let a = [segment.points[0][0], segment.points[0][1]];
                        let b = [segment.points[1][0], segment.points[1][1]];
                        if endpoint.is_some_and(|p| (p[0] - a[0]).hypot(p[1] - a[1]) > 1e-7) {
                            return Err(input("Retained cleanup requires connected curves"));
                        }
                        ring.push(a);
                        endpoint = Some(b);
                        total += 1;
                        if total > 20000 {
                            return Err(input("Retained cleanup exceeds 20000 samples"));
                        }
                    }
                }
                if ring.len() < 3
                    || endpoint.is_none_or(|p| (p[0] - ring[0][0]).hypot(p[1] - ring[0][1]) > 1e-7)
                {
                    return Err(input("Retained cleanup requires closed boundaries"));
                }
                rings.push(
                    planar_geometry::path::BezierPath::from_polygon(&ring, true)?
                        .simplify(deviation * 0.5)?
                        .to_ring(deviation * 0.5)?,
                );
            }
            let rings = planar_geometry::rings::normalize(
                &rings,
                planar_geometry::tessellation::FillRule::EvenOdd,
            )?;
            let loops = rings
                .into_iter()
                .map(brep_core::sketch::polygon_wire)
                .collect::<Result<Vec<_>>>()?;
            let loops = planar_trim::orient_even_odd(&loops, 1e-7)?;
            profile(loops, 1e-7)
        }
        Some("brep_profile_offset") => {
            let loops:Vec<Vec<Curve>>=field(&v,"loops")?;let distance:f64=field(&v,"distance")?;
            if loops.iter().flatten().any(|c|brep_core::bezier_profile::requires_general(c,tolerance)){
                let e=v.get("maximumSourceDeviationMm").map(|_|field::<f64>(&v,"maximumSourceDeviationMm")).transpose()?.unwrap_or(0.02);
                let(c,r)=brep_core::profile_certificate::certify_minkowski_disk_offset(&loops,distance,e)?;
                let output=r.components(e)?.into_iter().flatten().collect();let p=profile(output,tolerance)?;
                Ok(json!({"kind":"brep-profile","loops":p["loops"],"areaMm2":p["areaMm2"],"toleranceMm":tolerance,"geometryStatus":"bounded_approximation","maximumSourceDeviationMm":e,"minkowskiCertificate":c,"innerExactRings":r.inner_exact(),"outerExactRings":r.outer_exact(),"regionStatus":"inclusion_verified"}))
            }else{profile(planar_trim::offset(&loops,distance,tolerance)?,tolerance)}
        },
        Some("brep_profile_transform") => profile(
            brep_core::transform::profile(
                &field::<Vec<Vec<Curve>>>(&v, "loops")?,
                field(&v, "matrix")?,
                tolerance,
            )?,
            tolerance,
        ),
        Some("brep_profile_author") => {
            let loops = match field::<String>(&v, "kind")?.as_str() {
                "bezier" => {
                    let path=crate::path2d::authored_path(&v["path"])?;
                    let wire=brep_core::sketch::bezier_wire(path)?;
                    brep_core::bezier_profile::orient_even_odd(&[wire.clone()],tolerance)
                        .or_else(|_|brep_core::bezier_profile::normalize(&[wire],tolerance))?
                }
                "circle" => vec![brep_core::sketch::circle_wire(field(&v, "radius")?)?],
                "rectangle" => {
                    let [w, h]: [f64; 2] = field(&v, "size")?;
                    if !w.is_finite() || !h.is_finite() || w <= 0. || h <= 0. {
                        return Err(input("Invalid profile rectangle size"));
                    }
                    let [x, y] = if field::<bool>(&v, "center")? {
                        [-w / 2., -h / 2.]
                    } else {
                        [0., 0.]
                    };
                    vec![brep_core::sketch::polygon_wire(vec![
                        [x, y],
                        [x + w, y],
                        [x + w, y + h],
                        [x, y + h],
                    ])?]
                }
                "polygon" => {
                    let rings: Vec<Vec<[f64; 2]>> = field(&v, "rings")?;
                    let loops = rings
                        .into_iter()
                        .map(brep_core::sketch::polygon_wire)
                        .collect::<Result<Vec<_>>>()?;
                    planar_trim::orient_even_odd(&loops, tolerance)?
                }
                _ => return Err(input("Unsupported authored profile kind")),
            };
            profile(loops, tolerance)
        }
        Some("brep_profile_validate") => {
            let loops: Vec<Vec<Curve>> = field(&v, "loops")?;
            let fill_rule = match v.get("fillRule") {
                Some(_) => field::<String>(&v, "fillRule")?,
                None => "material-left".into(),
            };
            match fill_rule.as_str() {
                "material-left" => profile(loops, tolerance),
                "even-odd" => profile(
                    if loops.iter().flatten().any(|c| brep_core::bezier_profile::requires_general(c,tolerance)) {
                        orient_general(&loops,tolerance)?
                    } else {
                        planar_trim::orient_even_odd(&loops, tolerance)?
                    },
                    tolerance,
                ),
                _ => Err(input("Profile fillRule must be material-left or even-odd")),
            }
        }
        Some("brep_profile_boolean") => {
            let a: Vec<Vec<Curve>> = field(&v, "a")?;
            let b: Vec<Vec<Curve>> = field(&v, "b")?;
            let operation: String = field(&v, "operation")?;
            let result = if a.iter().chain(&b).flatten().any(|c| brep_core::bezier_profile::requires_general(c,tolerance)) {
                brep_core::bezier_profile::boolean(&active_spans(&a,1022)?, &active_spans(&b,1022)?, &operation, tolerance)?
            } else {
                planar_trim::boolean(&a, &b, &operation, tolerance)?
            };
            profile(result, tolerance)
        }
        Some("brep_profile_signed_area") => {
            let wire: Vec<Curve> = field(&v, "loop")?;
            encode(if wire.iter().any(|c| brep_core::bezier_profile::requires_general(c,tolerance)) {
                brep_core::bezier_profile::signed_area(&active_spans(&[wire.clone()],1022)?[0], tolerance)?
            } else {
                planar_trim::signed_area(&wire, tolerance)?
            })
        }
        Some("brep_profile_display") => {
            let loops: Vec<Vec<Curve>> = field(&v, "loops")?;
            let mut out = Vec::with_capacity(loops.len());
            for wire in loops {
                let mut ring = Vec::new();
                for curve in wire {
                    let count = if curve.degree == 1 { 1 } else { 24 };
                    ring.extend(curve.sample_uniform(count, false)?);
                }
                out.push(ring);
            }
            encode(out)
        }
        _ => Err(input("Unknown retained profile request")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rectangle(a: [f64; 2], b: [f64; 2]) -> Vec<Curve> {
        let points = [a, [b[0], a[1]], b, [a[0], b[1]]];
        (0..4)
            .map(|i| {
                Curve::from_polyline(vec![points[i].to_vec(), points[(i + 1) % 4].to_vec()])
                    .unwrap()
            })
            .collect()
    }
    fn reverse(wire: Vec<Curve>) -> Vec<Curve> {
        wire.into_iter()
            .rev()
            .map(|c| c.reverse().unwrap())
            .collect()
    }
    #[test]
    fn retained_intersection_query_reports_original_segments_and_work_bounds() {
        let loops = vec![
            vec![Curve::from_polyline(vec![vec![0., 0.], vec![2., 2.]]).unwrap()],
            vec![Curve::from_polyline(vec![vec![0., 2.], vec![2., 0.]]).unwrap()],
        ];
        let r = dispatch(
            json!({"op":"brep_profile_intersections","loops":loops,"maxPairs":1,"maxBoxes":8192}),
        )
        .unwrap();
        assert_eq!(r["scope"].as_str(), Some("distinct-profile-segment-pairs"));
        assert_eq!(r["complete"].as_bool(), Some(true), "{r:?}");
        assert!(r["boxesVisited"].as_u64().unwrap() <= 8192);
        assert_eq!(
            r["pairs"][0]["report"]["components"][0]["kind"].as_str(),
            Some("point")
        );
    }
    #[test]
    fn general_profile_validation_roundtrip_transform_and_area_enclosure() {
        let mut wire = rectangle([0., 0.], [2., 2.]);
        wire[0] = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., -1.], vec![2., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let loops = vec![wire];
        let original = value_codec::to_string(&loops).unwrap();
        let profile = profile(loops.clone(), 1e-7).unwrap();
        let bounds: [f64; 2] = field(&profile, "areaIntervalMm2").unwrap();
        assert!(bounds[0] <= 14. / 3. && bounds[1] >= 14. / 3.);
        assert_eq!(
            field::<Vec<Vec<Curve>>>(&profile, "loops").unwrap()[0][0].control_points,
            loops[0][0].control_points
        );
        let reloaded: Value =
            value_codec::from_str(&value_codec::to_string(&profile).unwrap()).unwrap();
        let checked =
            dispatch(json!({"op":"brep_profile_validate","loops":reloaded["loops"]})).unwrap();
        assert_eq!(profile, checked);
        let matrix = [
            -2., 0., 0., 0., 0., 3., 0., 0., 0., 0., 1., 0., 10., 20., 0., 1.,
        ];
        let moved =
            dispatch(json!({"op":"brep_profile_transform","loops":loops,"matrix":matrix})).unwrap();
        let area: [f64; 2] = field(&moved, "areaIntervalMm2").unwrap();
        assert!(area[0] <= 28. && area[1] >= 28.);
        assert_eq!(value_codec::to_string(&loops).unwrap(), original);
    }
    #[test]
    fn even_odd_normalizes_unordered_original_rings_and_retains_holes() {
        let loops = vec![
            rectangle([2., 2.], [8., 8.]),
            reverse(rectangle([0., 0.], [10., 10.])),
            reverse(rectangle([4., 4.], [6., 6.])),
        ];
        let report =
            dispatch(json!({"op":"brep_profile_validate","loops":loops,"fillRule":"even-odd"}))
                .unwrap();
        assert_eq!(report["areaMm2"].as_f64(), Some(68.));
        assert_eq!(
            report["geometryStatus"].as_str(),
            Some("numerical_uncertified")
        );
        let normalized: Vec<Vec<Curve>> = field(&report, "loops").unwrap();
        assert_eq!(
            normalized
                .iter()
                .map(|w| planar_trim::signed_area(w, 1e-7).unwrap())
                .collect::<Vec<_>>(),
            vec![-36., 100., 4.]
        );
        assert!(dispatch(json!({"op":"brep_profile_validate","loops":loops})).is_err());
    }
    #[test]
    fn empty_and_boolean_profiles_have_no_solid_or_mesh_placeholder() {
        let a = vec![rectangle([0., 0.], [2., 2.])];
        let report =
            dispatch(json!({"op":"brep_profile_boolean","a":a,"b":a,"operation":"difference"}))
                .unwrap();
        assert_eq!(report["areaMm2"].as_f64(), Some(0.));
        assert!(report["loops"].as_array().unwrap().is_empty());
        assert!(report.get("mesh").is_none());
        assert!(report.get("vertices").is_none());
        let empty =
            dispatch(json!({"op":"brep_profile_validate","loops":[],"fillRule":"even-odd"}))
                .unwrap();
        assert_eq!(report, empty);
        assert!(dispatch(json!({"op":"brep_profile_signed_area","loop":[]})).is_err());
    }
    #[test]
    fn even_odd_refuses_crossings_and_contacting_holes() {
        let points = [[0., 2.], [1., 1.], [2., 2.], [1., 3.]];
        let touching_hole = (0..4)
            .map(|i| {
                Curve::from_polyline(vec![points[i].to_vec(), points[(i + 1) % 4].to_vec()])
                    .unwrap()
            })
            .collect::<Vec<_>>();
        for loops in [
            vec![rectangle([0., 0.], [2., 2.]), rectangle([1., 1.], [3., 3.])],
            vec![rectangle([0., 0.], [4., 4.]), rectangle([0., 1.], [2., 3.])],
            vec![rectangle([0., 0.], [4., 4.]), touching_hole],
        ] {
            assert!(
                dispatch(json!({"op":"brep_profile_validate","loops":loops,"fillRule":"even-odd"}))
                    .is_err()
            );
        }
    }
    #[test]
    fn offsets_retained_regions_and_reports_empty_material() {
        let loops = vec![rectangle([0., 0.], [6., 4.])];
        let expanded =
            dispatch(json!({"op":"brep_profile_offset","loops":loops,"distance":1.})).unwrap();
        assert!(
            (expanded["areaMm2"].as_f64().unwrap() - (44. + std::f64::consts::PI)).abs() < 1e-7
        );
        let empty =
            dispatch(json!({"op":"brep_profile_offset","loops":loops,"distance":-5.})).unwrap();
        assert_eq!(empty["loops"], json!([]));
        assert_eq!(empty["geometryStatus"], json!("numerical_uncertified"));
    }
    #[test]
    fn profile_measure_transport_checks_exact_area_and_rejects_tampering() {
        let loops = vec![vec![Curve::from_polyline(vec![vec![0.,0.],vec![8.,0.],vec![8.,8.],vec![0.,8.],vec![0.,0.]]).unwrap()]];
        let c=dispatch(json!({"op":"brep_profile_certify_measure","loops":loops,"maximumAreaWidthMm2":1e-9})).unwrap();
        let verified=dispatch(json!({"op":"brep_profile_verify_measure_certificate","loops":loops,"certificate":c})).unwrap();
        assert_eq!(verified["scope"].as_str(),Some("represented-profile-topology-and-area"));
        assert_eq!(verified["spans"].as_u64(),Some(4));
        assert_eq!(verified["areaMm2"]["lower"],json!(["64","1"]));
        let mut bad=c.clone();bad["areaMm2"]["upper"]=json!(["63","1"]);
        assert!(dispatch(json!({"op":"brep_profile_verify_measure_certificate","loops":loops,"certificate":bad})).is_err());
        let mut bad=c.clone();bad["solidValid"]=json!(true);
        assert!(dispatch(json!({"op":"brep_profile_verify_measure_certificate","loops":loops,"certificate":bad})).is_err());
    }
    #[test]
    fn profile_certificate_native_transport_rechecks_proof_and_scope() {
        let loops = vec![rectangle([0., 0.], [10., 10.])];
        let c = dispatch(json!({"op":"brep_profile_certify","loops":loops})).unwrap();
        let verified =
            dispatch(json!({"op":"brep_profile_verify_certificate","loops":loops,"certificate":c}))
                .unwrap();
        assert_eq!(verified["status"].as_str(), Some("verified"));
        assert_eq!(
            verified["scope"].as_str(),
            Some("represented-profile-topology")
        );
        let mut bad = c.clone();
        bad["nestingDepth"] = json!([1]);
        assert!(
            dispatch(
                json!({"op":"brep_profile_verify_certificate","loops":loops,"certificate":bad})
            )
            .is_err()
        );
        let mut bad = c.clone();
        bad["solidValid"] = json!(true);
        assert!(
            dispatch(
                json!({"op":"brep_profile_verify_certificate","loops":loops,"certificate":bad})
            )
            .is_err()
        );
    }
}
