//! Complete endpoint-intersection-set proof for two retained clamped spans.
//! Cross-component contact is reported without mislabelling a touching
//! compound or a touching hole as an ordinary manifold profile/solid.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndpointContactCertificate {
    pub version: u32,
    pub source: String,
    pub separator: u16,
    pub common_endpoints: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedEndpointContacts {
    common_endpoints: usize,
}
impl VerifiedEndpointContacts {
    pub fn common_endpoints(&self) -> usize {
        self.common_endpoints
    }
}
fn input(a: &Curve, b: &Curve) -> Result<(String, Piece, Piece, usize)> {
    let source = source_bits(&[vec![a.clone()], vec![b.clone()]], 1e-7)?;
    let nets = extraction::nets(&[vec![a.clone()], vec![b.clone()]])?;
    if nets.len() != 2 || nets[0].0 != 0 || nets[1].0 != 1 {
        return Err(refused(
            "Endpoint contact proof needs one active span per curve",
        ));
    }
    let x = Piece::from_net(0, nets[0].1.clone());
    let y = Piece::from_net(1, nets[1].1.clone());
    let mut common = Vec::new();
    for p in [&x.points[0], x.points.last().unwrap()] {
        if [&y.points[0], y.points.last().unwrap()].contains(&p) && !common.contains(&p) {
            common.push(p);
        }
    }
    let count = common.len();
    Ok((format!("endpoint-contact-v1/{source}"), x, y, count))
}
pub fn certify_endpoint_contacts(a: &Curve, b: &Curve) -> Result<EndpointContactCertificate> {
    let (source, x, y, count) = input(a, b)?;
    let separator = normal_candidates(&x, &y)
        .find(|w| normal(&x, &y, *w).is_some_and(|n| separated(&x, &y, &n, count > 0)))
        .or_else(|| {
            arithmetic::endpoint_only(&x.net, &y.net, count > 0)
                .unwrap_or(false)
                .then_some(u16::MAX)
        })
        .or_else(|| {
            arithmetic::endpoint_census(&x.net, &y.net, count > 0)
                .unwrap_or(false)
                .then_some(u16::MAX - 1)
        })
        .ok_or_else(|| refused("Complete endpoint contact set remains unproved"))?;
    let c = EndpointContactCertificate {
        version: 1,
        source,
        separator,
        common_endpoints: count,
    };
    verify_endpoint_contacts(a, b, &c)?;
    Ok(c)
}
pub fn verify_endpoint_contacts(
    a: &Curve,
    b: &Curve,
    c: &EndpointContactCertificate,
) -> Result<VerifiedEndpointContacts> {
    let (source, x, y, count) = input(a, b)?;
    if c.version != 1 || c.source != source || c.common_endpoints != count {
        return Err(refused("Endpoint contact source or count mismatch"));
    }
    let proved = if c.separator == u16::MAX {
        arithmetic::endpoint_only(&x.net, &y.net, count > 0)?
    } else if c.separator == u16::MAX - 1 {
        arithmetic::endpoint_census(&x.net, &y.net, count > 0)?
    } else {
        normal(&x, &y, c.separator).is_some_and(|n| separated(&x, &y, &n, count > 0))
    };
    if !proved {
        return Err(refused("Endpoint contact separation claim is false"));
    }
    Ok(VerifiedEndpointContacts {
        common_endpoints: count,
    })
}
#[cfg(feature = "codec")]
mod serialization {
    use super::*;
    use value_codec::{Deserialize, Serialize, Value, error, json};
    impl Serialize for EndpointContactCertificate {
        fn to_value(&self) -> Value {
            json!({"version":self.version,"source":self.source,"separator":self.separator,"commonEndpoints":self.common_endpoints,"scope":"represented-endpoint-intersection-set"})
        }
    }
    impl<'de> Deserialize<'de> for EndpointContactCertificate {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            let keys = ["version", "source", "separator", "commonEndpoints", "scope"];
            let o = v
                .as_object()
                .ok_or_else(|| error("Expected endpoint contact certificate"))?;
            if o.len() != keys.len()
                || keys.iter().any(|k| !o.contains_key(*k))
                || v["scope"].as_str() != Some("represented-endpoint-intersection-set")
                || v["source"].as_str().is_none_or(|s| s.len() > 65700)
            {
                return Err(error("Endpoint contact schema or scope mismatch"));
            }
            Ok(Self {
                version: Deserialize::from_value(v["version"].clone())?,
                source: Deserialize::from_value(v["source"].clone())?,
                separator: Deserialize::from_value(v["separator"].clone())?,
                common_endpoints: Deserialize::from_value(v["commonEndpoints"].clone())?,
            })
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn quadratic(y: f64) -> Curve {
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., y], vec![2., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        }
    }
    #[test]
    fn two_endpoint_lens_is_certified_without_degenerate_chord_orientation() {
        let a = quadratic(-1.);
        let b = quadratic(1.).reverse().unwrap();
        let c = certify_endpoint_contacts(&a, &b).unwrap();
        assert_eq!(c.common_endpoints, 2);
        verify_endpoint_contacts(&a, &b, &c).unwrap();
        let loops = vec![vec![a, b]];
        let p = certify(&loops, 1e-7).unwrap();
        assert!(p.orientation_depth.is_some());
        verify(&loops, 1e-7, &p).unwrap();
    }
    #[test]
    fn quartic_curved_contact_uses_a_complete_exact_eliminant() {
        let mut a = quadratic(1.);
        a.degree = 4;
        a.knots = vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.];
        a.weights = vec![1.; 5];
        a.control_points = vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![2., 2.],
            vec![3., 1.],
            vec![4., 13.],
        ];
        let mut b = a.clone();
        b.control_points = vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![2., 1.],
            vec![3., 3.],
            vec![4., 7.],
        ];
        let c = certify_endpoint_contacts(&a, &b).unwrap();
        assert_eq!(c.common_endpoints, 1);
        assert_eq!(c.separator, u16::MAX - 1);
        verify_endpoint_contacts(&a, &b, &c).unwrap();
        let vertical = Curve::from_polyline(vec![vec![4., 13.], vec![4., 7.]]).unwrap();
        let loops = vec![vec![b, vertical.reverse().unwrap(), a.reverse().unwrap()]];
        let c = certify(&loops, 1e-7).unwrap();
        assert!(c.separating_normals.contains(&(u16::MAX - 1)));
        verify(&loops, 1e-7, &c).unwrap();
    }
    #[test]
    fn contact_proof_rejects_overlap_and_forged_empty_census() {
        let a = quadratic(-1.);
        let b = quadratic(1.);
        let mut c = certify_endpoint_contacts(&a, &b).unwrap();
        c.common_endpoints = 0;
        assert!(verify_endpoint_contacts(&a, &b, &c).is_err());
        assert!(certify_endpoint_contacts(&a, &a).is_err());
        let line = Curve::from_polyline(vec![vec![0., -0.5], vec![2., -0.5]]).unwrap();
        assert!(certify_endpoint_contacts(&a, &line).is_err());
    }
}
