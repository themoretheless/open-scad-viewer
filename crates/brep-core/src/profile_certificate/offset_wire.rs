//! Source-bound one-sided normal offsets with bevel, miter or minor-arc joins.
//! The complete joined target wire is approximated within a Hausdorff bound.
//! Global region clipping/injectivity is a separate operation and proof.
use super::*;
use num_traits::ToPrimitive;
use offset::Range;
#[derive(Clone, Debug, PartialEq)]
pub struct OffsetWirePart {
    pub parameters: Vec<f64>,
    pub points: Vec<[f64; 2]>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct OffsetWireCertificate {
    pub version: u32,
    pub source: String,
    pub closed: bool,
    pub join_style: String,
    pub parts: Vec<OffsetWirePart>,
    pub joins: Vec<Vec<[f64; 2]>>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct VerifiedOffsetWire {
    segments: usize,
    maximum_deviation_mm: f64,
}
impl VerifiedOffsetWire {
    pub fn segments(&self) -> usize {
        self.segments
    }
    pub fn maximum_deviation_mm(&self) -> f64 {
        self.maximum_deviation_mm
    }
}
impl OffsetWireCertificate {
    pub fn curves(&self) -> Result<Vec<Curve>> {
        let mut out = Vec::new();
        for (i, part) in self.parts.iter().enumerate() {
            for p in part.points.windows(2) {
                if p[0] != p[1] {
                    out.push(Curve::from_polyline(
                        p.iter().map(|p| p.to_vec()).collect(),
                    )?);
                }
            }
            if let Some(join) = self.joins.get(i) {
                for p in join.windows(2) {
                    if p[0] != p[1] {
                        out.push(Curve::from_polyline(
                            p.iter().map(|p| p.to_vec()).collect(),
                        )?);
                    }
                }
            }
        }
        // Removing zero-length joins preserves the wire image unless the
        // entire target is a single point (e.g. a zero offset of a point curve).
        // Keep that represented point rather than returning an empty image.
        if out.is_empty() {
            let p = self
                .parts
                .first()
                .and_then(|p| p.points.first())
                .ok_or_else(|| refused("Offset wire has no represented output point"))?;
            out.push(Curve::from_polyline(vec![p.to_vec(), p.to_vec()])?);
        }
        Ok(out)
    }
}
struct Cell {
    curve: usize,
    domain: [f64; 2],
    net: Vec<[Q; 3]>,
}
fn input(
    curves: &[Curve],
    distance: f64,
    deviation: f64,
    closed: bool,
    style: &str,
    limit: f64,
) -> Result<(String, Vec<Cell>)> {
    if curves.is_empty()
        || curves.len() > 128
        || !distance.is_finite()
        || distance.abs() > 1e6
        || !deviation.is_finite()
        || !(1e-9..=10.).contains(&deviation)
        || !limit.is_finite()
        || limit <= 0.
        || limit > 1e6
        || !matches!(style, "bevel" | "miter" | "round")
    {
        return Err(refused("Invalid joined normal-offset options"));
    }
    let bits = source_bits(&[curves.to_vec()], 1e-7)?;
    let mut cells = Vec::new();
    for (index, c) in curves.iter().enumerate() {
        if c.periodic && (!closed || curves.len() != 1) {
            return Err(refused(
                "Periodic offset source must be a closed single curve",
            ));
        }
        let [a, b] = c.domain();
        let mut cuts: Vec<_> = c
            .knots
            .iter()
            .copied()
            .filter(|t| *t >= a && *t <= b)
            .collect();
        cuts.dedup();
        for p in cuts.windows(2) {
            cells.push(Cell {
                curve: index,
                domain: [p[0], p[1]],
                net: extraction::interval(
                    c,
                    &Q::from_float(p[0]).unwrap(),
                    &Q::from_float(p[1]).unwrap(),
                )?,
            });
        }
    }
    if cells.is_empty() || cells.len() > 128 {
        return Err(refused("Joined offset cell budget exceeded"));
    }
    let count = if closed { cells.len() } else { cells.len() - 1 };
    for i in 0..count {
        let p = cells[i].net.last().unwrap();
        let q = &cells[(i + 1) % cells.len()].net[0];
        if &p[0] * &q[2] != &q[0] * &p[2] || &p[1] * &q[2] != &q[1] * &p[2] {
            return Err(refused("Joined offset source is not exactly continuous"));
        }
    }
    Ok((
        format!(
            "joined-normal-offset-v1/{:016x}/{:016x}/{}/{style}/{:016x}/{bits}",
            distance.to_bits(),
            deviation.to_bits(),
            u8::from(closed),
            limit.to_bits()
        ),
        cells,
    ))
}
fn point(q: Q) -> Range {
    Range {
        lo: q.clone(),
        hi: q,
    }
}
fn add(a: &Range, b: &Range) -> Range {
    Range {
        lo: &a.lo + &b.lo,
        hi: &a.hi + &b.hi,
    }
}
fn sub(a: &Range, b: &Range) -> Range {
    Range {
        lo: &a.lo - &b.hi,
        hi: &a.hi - &b.lo,
    }
}
fn center(net: &[[Q; 3]], end: bool) -> [Q; 2] {
    let p = if end { net.last().unwrap() } else { &net[0] };
    [&p[0] / &p[2], &p[1] / &p[2]]
}
fn tangent(net: &[[Q; 3]], end: bool) -> [Q; 2] {
    arithmetic::tangent_numerators(net).map(|v| {
        if end {
            v.last().unwrap().clone()
        } else {
            v[0].clone()
        }
    })
}
fn cross(a: &[Q; 2], b: &[Q; 2]) -> Q {
    &a[0] * &b[1] - &a[1] * &b[0]
}
fn dot(a: &[Q; 2], b: &[Q; 2]) -> Q {
    &a[0] * &b[0] + &a[1] * &b[1]
}
fn projected(c: &[Q; 2], ray: &[Q; 2], radius: &Q) -> Result<[Range; 2]> {
    let squared = dot(ray, ray);
    let lo = offset::sqrt_bound(&squared, false);
    let hi = offset::sqrt_bound(&squared, true);
    if lo <= Q::zero() {
        return Err(refused("Offset join has a singular normal"));
    }
    let inverse = Range {
        lo: Q::one() / hi,
        hi: Q::one() / lo,
    };
    Ok(std::array::from_fn(|k| {
        add(&point(c[k].clone()), &point(&ray[k] * radius).mul(&inverse))
    }))
}
fn rays(a: &Cell, b: &Cell, d: f64) -> ([Q; 2], [Q; 2]) {
    let sign = if d < 0. { -Q::one() } else { Q::one() };
    let normal = |t: [Q; 2]| [-&t[1] * &sign, &t[0] * &sign];
    (
        normal(tangent(&a.net, true)),
        normal(tangent(&b.net, false)),
    )
}
fn error_squared(p: [f64; 2], ideal: &[Range; 2]) -> Result<Q> {
    let mut e = Q::zero();
    for k in 0..2 {
        let q = Q::from_float(p[k]).ok_or_else(|| refused("Nonfinite offset join output"))?;
        e += sub(&point(q), &ideal[k]).max_abs().pow(2);
    }
    Ok(e)
}
fn miter_box(a: &Cell, b: &Cell, d: f64, limit: f64) -> Result<[Range; 2]> {
    let c = center(&a.net, true);
    let r = Q::from_float(d.abs()).unwrap();
    let (ra, rb) = rays(a, b, d);
    let u = tangent(&a.net, true);
    let v = tangent(&b.net, false);
    let denominator = cross(&u, &v);
    if denominator.is_zero() {
        return Err(refused("Miter has parallel tangent lines"));
    }
    let a = projected(&c, &ra, &r)?;
    let b = projected(&c, &rb, &r)?;
    let delta = [sub(&b[0], &a[0]), sub(&b[1], &a[1])];
    let t = sub(
        &delta[0].mul(&point(v[1].clone())),
        &delta[1].mul(&point(v[0].clone())),
    )
    .mul(&point(Q::one() / denominator));
    let m = std::array::from_fn(|k| add(&a[k], &t.mul(&point(u[k].clone()))));
    let max = Q::from_float(limit).unwrap().pow(2);
    for endpoint in [&a, &b] {
        if (0..2).fold(Q::zero(), |s, k| {
            s + sub(&m[k], &endpoint[k]).max_abs().pow(2)
        }) > max
        {
            return Err(refused("Miter exceeds the extension limit"));
        }
    }
    Ok(m)
}
fn round_check(a: &Cell, b: &Cell, d: f64, deviation: f64, points: &[[f64; 2]]) -> Result<bool> {
    let c = center(&a.net, true);
    let radius = Q::from_float(d.abs()).unwrap();
    let eps = Q::from_float(deviation).unwrap() / Q::from_integer(4.into());
    let (ra, rb) = rays(a, b, d);
    let turn = cross(&ra, &rb);
    let direction = if turn < Q::zero() {
        -Q::one()
    } else {
        Q::one()
    };
    if turn.is_zero() && dot(&ra, &rb) > Q::zero() {
        return Ok(points.len() == 2);
    }
    let mut vectors = Vec::new();
    vectors.push(ra.clone());
    for p in points.iter().skip(1).take(points.len() - 2) {
        let v = std::array::from_fn(|k| Q::from_float(p[k]).unwrap() - &c[k]);
        if cross(&ra, &v) * &direction < Q::zero() || cross(&v, &rb) * &direction < Q::zero() {
            return Ok(false);
        }
        vectors.push(v);
    }
    vectors.push(rb);
    let ideals = vectors
        .iter()
        .map(|v| projected(&c, v, &radius))
        .collect::<Result<Vec<_>>>()?;
    for (p, q) in points.iter().zip(&ideals) {
        if error_squared(*p, q)? > eps.pow(2) {
            return Ok(false);
        }
    }
    for (i, p) in vectors.windows(2).enumerate() {
        if cross(&p[0], &p[1]) * &direction < Q::zero() || dot(&p[0], &p[1]) < Q::zero() {
            return Ok(false);
        }
        let chord = (0..2).fold(Q::zero(), |s, k| {
            s + sub(&ideals[i][k], &ideals[i + 1][k]).max_abs().pow(2)
        });
        // For a circular minor arc <=90 degrees, sagitta <= chord²/(4r).
        if chord / (Q::from_integer(4.into()) * &radius) + &eps > Q::from_float(deviation).unwrap()
        {
            return Ok(false);
        }
    }
    Ok(true)
}
fn join(
    a: &Cell,
    b: &Cell,
    d: f64,
    deviation: f64,
    style: &str,
    limit: f64,
    left: [f64; 2],
    right: [f64; 2],
) -> Result<Vec<[f64; 2]>> {
    if style == "bevel" || d == 0. {
        return Ok(vec![left, right]);
    }
    let (ra, rb) = rays(a, b, d);
    if cross(&ra, &rb).is_zero() && dot(&ra, &rb) > Q::zero() {
        return Ok(vec![left, right]);
    }
    if style == "miter" {
        let m = miter_box(a, b, d, limit)?;
        let p = m.map(|r| {
            ((&r.lo + &r.hi) / Q::from_integer(2.into()))
                .to_f64()
                .unwrap_or(f64::NAN)
        });
        if error_squared(p, &miter_box(a, b, d, limit)?)? > Q::from_float(deviation).unwrap().pow(2)
        {
            return Err(refused("Rounded miter vertex exceeds error bound"));
        }
        return Ok(vec![left, p, right]);
    }
    let c = center(&a.net, true).map(|q| q.to_f64().unwrap_or(f64::NAN));
    let direction = if cross(&ra, &rb) < Q::zero() { -1. } else { 1. };
    let start = ra[1]
        .to_f64()
        .unwrap_or(f64::NAN)
        .atan2(ra[0].to_f64().unwrap_or(f64::NAN));
    let mut angle = rb[1]
        .to_f64()
        .unwrap_or(f64::NAN)
        .atan2(rb[0].to_f64().unwrap_or(f64::NAN))
        - start;
    if direction > 0. && angle < 0. {
        angle += std::f64::consts::TAU;
    }
    if direction < 0. && angle > 0. {
        angle -= std::f64::consts::TAU;
    }
    for n in [2, 4, 8, 16, 32, 64, 128, 256, 512, 1024] {
        let mut p = vec![left];
        for i in 1..n {
            let theta = start + angle * i as f64 / n as f64;
            p.push([c[0] + d.abs() * theta.cos(), c[1] + d.abs() * theta.sin()]);
        }
        p.push(right);
        if p.iter().flatten().all(|x| x.is_finite()) && round_check(a, b, d, deviation, &p)? {
            return Ok(p);
        }
    }
    Err(refused("Round offset join bound remains unproved"))
}
pub fn certify_offset_wire(
    curves: &[Curve],
    distance: f64,
    deviation: f64,
    closed: bool,
    style: &str,
    miter_limit: f64,
) -> Result<OffsetWireCertificate> {
    let (source, cells) = input(curves, distance, deviation, closed, style, miter_limit)?;
    let mut parts = Vec::new();
    for cell in &cells {
        let (parameters, points) = offset::produce_cell(
            &curves[cell.curve],
            cell.domain[0],
            cell.domain[1],
            distance,
            deviation * 0.25,
        )?;
        parts.push(OffsetWirePart { parameters, points });
    }
    let count = if closed { cells.len() } else { cells.len() - 1 };
    let mut joins = Vec::new();
    for i in 0..count {
        let j = (i + 1) % cells.len();
        joins.push(join(
            &cells[i],
            &cells[j],
            distance,
            deviation,
            style,
            miter_limit,
            *parts[i].points.last().unwrap(),
            parts[j].points[0],
        )?);
    }
    let c = OffsetWireCertificate {
        version: 1,
        source,
        closed,
        join_style: style.into(),
        parts,
        joins,
    };
    verify_offset_wire(curves, distance, deviation, miter_limit, &c)?;
    Ok(c)
}
pub fn verify_offset_wire(
    curves: &[Curve],
    distance: f64,
    deviation: f64,
    miter_limit: f64,
    c: &OffsetWireCertificate,
) -> Result<VerifiedOffsetWire> {
    let (source, cells) = input(
        curves,
        distance,
        deviation,
        c.closed,
        &c.join_style,
        miter_limit,
    )?;
    let joins = if c.closed {
        cells.len()
    } else {
        cells.len() - 1
    };
    let segments = c
        .parts
        .iter()
        .map(|p| p.points.len().saturating_sub(1))
        .sum::<usize>()
        + c.joins
            .iter()
            .map(|p| p.len().saturating_sub(1))
            .sum::<usize>();
    if c.version != 1
        || c.source != source
        || c.parts.len() != cells.len()
        || c.joins.len() != joins
        || segments > 2048
    {
        return Err(refused(
            "Joined offset certificate source or census mismatch",
        ));
    }
    for (cell, part) in cells.iter().zip(&c.parts) {
        if part.points.len() < 2
            || part.parameters.len() != part.points.len()
            || part.parameters[0] != cell.domain[0]
            || *part.parameters.last().unwrap() != cell.domain[1]
        {
            return Err(refused("Offset cell domain coverage mismatch"));
        }
        for (i, p) in part.parameters.windows(2).enumerate() {
            if p.iter().any(|x| !x.is_finite()) || p[0] >= p[1] {
                return Err(refused("Offset cell partition is not increasing"));
            }
            let net = extraction::interval(
                &curves[cell.curve],
                &Q::from_float(p[0]).unwrap(),
                &Q::from_float(p[1]).unwrap(),
            )?;
            if !offset::bound(
                &net,
                part.points[i],
                part.points[i + 1],
                distance,
                deviation * 0.25,
            )? {
                return Err(refused("Offset cell pointwise claim is false"));
            }
        }
    }
    for (i, p) in c.joins.iter().enumerate() {
        let j = (i + 1) % cells.len();
        if p.len() < 2
            || p.len() > 1025
            || p.iter().flatten().any(|x| !x.is_finite())
            || p[0] != *c.parts[i].points.last().unwrap()
            || *p.last().unwrap() != c.parts[j].points[0]
        {
            return Err(refused("Offset join connectivity mismatch"));
        }
        let (ra, rb) = rays(&cells[i], &cells[j], distance);
        if distance == 0.
            || c.join_style == "bevel"
            || cross(&ra, &rb).is_zero() && dot(&ra, &rb) > Q::zero()
        {
            if p.len() != 2 {
                return Err(refused("Bevel join has extra vertices"));
            }
            continue;
        }
        if c.join_style == "miter" {
            if p.len() != 3
                || error_squared(
                    p[1],
                    &miter_box(&cells[i], &cells[j], distance, miter_limit)?,
                )? > Q::from_float(deviation).unwrap().pow(2)
            {
                return Err(refused("Miter error claim is false"));
            }
        } else if !round_check(&cells[i], &cells[j], distance, deviation, p)? {
            return Err(refused("Round join error or angular coverage is false"));
        }
    }
    Ok(VerifiedOffsetWire {
        segments,
        maximum_deviation_mm: deviation,
    })
}
#[cfg(feature = "codec")]
mod serialization {
    use super::*;
    use value_codec::{Deserialize, Serialize, Value, error, json};
    impl Serialize for OffsetWirePart {
        fn to_value(&self) -> Value {
            json!({"parameters":self.parameters,"points":self.points})
        }
    }
    impl<'de> Deserialize<'de> for OffsetWirePart {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            if v.as_object().is_none_or(|o| {
                o.len() != 2 || !o.contains_key("parameters") || !o.contains_key("points")
            }) || ["parameters", "points"]
                .iter()
                .any(|k| v[*k].as_array().is_none_or(|a| a.len() > 2049))
            {
                return Err(error("Offset wire part schema or size mismatch"));
            }
            Ok(Self {
                parameters: Deserialize::from_value(v["parameters"].clone())?,
                points: Deserialize::from_value(v["points"].clone())?,
            })
        }
    }
    impl Serialize for OffsetWireCertificate {
        fn to_value(&self) -> Value {
            json!({"version":self.version,"source":self.source,"closed":self.closed,"joinStyle":self.join_style,"parts":self.parts,"joins":self.joins,"scope":"represented-joined-normal-offset-wire-approximation"})
        }
    }
    impl<'de> Deserialize<'de> for OffsetWireCertificate {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            let keys = [
                "version",
                "source",
                "closed",
                "joinStyle",
                "parts",
                "joins",
                "scope",
            ];
            if v.as_object()
                .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
                || v["scope"].as_str()
                    != Some("represented-joined-normal-offset-wire-approximation")
                || v["source"].as_str().is_none_or(|s| s.len() > 65800)
                || v["joinStyle"]
                    .as_str()
                    .is_none_or(|s| !matches!(s, "bevel" | "miter" | "round"))
                || v["parts"].as_array().is_none_or(|a| a.len() > 128)
                || v["joins"].as_array().is_none_or(|a| {
                    a.len() > 128
                        || a.iter()
                            .any(|p| p.as_array().is_none_or(|p| p.len() > 1025))
                })
            {
                return Err(error("Offset wire certificate schema or size mismatch"));
            }
            Ok(Self {
                version: Deserialize::from_value(v["version"].clone())?,
                source: Deserialize::from_value(v["source"].clone())?,
                closed: Deserialize::from_value(v["closed"].clone())?,
                join_style: Deserialize::from_value(v["joinStyle"].clone())?,
                parts: Deserialize::from_value(v["parts"].clone())?,
                joins: Deserialize::from_value(v["joins"].clone())?,
            })
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
        Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
    }
    #[test]
    fn irrational_oblique_normals_and_antipodal_round_joins_are_checked(){
        for end in [[8.,2.],[2.,2.],[0.,0.]]{for d in [-1.,1.]{
            let curves=vec![line([0.,0.],[5.,0.]),line([5.,0.],end)];
            for style in ["bevel","round"]{
                let c=certify_offset_wire(&curves,d,0.01,false,style,10.).unwrap();verify_offset_wire(&curves,d,0.01,10.,&c).unwrap();
                if style=="round"{
                    let mut wrong=c.clone();wrong.joins[0][1]=[5.+100.*d,100.];
                    assert!(verify_offset_wire(&curves,d,0.01,10.,&wrong).is_err());
                }
            }
            if end!=[0.,0.]{let c=certify_offset_wire(&curves,d,0.01,false,"miter",10.).unwrap();verify_offset_wire(&curves,d,0.01,10.,&c).unwrap();}
            else{assert!(certify_offset_wire(&curves,d,0.01,false,"miter",10.).is_err());}
        }}
    }
    #[test]
    fn zero_offset_preserves_a_collapsed_target_point_instead_of_an_empty_image() {
        let source = line([2., 3.], [2., 3.]);
        let c = certify_offset_wire(&[source.clone()], 0., 1e-4, false, "bevel", 1.).unwrap();
        verify_offset_wire(&[source], 0., 1e-4, 1., &c).unwrap();
        assert_eq!(
            c.curves().unwrap()[0].control_points,
            vec![vec![2., 3.], vec![2., 3.]]
        );
    }
    #[test]
    fn signed_corner_joins_are_bounded_connected_and_source_bound() {
        let curves = vec![line([0., 0.], [10., 0.]), line([10., 0.], [10., 10.])];
        for d in [-2., 2.] {
            for style in ["bevel", "miter", "round"] {
                let c = certify_offset_wire(&curves, d, 1e-4, false, style, 5.).unwrap();
                verify_offset_wire(&curves, d, 1e-4, 5., &c).unwrap();
                assert_eq!(c.parts.len(), 2);
                let mut bad = c.clone();
                bad.joins[0][0][0] += 1.;
                assert!(verify_offset_wire(&curves, d, 1e-4, 5., &bad).is_err());
                if c.joins[0].len() > 2 {
                    let mut bad = c.clone();
                    bad.joins[0][1][0] += 1.;
                    assert!(verify_offset_wire(&curves, d, 1e-4, 5., &bad).is_err());
                }
                assert!(verify_offset_wire(&curves, -d, 1e-4, 5., &c).is_err());
            }
        }
        assert!(certify_offset_wire(&curves, 2., 1e-4, false, "miter", 1.).is_err());
    }
    #[test]
    fn closed_polygon_and_internal_nurbs_corner_have_complete_joins() {
        let curve = Curve {
            degree: 1,
            knots: vec![0., 0., 0.25, 0.5, 0.75, 1., 1.],
            control_points: vec![
                vec![0., 0.],
                vec![10., 0.],
                vec![10., 10.],
                vec![0., 10.],
                vec![0., 0.],
            ],
            weights: vec![1., 2., 3., 2., 1.],
            periodic: false,
        };
        for style in ["bevel", "miter", "round"] {
            let c = certify_offset_wire(&[curve.clone()], -1., 1e-3, true, style, 3.).unwrap();
            assert_eq!(c.joins.len(), 4);
            verify_offset_wire(&[curve.clone()], -1., 1e-3, 3., &c).unwrap();
            let out = c.curves().unwrap();
            for i in 0..out.len() {
                assert_eq!(
                    out[i].control_points.last(),
                    out[(i + 1) % out.len()].control_points.first()
                );
            }
            let mut bad = c.clone();
            bad.joins.pop();
            assert!(verify_offset_wire(&[curve.clone()], -1., 1e-3, 3., &bad).is_err());
        }
    }
}
