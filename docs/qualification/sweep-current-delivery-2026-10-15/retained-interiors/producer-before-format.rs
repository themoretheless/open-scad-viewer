//! Qualification-only intrinsic differentiability of ACTUAL retained surfaces.
//! A positive rational tensor-product basis is analytic within every open span
//! and C^(degree-multiplicity) across each interior knot line. This proves
//! parametric C1/C2 only: immersion, sewn edges, periodic seams, source error
//! and global material admission require separate native proofs.
#[cfg(feature = "codec")]
mod enabled {
    use nurbs_core::surface::Surface;
    use sha2::{Digest, Sha256};
    use value_codec::{Value, json};

    type Answer<T> = Result<T, String>;

    fn inspect(surface: &Surface, work: &mut usize, maximum: usize) -> Answer<Value> {
        surface.validate().map_err(|error| error.to_string())?;
        let required = surface.weights.iter().map(Vec::len).sum::<usize>()
            + surface.knots_u.len() + surface.knots_v.len();
        if required > maximum.saturating_sub(*work) {
            return Err("input-item-budget-exhausted".into());
        }
        *work += required;
        let mut c1 = true;
        let mut c2 = true;
        let mut lines = Vec::new();
        for (axis, knots, degree, count) in [
            ("u", &surface.knots_u, surface.degree_u, surface.control_points.len()),
            ("v", &surface.knots_v, surface.degree_v, surface.control_points[0].len()),
        ] {
            let domain = [knots[degree], knots[count]];
            let mut start = 0;
            while start < knots.len() {
                let mut end = start + 1;
                while end < knots.len() && knots[end] == knots[start] { end += 1; }
                if knots[start] > domain[0] && knots[start] < domain[1] {
                    // Surface::validate -> Curve::validate_basis already
                    // rejects interior multiplicity degree+1, so C0 is real.
                    let multiplicity = end - start;
                    let order = degree.checked_sub(multiplicity)
                        .ok_or_else(|| "discontinuous-basis-not-admissible".to_string())?;
                    c1 &= order >= 1;
                    c2 &= order >= 2;
                    lines.push(json!({"axis":axis,"knot":knots[start],
                        "multiplicity":multiplicity,"degree":degree,"guaranteedBasisOrder":order}));
                }
                start = end;
            }
        }
        Ok(json!({"parametricC0Certified":true,"parametricC1Certified":c1,
            "parametricC2Certified":c2,"interiorKnotLines":lines,
            "periodicSeamsCertified":false,"surfaceRegularityCertified":false,
            "geometricG1G2Certified":false,
            "reason":if c2 {"positive-rational-basis-interior-C2"}else{"higher-order-knot-jet-proof-required"}}))
    }

    fn case(input: &Value, maximum: usize) -> Value {
        let mut work = 0;
        let mut faces = Vec::new();
        let mut reason = None;
        let mut owned = std::collections::BTreeSet::new();
        let Some(surfaces) = input["surfaces"].as_array() else {
            return json!({"file":input["file"],"complete":false,"reason":"missing-surfaces"});
        };
        if surfaces.is_empty() || surfaces.len() > brep_core::MAX_FACES {
            reason = Some("invalid-or-empty-surface-scope".to_string());
        } else {
            for entry in surfaces {
                let Some(face) = entry["face"].as_u64().filter(|&face| face < brep_core::MAX_FACES as u64) else {
                    reason = Some("invalid-face-owner".into()); break;
                };
                if !owned.insert(face) { reason = Some("duplicate-face-owner".into()); break; }
                let surface: Surface = match value_codec::from_value(entry["surface"].clone()) {
                    Ok(value) => value,
                    Err(error) => { reason = Some(error.to_string()); break; }
                };
                match inspect(&surface, &mut work, maximum) {
                    Ok(mut result) => { result["face"] = json!(face); faces.push(result); }
                    Err(error) => { reason = Some(error); break; }
                }
            }
        }
        let complete = reason.is_none() && faces.len() == surfaces.len();
        json!({"file":input["file"],"complete":complete,"reason":reason,
            "work":work,"maxInputItems":maximum,"inputItemUnit":"weights-and-knot-entries",
            "unvisitedFaces":surfaces.len()-faces.len(),"faces":faces})
    }

