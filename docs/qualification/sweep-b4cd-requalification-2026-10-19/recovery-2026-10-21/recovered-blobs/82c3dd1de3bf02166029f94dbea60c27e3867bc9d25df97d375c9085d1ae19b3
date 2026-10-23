//! Global positive-winding cleanup of checked joined offset wires. Every
//! crossing, overlap, removed border and output rounding is independently replayed.
use super::*;
use algebraic_intersections::{ExactRatio, decode_ratio, ratio};
use num_traits::ToPrimitive;
pub(super) mod arrangement;
use arrangement::Rings;
use num_traits::Signed;
#[derive(Clone, Debug, PartialEq)]
pub struct OffsetRegionCertificate {
    pub version: u32,
    pub source: String,
    pub join_style: String,
    pub original_topology: ProfileCertificate,
    pub wires: Vec<OffsetWireCertificate>,
    pub exact_rings: Vec<Vec<[ExactRatio; 2]>>,
}
#[derive(Clone, Debug)]
pub struct VerifiedOffsetRegions {
    components: Vec<Vec<Vec<Curve>>>,
    removed_internal_edges: bool,
}
impl VerifiedOffsetRegions {
    pub fn components(&self) -> &[Vec<Vec<Curve>>] {
        &self.components
    }
    pub fn global_regularization_checked(&self) -> bool {
        self.removed_internal_edges
    }
}
fn source(
    loops: &[Vec<Curve>],
    distance: f64,
    deviation: f64,
    style: &str,
    limit: f64,
) -> Result<String> {
    if !distance.is_finite()
        || distance.abs() > 1e6
        || !deviation.is_finite()
        || !(1e-6..=1.).contains(&deviation)
        || !limit.is_finite()
        || limit <= 0.
        || limit > 1e6
        || !matches!(style, "bevel" | "miter" | "round")
    {
        return Err(refused("Invalid offset-region options"));
    }
    Ok(format!(
        "offset-positive-regions-v1/{:016x}/{:016x}/{style}/{:016x}/{}",
        distance.to_bits(),
        deviation.to_bits(),
        limit.to_bits(),
        source_bits(loops, 1e-7)?
    ))
}
fn wire_rings(wires: &[OffsetWireCertificate]) -> Result<Rings> {
    let mut rings = Vec::new();
    let mut count = 0;
    for wire in wires {
        let curves = wire.curves()?;
        let mut points = Vec::new();
        let mut last = None;
        for curve in curves {
            let a = &curve.control_points[0];
            let b = curve.control_points.last().unwrap();
            if last.as_ref().is_some_and(|p: &Vec<f64>| p != a) {
                return Err(refused("Offset raw wire is disconnected"));
            }
            points.push([Q::from_float(a[0]).unwrap(), Q::from_float(a[1]).unwrap()]);
            last = Some(b.clone());
        }
        if points.len() < 3
            || last.as_ref().is_none_or(|p| {
                Q::from_float(p[0]).unwrap() != points[0][0]
                    || Q::from_float(p[1]).unwrap() != points[0][1]
            })
        {
            return Err(refused("Offset raw wire is not a closed region boundary"));
        }
        count += points.len();
        if count > 1022 {
            return Err(refused("Offset-region raw edge budget exceeded"));
        }
        rings.push(points);
    }
    Ok(rings)
}
fn decode_rings(value: &[Vec<[ExactRatio; 2]>]) -> Result<Rings> {
    if value.len() > 256 || value.iter().map(Vec::len).sum::<usize>() > 4096 {
        return Err(refused("Exact offset-region output size exceeded"));
    }
    value
        .iter()
        .map(|r| {
            if r.len() < 3 {
                return Err(refused("Exact offset region has a short ring"));
            }
            r.iter()
                .map(|p| Ok([decode_ratio(&p[0])?, decode_ratio(&p[1])?]))
                .collect()
        })
        .collect()
}
pub(super) fn rounded(rings: &Rings, deviation: f64) -> Result<(Rings, Vec<Vec<Vec<Curve>>>)> {
    let max = Q::from_float(deviation * 0.5).unwrap().pow(2);
    let mut q = Vec::new();
    for ring in rings {
        let mut exact = Vec::new();
        for p in ring {
            let x = p.clone().map(|v| v.to_f64().unwrap_or(f64::NAN));
            if x.iter().any(|v| !v.is_finite()) {
                return Err(refused("Offset region output exceeds numeric range"));
            }
            let v = x.map(|v| Q::from_float(v).unwrap());
            let error = (0..2).fold(Q::zero(), |sum, k| sum + (&p[k] - &v[k]).pow(2));
            if error > max {
                return Err(refused("Offset-region vertex rounding exceeds its bound"));
            }
            exact.push(v);
        }
        q.push(exact);
    }
    arrangement::check_rounding(rings, &q)?;
    if rings
        .iter()
        .zip(&q)
        .any(|(a, b)| arrangement::area(a).signum() != arrangement::area(b).signum())
        || arrangement::groups(rings)? != arrangement::groups(&q)?
    {
        return Err(refused(
            "Offset-region rounding changes ring orientation or ownership",
        ));
    }
    // Replayed positive fill must retain precisely the same ring components.
    let replay = arrangement::regularize(&q)?;
    let order = arrangement::groups(&replay)?;
    let mut components = Vec::new();
    for group in order {
        let mut component = Vec::new();
        for i in group {
            let points = replay[i]
                .iter()
                .map(|p| {
                    Ok([
                        p[0].to_f64()
                            .ok_or_else(|| refused("Offset coordinate range"))?,
                        p[1].to_f64()
                            .ok_or_else(|| refused("Offset coordinate range"))?,
                    ])
                })
                .collect::<Result<Vec<_>>>()?;
            component.push(crate::sketch::polygon_wire(points)?);
        }
        components.push(component);
    }
    Ok((q, components))
}
pub fn certify_offset_regions(
    loops: &[Vec<Curve>],
    distance: f64,
    deviation: f64,
    style: &str,
    miter_limit: f64,
) -> Result<OffsetRegionCertificate> {
    let source = source(loops, distance, deviation, style, miter_limit)?;
    let original_topology = certify(loops, 1e-7)?;
    let wires = loops
        .iter()
        .map(|wire| certify_offset_wire(wire, -distance, deviation * 0.5, true, style, miter_limit))
        .collect::<Result<Vec<_>>>()?;
    let rings = arrangement::regularize(&wire_rings(&wires)?)?;
    let c = OffsetRegionCertificate {
        version: 1,
        source,
        join_style: style.into(),
        original_topology,
        wires,
        exact_rings: rings
            .iter()
            .map(|r| r.iter().map(|p| [ratio(&p[0]), ratio(&p[1])]).collect())
            .collect(),
    };
    verify_offset_regions(loops, distance, deviation, miter_limit, &c)?;
    Ok(c)
}
pub fn verify_offset_regions(
    loops: &[Vec<Curve>],
    distance: f64,
    deviation: f64,
    miter_limit: f64,
    c: &OffsetRegionCertificate,
) -> Result<VerifiedOffsetRegions> {
    if c.version != 1
        || c.source != source(loops, distance, deviation, &c.join_style, miter_limit)?
        || c.wires.len() != loops.len()
    {
        return Err(refused("Offset-region source or wire census mismatch"));
    }
    verify(loops, 1e-7, &c.original_topology)?;
    for (wire, proof) in loops.iter().zip(&c.wires) {
        if !proof.closed || proof.join_style != c.join_style {
            return Err(refused("Offset-region wire style or closure changed"));
        }
        verify_offset_wire(wire, -distance, deviation * 0.5, miter_limit, proof)?;
    }
    let exact = decode_rings(&c.exact_rings)?;
    let reconstructed = arrangement::regularize(&wire_rings(&c.wires)?)?;
    if exact != reconstructed {
        return Err(refused(
            "Offset-region global winding or boundary claim is false",
        ));
    }
    let (_, components) = rounded(&exact, deviation)?;
    Ok(VerifiedOffsetRegions {
        components,
        removed_internal_edges: true,
    })
}
#[cfg(feature = "codec")]
mod serialization {
    use super::*;
    use value_codec::{Deserialize, Serialize, Value, error, json};
    impl Serialize for OffsetRegionCertificate {
        fn to_value(&self) -> Value {
            json!({"version":self.version,"source":self.source,"joinStyle":self.join_style,"originalTopology":self.original_topology,"wires":self.wires,"exactRings":self.exact_rings,"scope":"represented-offset-positive-region-regularization"})
        }
    }
    impl<'de> Deserialize<'de> for OffsetRegionCertificate {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            let keys = [
                "version",
                "source",
                "joinStyle",
                "originalTopology",
                "wires",
                "exactRings",
                "scope",
            ];
            if v.as_object()
                .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
                || v["scope"].as_str() != Some("represented-offset-positive-region-regularization")
                || v["source"].as_str().is_none_or(|s| s.len() > 66000)
                || v["wires"].as_array().is_none_or(|a| a.len() > 32)
                || v["exactRings"].as_array().is_none_or(|a| {
                    a.len() > 256
                        || a.iter()
                            .any(|r| r.as_array().is_none_or(|r| r.len() > 4096))
                })
            {
                return Err(error("Offset-region certificate schema or size mismatch"));
            }
            Ok(Self {
                version: Deserialize::from_value(v["version"].clone())?,
                source: Deserialize::from_value(v["source"].clone())?,
                join_style: Deserialize::from_value(v["joinStyle"].clone())?,
                original_topology: Deserialize::from_value(v["originalTopology"].clone())?,
                wires: Deserialize::from_value(v["wires"].clone())?,
                exact_rings: Deserialize::from_value(v["exactRings"].clone())?,
            })
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn box_wire(x: f64) -> Vec<Curve> {
        crate::sketch::polygon_wire(vec![[x, 0.], [x + 4., 0.], [x + 4., 4.], [x, 4.]]).unwrap()
    }
    #[test]
    fn overlapping_offsets_merge_globally_and_false_boundaries_fail() {
        let source = vec![box_wire(0.), box_wire(5.)];
        let c = certify_offset_regions(&source, 1., 0.02, "round", 4.).unwrap();
        let r = verify_offset_regions(&source, 1., 0.02, 4., &c).unwrap();
        assert_eq!(r.components.len(), 1);
        let mut bad = c;
        bad.exact_rings[0].pop();
        assert!(verify_offset_regions(&source, 1., 0.02, 4., &bad).is_err());
        crate::prism::extrude(&r.components[0], 0., 2.)
            .unwrap()
            .validate()
            .unwrap();
    }
    #[test]
    fn inward_corner_artifacts_are_removed_and_holes_keep_ownership() {
        let source = vec![box_wire(0.)];
        let c = certify_offset_regions(&source, -1., 0.02, "round", 4.).unwrap();
        let r = verify_offset_regions(&source, -1., 0.02, 4., &c).unwrap();
        assert_eq!(r.components.len(), 1);
        let area = crate::planar_trim::signed_area(&r.components[0][0], 1e-7).unwrap();
        assert!((area - 4.).abs() < 1e-10);
        let outer =
            crate::sketch::polygon_wire(vec![[0., 0.], [12., 0.], [12., 12.], [0., 12.]]).unwrap();
        let hole =
            crate::sketch::polygon_wire(vec![[4., 4.], [4., 8.], [8., 8.], [8., 4.]]).unwrap();
        let c =
            certify_offset_regions(&[outer.clone(), hole.clone()], 0.5, 0.02, "round", 4.).unwrap();
        let r = verify_offset_regions(&[outer, hole], 0.5, 0.02, 4., &c).unwrap();
        assert_eq!(r.components[0].len(), 2);
    }
}
