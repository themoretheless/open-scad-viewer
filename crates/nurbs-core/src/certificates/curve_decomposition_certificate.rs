//! Parameterwise error between one original NURBS span and a retained Bezier.
//! Original blossom controls are enclosed, never replaced by rounded extraction.
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I};
#[derive(Clone, Debug)]
pub struct Report {
    pub error_upper: Option<f64>,
    pub products: usize,
    pub reason: Option<&'static str>,
}
/// Complete ordered active-span coverage, with one shared product allowance.
/// A partial partition never exposes an aggregate error bound. This is a
/// source/retained curve correspondence premise, not filled-region ownership.
pub fn inspect_partition(original: &Curve, retained: &[Curve], max_products: usize) -> Result<Report> {
    original.validate()?;
    check(max_products <= 1000000, "Decomposition product budget exceeds1000000")?;
    let spans = (original.degree..original.control_points.len())
        .filter(|&i| original.knots[i] < original.knots[i + 1])
        .collect::<Vec<_>>();
    let mut out = Report { error_upper: None, products: 0, reason: Some("incomplete-span-partition") };
    if spans.is_empty() || retained.len() != spans.len() { return Ok(out); }
    let mut upper = 0f64;
    for (span, curve) in spans.into_iter().zip(retained) {
        let report = inspect(original, span, curve, max_products - out.products)?;
        out.products += report.products;
        let Some(error) = report.error_upper else { out.reason = report.reason; return Ok(out); };
        upper = upper.max(error);
    }
    out.error_upper = Some(upper);
    out.reason = None;
    Ok(out)
}
fn choose(n: usize, k: usize) -> f64 {
    // n <= 50: binomial coefficients are below 2^53 and integer
    // intermediates fit u64, so conversion introduces no rounding.
    let mut value = 1u64;
    for i in 0..k.min(n - k) {
        value = value * (n - i) as u64 / (i + 1) as u64;
    }
    value as f64
}
/// Normalized traversal of retained's active domain corresponds to the whole
/// original knot span. This proves a curve bound, not wall/cap ownership.
pub fn inspect(
    original: &Curve,
    span: usize,
    retained: &Curve,
    max_products: usize,
) -> Result<Report> {
    original.validate()?;
    retained.validate()?;
    check(
        max_products <= 1000000,
        "Decomposition product budget exceeds1000000",
    )?;
    let mut out = Report {
        error_upper: None,
        products: 0,
        reason: Some("unsupported-layout"),
    };
    let p = original.degree;
    // Periodicity changes continuity metadata, not the stored active-span
    // rational basis. This bound covers one span; seam closure is independent.
    if retained.periodic
        || p > 25
        || span < p
        || span >= original.control_points.len()
        || original.knots[span] >= original.knots[span + 1]
        || retained.degree != p
        || retained.control_points.len() != p + 1
        || original
            .control_points
            .iter()
            .chain(&retained.control_points)
            .any(|v| v.len() != 3)
        || !retained.knots[..=p].iter().all(|x| *x == retained.knots[p])
        || !retained.knots[p + 1..]
            .iter()
            .all(|x| *x == retained.knots[p + 1])
    {
        return Ok(out);
    }
    let required = (p + 1) * (p + 1);
    if required > max_products {
        out.reason = Some("work-limit");
        return Ok(out);
    }
    out.products = required;
    let computed = (|| -> Result<f64> {
        let a = crate::curve_distance::restricted_controls(
            original,
            span,
            I::new(original.knots[span], original.knots[span + 1])?,
        )?;
        let origin = &original.control_points[span - p];
        let scale = (span - p..=span)
            .map(|i| original.weights[i])
            .fold(0., f64::max);
        let b = retained
            .control_points
            .iter()
            .zip(&retained.weights)
            .map(|(point, weight)| {
                let w = I::point(*weight).div(I::point(scale))?;
                let mut h = point
                    .iter()
                    .zip(origin)
                    .map(|(x, o)| I::point(*x).sub(I::point(*o))?.mul(w))
                    .collect::<Result<Vec<_>>>()?;
                h.push(w);
                Ok(h)
            })
            .collect::<Result<Vec<_>>>()?;
        let min_a = a.iter().map(|h| h[3].lo).fold(f64::INFINITY, f64::min);
        let min_b = b.iter().map(|h| h[3].lo).fold(f64::INFINITY, f64::min);
        let denominator = I::point(min_a).mul(I::point(min_b))?;
        let mut numerator = vec![[I::point(0.); 3]; 2 * p + 1];
        for i in 0..=p {
            for j in 0..=p {
                let factor = I::point(choose(p, i))
                    .mul(I::point(choose(p, j)))?
                    .div(I::point(choose(2 * p, i + j)))?;
                for k in 0..3 {
                    let difference = a[i][k].mul(b[j][3])?.sub(b[j][k].mul(a[i][3])?)?;
                    numerator[i + j][k] = numerator[i + j][k].add(difference.mul(factor)?)?;
                }
            }
        }
        let bounds = (0..3)
            .map(|k| {
                I::new(
                    numerator
                        .iter()
                        .map(|h| h[k].lo)
                        .fold(f64::INFINITY, f64::min),
                    numerator
                        .iter()
                        .map(|h| h[k].hi)
                        .fold(f64::NEG_INFINITY, f64::max),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let (_, norm) = crate::distance_bounds::box_distance(&bounds, &[I::point(0.); 3])?;
        Ok(I::point(norm).div(denominator)?.hi)
    })();
    match computed {
        Ok(bound) => {
            out.error_upper = Some(bound);
            out.reason = None;
        }
        Err(_) => out.reason = Some("numeric-range"),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unclamped_endpoint_spans_keep_parameterwise_error_and_work_limits() {
        let original = Curve {
            degree: 2,
            knots: vec![-2., -1., 0., 1., 2., 3., 4., 5.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 2., 0.],
                vec![2., -1., 0.],
                vec![3., 1., 0.],
                vec![4., 0., 0.],
            ],
            weights: vec![1., 0.75, 1.25, 2., 1.],
            periodic: false,
        };
        let before = original.clone();
        let parts = original.decompose().unwrap();
        assert_eq!(parts.len(), 3);
        let retained=parts.iter().map(|p|p.definition().clone()).collect::<Vec<_>>();
        let complete=inspect_partition(&original,&retained,27).unwrap();
        assert!(complete.error_upper.unwrap()<1e-10);
        assert_eq!(complete.products,27);
        let partial=inspect_partition(&original,&retained,26).unwrap();
        assert!(partial.error_upper.is_none());
        assert_eq!(partial.products,18);
        assert_eq!(partial.reason,Some("work-limit"));
        assert!(inspect_partition(&original,&retained[..2],27).unwrap().error_upper.is_none());
        let mut damaged=retained.clone();damaged[1].control_points[1][2]=0.25;
        assert!(inspect_partition(&original,&damaged,27).unwrap().error_upper.unwrap()>=0.01);
        let mut permuted=retained.clone();permuted.swap(0,2);
        let permuted_bound=inspect_partition(&original,&permuted,27).unwrap().error_upper.unwrap();
        assert!(permuted_bound>0.1);
        // Equal partition cardinality cannot masquerade as correct station
        // correspondence: compare all spans against independently evaluated
        // original parameters, including the exchanged endpoint spans.
        for (index,part) in permuted.iter().enumerate() {
            let span=original.degree+index;let domain=part.domain();
            for step in 0..=10 {
                let t=step as f64/10.;
                let source=original.evaluate(original.knots[span]+t*(original.knots[span+1]-original.knots[span])).unwrap().point;
                let target=part.evaluate(domain[0]+t*(domain[1]-domain[0])).unwrap().point;
                let error=source.iter().zip(target).map(|(a,b)|(a-b).powi(2)).sum::<f64>().sqrt();
                assert!(error<=permuted_bound);
            }
        }
        for (span, part) in (2..5).zip(parts) {
            assert_eq!(part.domain(), [original.knots[span], original.knots[span + 1]]);
            let report = inspect(&original, span, part.definition(), 9).unwrap();
            assert!(report.error_upper.unwrap() < 1e-10, "{report:?}");
            assert_eq!(report.products, 9);
            assert!(inspect(&original, span, part.definition(), 8).unwrap().error_upper.is_none());
            let mut displaced = part.definition().clone();
            // Interior-pole displacement defeats endpoint-only comparison.
            displaced.control_points[1][2] = 0.25;
            let report = inspect(&original, span, &displaced, 9).unwrap();
            let middle = (original.knots[span] + original.knots[span + 1]) / 2.;
            let actual = displaced.evaluate(middle).unwrap().point[2].abs();
            assert!(actual > 0.);
            assert!(report.error_upper.unwrap() >= actual);
        }
        assert_eq!(original, before);
    }
    #[test]
    fn degree_25_whole_domain_and_product_budget() {
        let c = Curve {
            degree: 25,
            knots: [vec![0.; 26], vec![1.; 26]].concat(),
            control_points: (0..26).map(|i| vec![i as f64 / 25., 0., 0.]).collect(),
            weights: vec![1.; 26],
            periodic: false,
        };
        let r = inspect(&c, 25, &c, 676).unwrap();
        assert!(r.error_upper.unwrap() < 1e-9);
        assert!(inspect(&c, 25, &c, 675).unwrap().error_upper.is_none());
        let mut translated = c.clone();
        for point in &mut translated.control_points {
            point[2] = 0.125;
        }
        assert!(
            inspect(&c, 25, &translated, 676)
                .unwrap()
                .error_upper
                .unwrap()
                >= 0.125
        );
    }
    #[test]
    fn nonuniform_rational_extraction_and_independent_translation() {
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.4, 1., 1., 1.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![0.3, 0.8, 0.],
                vec![0.7, -0.2, 0.],
                vec![1., 0., 0.],
            ],
            weights: vec![1., 0.75, 1.25, 2.],
            periodic: false,
        };
        let before = c.clone();
        let parts = c.decompose().unwrap();
        for (span, part) in [2, 3].into_iter().zip(parts) {
            let r = inspect(&c, span, part.definition(), 1000).unwrap();
            assert!(r.error_upper.unwrap() < 1e-10);
            #[cfg(feature = "transport")]
            {
                let request = value_codec::json!({"op":"curve_decomposition_audit","curve":c,"span":span,"retained":part.definition(),"maxProducts":1000});
                let report = crate::transport::dispatch(request.clone()).unwrap();
                assert!(report["errorUpper"].as_f64().unwrap() < 1e-10);
                assert_eq!(report["continuousBound"], false);
                let mut cut = request.clone();
                cut["maxProducts"] = value_codec::json!(r.products - 1);
                assert!(crate::transport::dispatch(cut).unwrap()["errorUpper"].is_null());
                let mut invalid = request;
                invalid["maxProducts"] = value_codec::json!(-1);
                assert!(crate::transport::dispatch(invalid).is_err());
            }
            assert!(
                inspect(&c, span, part.definition(), r.products - 1)
                    .unwrap()
                    .error_upper
                    .is_none()
            );
            let mut shifted = part.definition().clone();
            for pole in &mut shifted.control_points {
                pole[2] += 0.125;
            }
            assert!(
                inspect(&c, span, &shifted, 1000)
                    .unwrap()
                    .error_upper
                    .unwrap()
                    >= 0.125
            );
            assert!(
                inspect(&c, 0, part.definition(), 1000)
                    .unwrap()
                    .error_upper
                    .is_none()
            );
        }
        assert_eq!(c, before);
    }
}

pub struct BatchReport {
    pub error_upper:Option<f64>,pub products:usize,pub pairs_inspected:usize,pub reason:Option<&'static str>,
}
/// Complete explicitly declared pair set under one shared product budget.
/// No omitted span, filled-region or original sweep transport claim is inferred.
pub fn inspect_batch(pairs:&[(&Curve,usize,&Curve)],max_products:usize)->Result<BatchReport> {
    let mut out=BatchReport {error_upper:None,products:0,pairs_inspected:0,reason:Some("work-limit")};
    if max_products>1000000 || pairs.is_empty() || pairs.len()>2048 {return Ok(out);}
    let mut maximum=0f64;
    for &(original,span,retained) in pairs {
        let r=inspect(original,span,retained,max_products-out.products)?;
        check(r.products<=max_products-out.products,"Invalid native decomposition product accounting")?;
        out.products+=r.products;out.pairs_inspected+=1;
        let Some(upper)=r.error_upper else {out.reason=r.reason.or(Some("span-unproved"));return Ok(out);};
        maximum=maximum.max(upper);
    }
    out.error_upper=Some(maximum);out.reason=None;Ok(out)
}
#[cfg(test)]
mod batch_tests {
    use super::*;
    #[test]
    fn batch_has_one_budget_and_never_returns_a_partial_maximum() {
        let c=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.,0.,0.],vec![1.,1.,0.],vec![2.,0.,0.]],weights:vec![1.;3],periodic:false};
        #[cfg(feature="transport")]
        {
            let request=value_codec::json!({"op":"curve_decomposition_batch_audit","pairs":[
                {"curve":c,"span":2,"retained":c},{"curve":c,"span":2,"retained":c}],"maxProducts":1000});
            let r=crate::transport::dispatch(request.clone()).unwrap();
            assert!(r["errorUpper"].as_f64().is_some());
            assert_eq!(r["pairsInspected"],2);
            let mut zero=request; zero["maxProducts"]=value_codec::json!(-1);
            let r=crate::transport::dispatch(zero).unwrap();
            assert!(r["errorUpper"].is_null()); assert_eq!(r["reason"],"work-limit");
        }
        let pairs=[(&c,2,&c),(&c,2,&c)];
        let r=inspect_batch(&pairs,1000).unwrap();
        assert!(r.error_upper.is_some() && r.pairs_inspected==2);
        let short=inspect_batch(&pairs,r.products-1).unwrap();
        assert!(short.error_upper.is_none() && short.pairs_inspected==2 && short.products<r.products);
        assert!(inspect_batch(&[],1000).unwrap().error_upper.is_none());
        assert!(inspect_batch(&[(&c,2,&c),(&c,0,&c)],1000).unwrap().error_upper.is_none());
    }
}