    fn hash(bytes: &[u8]) -> String {
        Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
    }

    pub fn run() {
        let args = std::env::args().skip(1).collect::<Vec<_>>();
        assert_eq!(args.len(), 2, "Supply exact surface request JSON and output JSON");
        let bytes = std::fs::read(&args[0]).unwrap();
        let input: Value = value_codec::from_str(std::str::from_utf8(&bytes).unwrap()).unwrap();
        let maximum = usize::try_from(input["maxInputItemsPerCase"].as_u64().unwrap()).unwrap();
        assert!(maximum <= 1000000);
        let requested = input["cases"].as_array().unwrap();
        assert!(!requested.is_empty() && requested.len() <= 256);
        let cases = requested.iter().map(|input| case(input, maximum)).collect::<Vec<_>>();
        let passed = cases.iter().all(|item| item["complete"] == json!(true));
        let output = json!({"schema":"retained-surface-interior-continuity/1","passed":passed,
            "requestSha256":hash(&bytes),"producerSha256":hash(include_bytes!("certify_retained_surface_interiors.rs")),
            "cases":cases,"continuousBound":false,"solidCertified":false,
            "scope":"Native positive-rational-basis sufficient C1/C2 proof on natural-domain interiors only. False means unproved by this condition, not a discontinuity counterexample. Immersion, physical trimmed domains, natural/periodic edges and global geometry remain separate."});
        std::fs::write(&args[1], value_codec::to_string(&output).unwrap()).unwrap();
        if !passed { std::process::exit(1); }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn graph(degree: usize, knots: Vec<f64>, count: usize) -> Surface {
            Surface {degree_u:degree,degree_v:1,knots_u:knots,knots_v:vec![0.,0.,1.,1.],
                control_points:(0..count).map(|i| vec![vec![i as f64,0.,0.],vec![i as f64,1.,0.]]).collect(),
                weights:vec![vec![1.,1.];count],periodic_u:false,periodic_v:false}
        }
        #[test]
        fn basis_orders_and_single_span_are_distinct_from_geometric_regularity() {
            let single = graph(1,vec![0.,0.,1.,1.],2);
            let proof = inspect(&single,&mut 0,1000).unwrap();
            assert_eq!(proof["parametricC2Certified"],true);
            assert_eq!(proof["surfaceRegularityCertified"],false);
            let c1 = graph(2,vec![0.,0.,0.,0.5,1.,1.,1.],4);
            let proof = inspect(&c1,&mut 0,1000).unwrap();
            assert_eq!(proof["parametricC1Certified"],true);
            assert_eq!(proof["parametricC2Certified"],false);
            let c2 = graph(3,vec![0.,0.,0.,0.,0.5,1.,1.,1.,1.],5);
            assert_eq!(inspect(&c2,&mut 0,1000).unwrap()["parametricC2Certified"],true);
            let c0 = graph(2,vec![0.,0.,0.,0.5,0.5,1.,1.,1.],5);
            assert_eq!(inspect(&c0,&mut 0,1000).unwrap()["parametricC1Certified"],false);
        }
        #[test]
        fn disconnected_basis_invalid_weights_and_exhausted_scope_refuse() {
            let disconnected = graph(1,vec![0.,0.,0.5,0.5,1.,1.],4);
            assert!(inspect(&disconnected,&mut 0,1000).is_err());
            let mut invalid = graph(1,vec![0.,0.,1.,1.],2);
            invalid.weights[0][0]=0.;assert!(inspect(&invalid,&mut 0,1000).is_err());
            let valid = graph(1,vec![0.,0.,1.,1.],2);
            assert!(inspect(&valid,&mut 0,0).is_err());
            assert_eq!(case(&json!({"file":"empty","surfaces":[]}),1000)["complete"],false);
            let duplicate = json!({"file":"duplicate","surfaces":[{"face":0,"surface":valid},{"face":0,"surface":valid}]});
            assert_eq!(case(&duplicate,1000)["complete"],false);
        }
    }
}

#[cfg(feature = "codec")]
fn main() { enabled::run(); }
#[cfg(not(feature = "codec"))]
fn main() { eprintln!("Native qualification input requires the codec feature"); std::process::exit(2); }
