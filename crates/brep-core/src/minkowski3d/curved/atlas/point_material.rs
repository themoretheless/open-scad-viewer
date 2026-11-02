//! Exact point membership transferred through the entire source homotopy.
use super::*;

pub(crate) struct MaterialOracle {
    patches: Vec<Patch>,
    faces: Vec<Face>,
}
impl MaterialOracle {
    pub(crate) fn new(model: &Model, depth: u8) -> Result<Self> {
        let (certificate, patches) = checked_charts(model, depth)?;
        difference::connected_outer_volume(model, &certificate)?;
        let (_, faces) = boolean::nominal(&certificate);
        Ok(Self { patches, faces })
    }
    pub(crate) fn classify(&self, point: &P, work: &mut Work) -> Result<i32> {
        let h = [
            point[0].clone(),
            point[1].clone(),
            point[2].clone(),
            Q::one(),
        ];
        let net = vec![vec![h]];
        let stationary = [net.clone(), net];
        for patch in &self.patches {
            if !disjoint(&stationary, &patch.motion, 0, work)? {
                return Err(fail(
                    "material seed is not separated from the FULL source homotopy",
                ));
            }
        }
        let winding = winding(point, &self.faces, work)?;
        if !matches!(winding, 0 | 1) {
            return Err(fail("original solid winding is neither zero nor one"));
        }
        Ok(winding)
    }
}
