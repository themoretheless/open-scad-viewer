use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PocketCoordinateFrame {
    pub axes: [i8; 3],
}
impl PocketCoordinateFrame {
    fn validate(&self) -> Result<()> {
        if self
            .axes
            .iter()
            .any(|a| a.unsigned_abs() < 1 || a.unsigned_abs() > 3)
            || self
                .axes
                .iter()
                .map(|a| a.unsigned_abs())
                .collect::<BTreeSet<_>>()
                .len()
                != 3
        {
            return Err(fail(
                "pocket coordinate frame is not a signed axis permutation",
            ));
        }
        Ok(())
    }
    fn negative(&self) -> bool {
        let mut s = self.axes.iter().filter(|a| **a < 0).count();
        for i in 0..3 {
            for j in i + 1..3 {
                if self.axes[i].abs() > self.axes[j].abs() {
                    s += 1;
                }
            }
        }
        s % 2 == 1
    }
    pub fn inverse(&self) -> Result<Self> {
        self.validate()?;
        let mut axes = [0; 3];
        for i in 0..3 {
            axes[self.axes[i].unsigned_abs() as usize - 1] = (i as i8 + 1) * self.axes[i].signum();
        }
        Ok(Self { axes })
    }
    fn point(&self, p: &[f64]) -> Vec<f64> {
        self.axes
            .iter()
            .map(|a| p[a.unsigned_abs() as usize - 1] * f64::from(a.signum()))
            .collect()
    }
    pub fn transform_surface(&self, s: &Surface) -> Result<Surface> {
        self.validate()?;
        let mut out = s.clone();
        for row in &mut out.control_points {
            for p in row {
                if p.len() != 3 {
                    return Err(fail(
                        "pocket frame requires three-dimensional surface controls",
                    ));
                }
                *p = self.point(p);
            }
        }
        Ok(out)
    }
    pub fn transform_model(&self, m: &Model) -> Result<Model> {
        self.validate()?;
        let mut out = m.clone();
        for v in &mut out.vertices {
            v.point = self.point(&v.point).try_into().unwrap();
        }
        for e in &mut out.edges {
            for p in &mut e.curve.control_points {
                if p.len() != 3 {
                    return Err(fail("pocket frame requires three-dimensional source edges"));
                }
                *p = self.point(p);
            }
        }
        for f in &mut out.faces {
            f.surface = self.transform_surface(&f.surface)?;
        }
        if self.negative() {
            for s in &mut out.shells {
                for f in &mut s.faces {
                    f.reversed ^= true;
                }
            }
        }
        out.rebuild_topology_ids();
        out.validate()?;
        Ok(out)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct FramedPocketModelCertificate {
    pub version: u32,
    pub source: String,
    pub floor: String,
    pub frame: PocketCoordinateFrame,
    pub local: PocketModelCertificate,
    pub result: String,
}
#[derive(Clone, Debug)]
pub struct CertifiedFramedPocketModel {
    model: Model,
    certificate: FramedPocketModelCertificate,
}
impl CertifiedFramedPocketModel {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn certificate(&self) -> &FramedPocketModelCertificate {
        &self.certificate
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ModelPocketRequest {
    pub source_face: usize,
    pub floor: Surface,
    pub domain: DomainPolynomial,
    pub factors: Vec<DomainPolynomial>,
    pub window: [[ExactRatio; 2]; 2],
    pub selected: Option<Vec<u32>>,
    pub depth: u8,
}
fn world_result(
    source: &Model,
    local: CertifiedPocketModel,
    frame: PocketCoordinateFrame,
) -> Result<CertifiedFramedPocketModel> {
    let mut result = frame.inverse()?.transform_model(local.model())?;
    result.inherit_topology_ids(&[source]);
    result.validate()?;
    let certificate = FramedPocketModelCertificate {
        version: 1,
        source: source_id(&Source::Brep(source.clone()))?,
        floor: local.certificate.pocket.floor_source.clone(),
        frame,
        local: local.certificate,
        result: source_id(&Source::Brep(result.clone()))?,
    };
    Ok(CertifiedFramedPocketModel {
        model: result,
        certificate,
    })
}
/// Exact signed-coordinate isometries permit carriers folded in the original
/// XY view. No nonlinear inverse projection or metric approximation is assumed.
pub fn certify_framed_pocket_boolean_model(
    source: &Model,
    request: &ModelPocketRequest,
    operation: &str,
    frame: PocketCoordinateFrame,
) -> Result<CertifiedFramedPocketModel> {
    let local_source = frame.transform_model(source)?;
    let local_floor = frame.transform_surface(&request.floor)?;
    let pocket = certify_model_pocket(
        &local_source,
        request.source_face,
        &local_floor,
        PocketRegion {
            domain: &request.domain,
            factors: &request.factors,
            window: request.window.clone(),
        },
        request.selected.as_deref(),
        request.depth,
    )?;
    world_result(
        source,
        certify_pocket_boolean_model(&pocket, operation)?,
        frame,
    )
}
/// Select the first structurally admissible signed frame (XYZ first). Once
/// selected, a material or conversion refusal never tries a different cut
/// direction and never falls through to a numerical geometry substitute.
pub fn certify_oriented_pocket_boolean_model(
    source: &Model,
    request: &ModelPocketRequest,
    operation: &str,
) -> Result<CertifiedFramedPocketModel> {
    source.validate()?;
    request.floor.validate()?;
    let face = source
        .faces
        .get(request.source_face)
        .ok_or_else(|| fail("oriented pocket source face is absent"))?;
    let used = source
        .shells
        .iter()
        .flat_map(|s| &s.faces)
        .filter(|f| f.face == request.source_face)
        .collect::<Vec<_>>();
    if used.len() != 1 {
        return Err(fail("oriented pocket source needs one shell face use"));
    }
    for axes in [
        [1, 2, 3],
        [1, 3, 2],
        [2, 1, 3],
        [2, 3, 1],
        [3, 1, 2],
        [3, 2, 1],
    ] {
        for sign in [1, -1] {
            let frame = PocketCoordinateFrame {
                axes: [axes[0], axes[1], axes[2] * sign],
            };
            let upper = frame.transform_surface(&face.surface)?;
            let lower = frame.transform_surface(&request.floor)?;
            if let Ok(a) = super::super::super::align(&upper, &lower, &mut Work::default()) {
                let orientation =
                    (&a.xy[0][1] * &a.xy[1][2] - &a.xy[0][2] * &a.xy[1][1]).is_negative();
                if used[0].reversed ^ frame.negative() == orientation {
                    return certify_framed_pocket_boolean_model(source, request, operation, frame);
                }
            }
        }
    }
    Err(fail(
        "no signed coordinate frame proves affine projections for both edited NURBS carriers",
    ))
}
pub fn verify_framed_pocket_boolean_model(
    source: &Model,
    floor: &Surface,
    result: &Model,
    c: &FramedPocketModelCertificate,
) -> Result<CertifiedFramedPocketModel> {
    if c.version != 1
        || c.source != source_id(&Source::Brep(source.clone()))?
        || c.result != source_id(&Source::Brep(result.clone()))?
    {
        return Err(fail("framed ordinary Model source/result binding mismatch"));
    }
    let local_source = c.frame.transform_model(source)?;
    let local_floor = c.frame.transform_surface(floor)?;
    if c.floor != super::super::super::source(&local_floor)? {
        return Err(fail("framed ordinary Model floor binding mismatch"));
    }
    let pocket = verify_model_pocket(&local_source, &local_floor, &c.local.pocket)?;
    let local = certify_pocket_boolean_model_with_arcs(&pocket, &c.local.operation, &c.local.arcs)?;
    let expected = world_result(source, local, c.frame)?;
    if expected.certificate != *c || expected.model != *result {
        return Err(fail(
            "framed ordinary Model geometry/topology certificate mismatch",
        ));
    }
    Ok(expected)
}
#[derive(Clone, Debug, PartialEq)]
pub struct PocketModelBatchCertificate {
    pub version: u32,
    pub source: String,
    pub steps: Vec<FramedPocketModelCertificate>,
    pub result: String,
}
#[derive(Clone, Debug)]
pub struct CertifiedPocketModelBatch {
    model: Model,
    certificate: PocketModelBatchCertificate,
}
impl CertifiedPocketModelBatch {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn certificate(&self) -> &PocketModelBatchCertificate {
        &self.certificate
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
/// A checked sequence on multiple original face slots, not one SS cut that
/// crosses their common trim edges. Each intermediate result is an ordinary
/// Model and must qualify as the next operation's actual source.
pub fn certify_model_pocket_batch(
    source: &Model,
    requests: &[ModelPocketRequest],
) -> Result<CertifiedPocketModelBatch> {
    if requests.is_empty() || requests.len() > 16 {
        return Err(fail("ordinary Model pocket batch size must be 1..16"));
    }
    let mut model = source.clone();
    let mut steps = vec![];
    for request in requests {
        let result = certify_oriented_pocket_boolean_model(&model, request, "difference")?;
        model = result.model;
        steps.push(result.certificate);
    }
    let certificate = PocketModelBatchCertificate {
        version: 1,
        source: source_id(&Source::Brep(source.clone()))?,
        steps,
        result: source_id(&Source::Brep(model.clone()))?,
    };
    Ok(CertifiedPocketModelBatch { model, certificate })
}
pub fn verify_model_pocket_batch(
    source: &Model,
    floors: &[Surface],
    result: &Model,
    c: &PocketModelBatchCertificate,
) -> Result<CertifiedPocketModelBatch> {
    if c.version != 1
        || c.source != source_id(&Source::Brep(source.clone()))?
        || c.steps.len() != floors.len()
        || c.steps.is_empty()
        || c.steps.len() > 16
    {
        return Err(fail("ordinary Model batch source/size binding mismatch"));
    }
    let mut model = source.clone();
    for (step, floor) in c.steps.iter().zip(floors) {
        let ls = step.frame.transform_model(&model)?;
        let lf = step.frame.transform_surface(floor)?;
        let pocket = verify_model_pocket(&ls, &lf, &step.local.pocket)?;
        let local = certify_pocket_boolean_model_with_arcs(
            &pocket,
            &step.local.operation,
            &step.local.arcs,
        )?;
        if step.local.operation != "difference" {
            return Err(fail(
                "ordinary Model pocket batch contains a different Boolean operation",
            ));
        }
        let next = world_result(&model, local, step.frame)?;
        if next.certificate != *step {
            return Err(fail("ordinary Model batch step proof mismatch"));
        }
        model = next.model;
    }
    if model != *result || c.result != source_id(&Source::Brep(model.clone()))? {
        return Err(fail("ordinary Model batch result mismatch"));
    }
    Ok(CertifiedPocketModelBatch {
        model,
        certificate: c.clone(),
    })
}
