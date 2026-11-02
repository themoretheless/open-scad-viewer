//! Original trimmed SIDE CARRIER profiles, not substituted top rim curves.
use super::*;
use crate::profile_certificate::original_restriction::{
    self as exact,
    wire::{self, jordan, nesting, regularity::SourceRegularity},
};
#[derive(Clone, Debug)]
pub struct NativeProfiles {
    supports: SourceSupports,
    regularity: Vec<SourceRegularity>,
    region: nesting::Nesting,
    oriented_faces: Vec<usize>,
}
fn derive(
    supports: &SourceSupports,
) -> Result<(Vec<SourceRegularity>, nesting::Nesting, Vec<usize>)> {
    supports.reverify()?;
    let mut wires = Vec::new();
    let mut regularity = Vec::new();
    let mut oriented_faces = BTreeSet::new();
    for (wi, raw) in supports.raw.iter().enumerate() {
        let mut pieces = Vec::new();
        for ci in 0..raw.len() {
            let bindings = supports
                .bindings
                .iter()
                .filter(|b| b.source == [wi, ci])
                .collect::<Vec<_>>();
            if bindings.len() != 1 {
                return Err(error("Native profile has no unique original rim binding"));
            }
            let binding = bindings[0];
            let data = supports
                .faces
                .iter()
                .find(|f| f.face == binding.face)
                .ok_or_else(|| error("Native profile side definition missing"))?;
            let uses = supports.source.shells[0]
                .faces
                .iter()
                .filter(|u| u.face == data.face)
                .collect::<Vec<_>>();
            if uses.len() != 1 {
                return Err(error("Native profile side has no unique shell use"));
            }
            let axis_sign = if data.axis == 0 { -1 } else { 1 };
            let height_sign = if data.vertical_z[1] > data.vertical_z[0] {
                1
            } else {
                -1
            };
            let profile_sign = if binding.window[1] > binding.window[0] {
                1
            } else {
                -1
            };
            let use_sign = if uses[0].reversed { -1 } else { 1 };
            if axis_sign * height_sign * profile_sign * use_sign != 1 {
                return Err(error(
                    "Native side carrier normal opposes the material boundary",
                ));
            }
            oriented_faces.insert(data.face);
            let native = exact::restrict_binary64(&data.profile, binding.window)?;
            let original_rim = raw[ci].full()?;
            for (native_t, rim_t) in native
                .parameter_range()
                .into_iter()
                .zip(original_rim.parameter_range())
            {
                if native.evaluate_natural(native_t)? != original_rim.evaluate_natural(rim_t)? {
                    return Err(error(
                        "Native side profile has mismatched directed rim endpoints",
                    ));
                }
            }
            pieces.push(native);
        }
        let mask = vec![false; pieces.len()];
        let wire = wire::sew(pieces, mask, ["0".into(), "1".into()])?;
        regularity.push(wire.qualify_source_regularity()?);
        wires.push(wire);
    }
    if oriented_faces.len() != supports.faces.len() {
        return Err(error("Native oriented side census incomplete"));
    }
    let jordan = jordan::qualify(&wires, jordan::Limits::default())?;
    let region = nesting::qualify(&jordan, nesting::Limits::default())?;
    Ok((regularity, region, oriented_faces.into_iter().collect()))
}
impl SourceSupports {
    pub fn qualify_native_profiles(&self) -> Result<NativeProfiles> {
        let (regularity, region, oriented_faces) = derive(self)?;
        Ok(NativeProfiles {
            supports: self.clone(),
            regularity,
            region,
            oriented_faces,
        })
    }
}
impl NativeProfiles {
    pub fn supports(&self) -> &SourceSupports {
        &self.supports
    }
    pub fn regularity(&self) -> &[SourceRegularity] {
        &self.regularity
    }
    pub fn region(&self) -> &nesting::Nesting {
        &self.region
    }
    pub fn oriented_faces(&self) -> &[usize] {
        &self.oriented_faces
    }
    pub fn reverify(&self) -> Result<()> {
        let (regularity, region, faces) = derive(&self.supports)?;
        if self.oriented_faces != faces
            || self.regularity.len() != regularity.len()
            || self.region.orientations() != region.orientations()
            || self.region.containment() != region.containment()
            || self.region.jordan().pairs() != region.jordan().pairs()
        {
            return Err(error("Native profile region/orientation replay changed"));
        }
        self.region.reverify()?;
        for (a, b) in self.regularity.iter().zip(regularity) {
            a.reverify()?;
            if a.span_count() != b.span_count() || a.internal_joins() != b.internal_joins() {
                return Err(error("Native profile regularity replay changed"));
            }
            for (a, b) in a.wire().sources().iter().zip(b.wire().sources()) {
                if a.source() != b.source() || a.parameter_range() != b.parameter_range() {
                    return Err(error("Native trimmed carrier definition changed"));
                }
            }
        }
        Ok(())
    }
}
