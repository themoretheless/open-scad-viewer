//! Quantified image-set equality of piecewise rational curves. No monotone
//! source coordinate or nonzero jet is assumed. Both inclusions are checked.
use super::*;
use algebraic_intersections::{ExactPolynomial, ExactRatio, decode_ratio, polynomial, ratio};
use arithmetic::{algebraic_events as roots, bivariate as bi};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageRelationWitness {
    pub target_cell: usize,
    pub relation: Vec<Vec<ExactRatio>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageCoverageWitness {
    pub relations: Vec<ImageRelationWitness>,
    pub projection: ExactPolynomial,
    pub roots: Vec<[ExactRatio; 2]>,
    pub sections: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurveImageCertificate {
    pub version: u32,
    pub source: String,
    pub forward: Vec<ImageCoverageWitness>,
    pub backward: Vec<ImageCoverageWitness>,
}
#[derive(Clone, Debug)]
pub struct VerifiedCurveImage {
    pub source_cells: [usize; 2],
    pub coverage_sections: usize,
}
type Net = Vec<[Q; 3]>;
fn source(a: &Curve, b: &Curve, da: [f64; 2], db: [f64; 2]) -> Result<String> {
    Ok(format!(
        "curve-image-coverage-v1/{}",
        agreement::source(a, b, da, db)?
    ))
}
fn nets(c: &Curve, d: [f64; 2]) -> Result<Vec<Net>> {
    let n = multispan::cells(c, d, 0)?;
    if n.len() > 64 {
        return Err(refused("Quantified image source exceeds 64 active cells"));
    }
    Ok(n)
}
fn decode_projection(p: &ExactPolynomial) -> Result<Vec<Q>> {
    if p.len() > 1025 {
        return Err(refused("Image projection degree exceeded"));
    }
    p.iter().map(decode_ratio).collect()
}
fn samples(intervals: &[[Q; 2]]) -> Vec<Q> {
    let interior: Vec<_> = intervals
        .iter()
        .filter(|p| p[0] != p[1] || (p[0] != Q::zero() && p[0] != Q::one()))
        .collect();
    let mut out = Vec::new();
    let mut last = Q::zero();
    for r in interior {
        out.push((&last + &r[0]) / Q::from_integer(2.into()));
        last = r[1].clone();
    }
    out.push((last + Q::one()) / Q::from_integer(2.into()));
    out
}
fn produce_coverage(a: &Net, targets: &[Net]) -> Result<ImageCoverageWitness> {
    let mut relations = Vec::new();
    let mut critical = Vec::new();
    let mut exact = Vec::new();
    for (j, b) in targets.iter().enumerate() {
        if let Ok(h) = bi::image_relation(a, b) {
            critical.push(bi::image_relation_projection(a, b, &h)?);
            relations.push(ImageRelationWitness {
                target_cell: j,
                relation: h.iter().map(|p| p.iter().map(ratio).collect()).collect(),
            });
            exact.push(h);
        }
    }
    if relations.is_empty() {
        return Err(refused("No common component covers this source image cell"));
    }
    let projection = bi::combine_image_projections(&critical)?;
    let intervals = roots::isolates(
        &projection,
        Q::zero(),
        Q::one(),
        &Q::new(1.into(), 1_000_000.into()),
    )?;
    let mut sections = Vec::new();
    for s in samples(&intervals) {
        let mut found = None;
        for (i, h) in exact.iter().enumerate() {
            if bi::image_fiber_exists(h, &s)? {
                found = Some(i);
                break;
            }
        }
        sections.push(found.ok_or_else(|| {
            refused("Curve image has a proven uncovered real parameter interval")
        })?);
    }
    Ok(ImageCoverageWitness {
        relations,
        projection: polynomial(&projection),
        roots: intervals
            .into_iter()
            .map(|r| r.map(|q| ratio(&q)))
            .collect(),
        sections,
    })
}
fn verify_coverage(a: &Net, targets: &[Net], c: &ImageCoverageWitness) -> Result<usize> {
    if c.relations.is_empty()
        || c.relations.len() > 64
        || c.roots.len() > 256
        || c.sections.len() > 257
    {
        return Err(refused("Image coverage witness size mismatch"));
    }
    let mut exact = Vec::new();
    let mut critical = Vec::new();
    let mut previous = None;
    for rel in &c.relations {
        if rel.target_cell >= targets.len() || previous.is_some_and(|p| p >= rel.target_cell) {
            return Err(refused("Image target cells are false or reordered"));
        }
        previous = Some(rel.target_cell);
        let h = nonlinear::decode(&rel.relation)?;
        critical.push(bi::image_relation_projection(
            a,
            &targets[rel.target_cell],
            &h,
        )?);
        exact.push(h);
    }
    let p = bi::combine_image_projections(&critical)?;
    if decode_projection(&c.projection)? != p {
        return Err(refused("Image critical parameter polynomial is false"));
    }
    let intervals = c
        .roots
        .iter()
        .map(|r| Ok([decode_ratio(&r[0])?, decode_ratio(&r[1])?]))
        .collect::<Result<Vec<_>>>()?;
    let mut previous = None;
    // Complete Sturm census. End roots are separate point intervals; interval
    // endpoints are nonroots and may be shared only at a noncritical rational.
    let eval = |x: &Q| p.iter().rev().fold(Q::zero(), |v, c| v * x + c);
    for r in &intervals {
        let [a, b] = r;
        if a < &Q::zero()
            || b > &Q::one()
            || a > b
            || previous
                .as_ref()
                .is_some_and(|q: &Q| q > a || (q == a && eval(a).is_zero()))
            || roots::count(&p, a, b)? != 1
            || (a != b && (eval(a).is_zero() || eval(b).is_zero()))
        {
            return Err(refused("Image critical root isolation is false"));
        }
        previous = Some(b.clone());
    }
    let count = roots::count(&p, &Q::zero(), &Q::one())?
        + usize::from(eval(&Q::zero()).is_zero())
        + usize::from(eval(&Q::one()).is_zero());
    if count != intervals.len() {
        return Err(refused("Image critical root census is incomplete"));
    }
    let s = samples(&intervals);
    if s.len() != c.sections.len() {
        return Err(refused("Image parameter section census is incomplete"));
    }
    for (x, index) in s.iter().zip(&c.sections) {
        let h = exact
            .get(*index)
            .ok_or_else(|| refused("Image section names a missing relation"))?;
        if eval(x).is_zero() || !bi::image_fiber_exists(h, x)? {
            return Err(refused(
                "Image section has no checked target parameter in [0,1]",
            ));
        }
    }
    // Root existence is constant on each real projection cell. Coverage of the
    // dense open cells extends to all critical parameters by compactness of the
    // finite union of closed target images and continuity of the source cell.
    Ok(s.len())
}
pub fn certify_curve_image_agreement(
    a: &Curve,
    b: &Curve,
    da: [f64; 2],
    db: [f64; 2],
) -> Result<CurveImageCertificate> {
    let source = source(a, b, da, db)?;
    let x = nets(a, da)?;
    let y = nets(b, db)?;
    let c = CurveImageCertificate {
        version: 1,
        source,
        forward: x
            .iter()
            .map(|n| produce_coverage(n, &y))
            .collect::<Result<_>>()?,
        backward: y
            .iter()
            .map(|n| produce_coverage(n, &x))
            .collect::<Result<_>>()?,
    };
    verify_curve_image_agreement(a, b, da, db, &c)?;
    Ok(c)
}
pub fn verify_curve_image_agreement(
    a: &Curve,
    b: &Curve,
    da: [f64; 2],
    db: [f64; 2],
    c: &CurveImageCertificate,
) -> Result<VerifiedCurveImage> {
    if c.version != 1 || c.source != source(a, b, da, db)? {
        return Err(refused("Curve image source identity mismatch"));
    }
    let x = nets(a, da)?;
    let y = nets(b, db)?;
    if c.forward.len() != x.len() || c.backward.len() != y.len() {
        return Err(refused("Curve image source-cell coverage is incomplete"));
    }
    let mut count = 0;
    for (n, p) in x.iter().zip(&c.forward) {
        count += verify_coverage(n, &y, p)?;
    }
    for (n, p) in y.iter().zip(&c.backward) {
        count += verify_coverage(n, &x, p)?;
    }
    Ok(VerifiedCurveImage {
        source_cells: [x.len(), y.len()],
        coverage_sections: count,
    })
}
#[cfg(feature = "codec")]
mod serialization {
    use super::*;
    use value_codec::{Deserialize, Serialize, Value, error, json};
    fn schema(v: &Value, keys: &[&str]) -> value_codec::Result<()> {
        if v.as_object()
            .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
        {
            return Err(error("Curve image witness schema mismatch"));
        }
        Ok(())
    }
    impl Serialize for ImageRelationWitness {
        fn to_value(&self) -> Value {
            json!({"targetCell":self.target_cell,"relation":self.relation})
        }
    }
    impl<'de> Deserialize<'de> for ImageRelationWitness {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            schema(&v, &["targetCell", "relation"])?;
            if v["relation"].as_array().is_none_or(|a| {
                a.len() > 18 || a.iter().any(|p| p.as_array().is_none_or(|p| p.len() > 257))
            }) {
                return Err(error("Image relation size exceeded"));
            }
            Ok(Self {
                target_cell: Deserialize::from_value(v["targetCell"].clone())?,
                relation: Deserialize::from_value(v["relation"].clone())?,
            })
        }
    }
    impl Serialize for ImageCoverageWitness {
        fn to_value(&self) -> Value {
            json!({"relations":self.relations,"projection":self.projection,"roots":self.roots,"sections":self.sections})
        }
    }
    impl<'de> Deserialize<'de> for ImageCoverageWitness {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            schema(&v, &["relations", "projection", "roots", "sections"])?;
            if v["relations"].as_array().is_none_or(|a| a.len() > 64)
                || v["projection"].as_array().is_none_or(|a| a.len() > 1025)
                || v["roots"].as_array().is_none_or(|a| a.len() > 256)
                || v["sections"].as_array().is_none_or(|a| a.len() > 257)
            {
                return Err(error("Image coverage size exceeded"));
            }
            Ok(Self {
                relations: Deserialize::from_value(v["relations"].clone())?,
                projection: Deserialize::from_value(v["projection"].clone())?,
                roots: Deserialize::from_value(v["roots"].clone())?,
                sections: Deserialize::from_value(v["sections"].clone())?,
            })
        }
    }
    impl Serialize for CurveImageCertificate {
        fn to_value(&self) -> Value {
            json!({"version":self.version,"source":self.source,"scope":"represented-curve-image-set-agreement","forward":self.forward,"backward":self.backward})
        }
    }
    impl<'de> Deserialize<'de> for CurveImageCertificate {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            schema(&v, &["version", "source", "scope", "forward", "backward"])?;
            if v["scope"].as_str() != Some("represented-curve-image-set-agreement")
                || v["source"].as_str().is_none_or(|s| s.len() > 140000)
                || v["forward"].as_array().is_none_or(|a| a.len() > 64)
                || v["backward"].as_array().is_none_or(|a| a.len() > 64)
            {
                return Err(error("Curve image certificate size or scope mismatch"));
            }
            Ok(Self {
                version: Deserialize::from_value(v["version"].clone())?,
                source: Deserialize::from_value(v["source"].clone())?,
                forward: Deserialize::from_value(v["forward"].clone())?,
                backward: Deserialize::from_value(v["backward"].clone())?,
            })
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn c(p: Vec<Vec<f64>>) -> Curve {
        let n = p.len();
        Curve {
            degree: n - 1,
            knots: [vec![0.; n], vec![1.; n]].concat(),
            weights: vec![1.; n],
            control_points: p,
            periodic: false,
        }
    }
    #[test]
    fn nonmonotone_retracing_with_nonlinear_speed_and_false_extent() {
        let a = c(vec![vec![0., 0.], vec![6., 0.], vec![0., 0.]]);
        let b = c(vec![
            vec![0., 0.],
            vec![0., 0.],
            vec![8., 0.],
            vec![0., 0.],
            vec![0., 0.],
        ]);
        let proof = certify_curve_image_agreement(&a, &b, [0., 1.], [0., 1.]).unwrap();
        verify_curve_image_agreement(&a, &b, [0., 1.], [0., 1.], &proof).unwrap();
        let mut bad = proof;
        bad.backward.clear();
        assert!(verify_curve_image_agreement(&a, &b, [0., 1.], [0., 1.], &bad).is_err());
        let mut larger = b;
        larger.control_points[2][0] = 9.;
        assert!(certify_curve_image_agreement(&a, &larger, [0., 1.], [0., 1.]).is_err());
    }
    #[test]
    fn nonlinear_irrational_cusp_preimage_without_any_regular_jet_assumption() {
        let a = c(vec![
            vec![60., -60.],
            vec![-20., 60.],
            vec![-20., -60.],
            vec![60., 60.],
        ]);
        let choose = |n: usize, k: usize| {
            if n < k {
                0
            } else {
                (0..k).fold(1, |v, i| v * (n - i) / (i + 1))
            }
        };
        let b = c((0..=6)
            .map(|i| {
                vec![
                    (60 - 16 * choose(i, 2) as i64 + 16 * choose(i, 4) as i64) as f64,
                    (-60 + 24 * choose(i, 2) as i64 - 48 * choose(i, 4) as i64
                        + 480 * choose(i, 6) as i64) as f64,
                ]
            })
            .collect());
        let proof = certify_curve_image_agreement(&a, &b, [0., 1.], [0., 1.]).unwrap();
        verify_curve_image_agreement(&a, &b, [0., 1.], [0., 1.], &proof).unwrap();
        let mut bad = proof;
        bad.forward[0].sections.pop();
        assert!(verify_curve_image_agreement(&a, &b, [0., 1.], [0., 1.], &bad).is_err());
    }
    #[test]
    fn union_of_target_cells_switches_at_an_irrational_parameter() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            control_points: vec![
                vec![0., 0.],
                vec![12., 0.],
                vec![24., 1.],
                vec![36., 2.],
                vec![48., 4.],
            ],
            weights: vec![1.; 5],
            periodic: false,
        };
        let b = c(vec![
            vec![0., 0.],
            vec![0., 0.],
            vec![8., 0.],
            vec![24., 0.],
            vec![48., 4.],
        ]);
        let proof = certify_curve_image_agreement(&a, &b, [0., 1.], [0., 1.]).unwrap();
        assert_eq!(proof.forward.len(), 2);
        assert!(proof.backward[0].relations.len() >= 2);
        verify_curve_image_agreement(&a, &b, [0., 1.], [0., 1.], &proof).unwrap();
        let mut bad = proof;
        let first = bad.backward[0].sections[0];
        *bad.backward[0].sections.last_mut().unwrap() = first;
        assert!(verify_curve_image_agreement(&a, &b, [0., 1.], [0., 1.], &bad).is_err());
    }
}
