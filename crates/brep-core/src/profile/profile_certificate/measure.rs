//! Independently checked Green-area enclosures. Decimal integer ratios cross
//! JSON without rounding; no editor area, samples, or floating-point quadrature
//! participate in this certificate. Topology is reverified on every admission.
use super::*;
use num_bigint::BigInt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactAreaInterval {
    pub lower: [String; 2],
    pub upper: [String; 2],
}
impl ExactAreaInterval {
    fn from_range(r: &orientation::Range) -> Self {
        let encode = |q: &Q| [q.numer().to_string(), q.denom().to_string()];
        Self {
            lower: encode(&r.lo),
            upper: encode(&r.hi),
        }
    }
    fn range(&self) -> Result<orientation::Range> {
        fn ratio(s: &[String; 2]) -> Result<Q> {
            let parse = |s: &str| -> Result<BigInt> {
                if s.is_empty() || s.len() > 16384 {
                    return Err(refused("Area ratio exceeds its size budget"));
                }
                let n = s
                    .parse::<BigInt>()
                    .map_err(|_| refused("Invalid exact area integer"))?;
                if n.to_string() != s {
                    return Err(refused("Noncanonical exact area integer"));
                }
                Ok(n)
            };
            let (n, d) = (parse(&s[0])?, parse(&s[1])?);
            if d <= BigInt::zero() {
                return Err(refused("Area denominator must be positive"));
            }
            let q = Q::new(n, d);
            if q.numer().to_string() != s[0] || q.denom().to_string() != s[1] {
                return Err(refused("Exact area fraction must be reduced"));
            }
            Ok(q)
        }
        let (lo, hi) = (ratio(&self.lower)?, ratio(&self.upper)?);
        if lo > hi {
            return Err(refused("Exact area interval is reversed"));
        }
        Ok(orientation::Range { lo, hi })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileMeasureCertificate {
    pub version: u32,
    pub topology: ProfileCertificate,
    pub area_depth: u8,
    pub area_mm2: ExactAreaInterval,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedMeasuredProfile {
    profile: VerifiedProfile,
    area_mm2: ExactAreaInterval,
}
impl VerifiedMeasuredProfile {
    pub fn profile(&self) -> &VerifiedProfile {
        &self.profile
    }
    pub fn area_mm2(&self) -> &ExactAreaInterval {
        &self.area_mm2
    }
}

fn total(loops: &[Vec<Curve>], depth: u8) -> Result<orientation::Range> {
    let mut total = orientation::Range {
        lo: Q::zero(),
        hi: Q::zero(),
    };
    for r in orientation::bounds(loops, depth)? {
        total.lo += r.lo;
        total.hi += r.hi;
    }
    Ok(total)
}

pub fn certify_measure(
    loops: &[Vec<Curve>],
    tolerance: f64,
    maximum_width_mm2: f64,
) -> Result<ProfileMeasureCertificate> {
    if !maximum_width_mm2.is_finite() || maximum_width_mm2 <= 0. {
        return Err(refused("Area enclosure width must be positive and finite"));
    }
    let topology = certify(loops, tolerance)?;
    let maximum = Q::from_float(maximum_width_mm2).unwrap();
    for area_depth in 0..=12 {
        let r = total(loops, area_depth)?;
        if &r.hi - &r.lo <= maximum {
            let c = ProfileMeasureCertificate {
                version: 1,
                topology,
                area_depth,
                area_mm2: ExactAreaInterval::from_range(&r),
            };
            verify_measure(loops, tolerance, &c)?;
            return Ok(c);
        }
    }
    Err(refused(
        "Requested exact area enclosure remains unproved within its work budget",
    ))
}

pub fn verify_measure(
    loops: &[Vec<Curve>],
    tolerance: f64,
    c: &ProfileMeasureCertificate,
) -> Result<VerifiedMeasuredProfile> {
    if c.version != 1 {
        return Err(refused("Unknown profile measure certificate version"));
    }
    let profile = verify(loops, tolerance, &c.topology)?;
    let claimed = c.area_mm2.range()?;
    let actual = total(loops, c.area_depth)?;
    if claimed.lo > actual.lo || claimed.hi < actual.hi {
        return Err(refused(
            "Claimed exact area interval does not enclose the checked Green bounds",
        ));
    }
    Ok(VerifiedMeasuredProfile {
        profile,
        area_mm2: c.area_mm2.clone(),
    })
}

#[cfg(feature = "codec")]
mod serialization {
    use super::*;
    use value_codec::{Deserialize, Serialize, Value, error, json};
    impl Serialize for ExactAreaInterval {
        fn to_value(&self) -> Value {
            json!({"lower":self.lower,"upper":self.upper})
        }
    }
    impl<'de> Deserialize<'de> for ExactAreaInterval {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            let o = v
                .as_object()
                .ok_or_else(|| error("Expected exact area interval"))?;
            if o.len() != 2 || !o.contains_key("lower") || !o.contains_key("upper") {
                return Err(error("Exact area interval fields do not match schema"));
            }
            for k in ["lower", "upper"] {
                let a = v[k]
                    .as_array()
                    .ok_or_else(|| error("Expected exact area ratio"))?;
                if a.len() != 2 || a.iter().any(|s| s.as_str().is_none_or(|s| s.len() > 16384)) {
                    return Err(error("Exact area ratio exceeds its size budget"));
                }
            }
            let r = Self {
                lower: Deserialize::from_value(v["lower"].clone())?,
                upper: Deserialize::from_value(v["upper"].clone())?,
            };
            r.range().map_err(|e| error(e.to_string()))?;
            Ok(r)
        }
    }
    impl Serialize for ProfileMeasureCertificate {
        fn to_value(&self) -> Value {
            json!({"version":self.version,"scope":"represented-profile-topology-and-area",
                "topology":self.topology,"areaDepth":self.area_depth,"areaMm2":self.area_mm2})
        }
    }
    impl<'de> Deserialize<'de> for ProfileMeasureCertificate {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            let o = v
                .as_object()
                .ok_or_else(|| error("Expected profile measure certificate"))?;
            let keys = ["version", "scope", "topology", "areaDepth", "areaMm2"];
            if o.len() != keys.len()
                || keys.iter().any(|k| !o.contains_key(*k))
                || v["scope"].as_str() != Some("represented-profile-topology-and-area")
            {
                return Err(error(
                    "Profile measure certificate fields or scope do not match schema",
                ));
            }
            Ok(Self {
                version: Deserialize::from_value(v["version"].clone())?,
                topology: Deserialize::from_value(v["topology"].clone())?,
                area_depth: Deserialize::from_value(v["areaDepth"].clone())?,
                area_mm2: Deserialize::from_value(v["areaMm2"].clone())?,
            })
        }
    }
}
