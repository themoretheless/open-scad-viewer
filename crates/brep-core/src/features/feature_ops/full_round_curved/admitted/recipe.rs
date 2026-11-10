//! Frozen proposal inputs. They are untrusted input, never proof fields.
//! Reconstructs exact geometry without rerunning floating contact solving.
use super::*;
use crate::profile_certificate::{
    original_restriction::{
        self as exact,
        wire::{self, jordan, nesting},
    },
    ExactRatio,
};
#[derive(Clone, Debug)]
pub struct Recipe {
    pub contact_ranges: [[ExactRatio; 2]; 2],
    pub bridges: Vec<Curve>,
    pub center: [ExactRatio; 2],
    pub radius: ExactRatio,
}
impl Recipe {
    pub(super) fn capture(proposed: &CurvedFullRound) -> Self {
        Self {
            contact_ranges: proposed
                .retained_supports
                .each_ref()
                .map(|s| s.parameter_range()),
            bridges: proposed.proposal.bridges().to_vec(),
            center: proposed.exact_radius.center(),
            radius: proposed.exact_radius.radius(),
        }
    }
}
pub fn restore(
    source: &Model,
    faces: [usize; 3],
    recipe: &Recipe,
    limits: material::Limits,
) -> Result<AdmittedFullRound> {
    source.validate()?;
    if source.bodies.len() != 1
        || source.shells.len() != 1
        || faces.into_iter().collect::<BTreeSet<_>>().len() != 3
        || recipe.bridges.len() != 2
    {
        return Err(error(
            "Invalid exact Full Round recipe selection or bridge census",
        ));
    }
    let p = crate::prism::recognize(source)?
        .ok_or_else(|| error("Exact Full Round recipe needs an XY prism"))?;
    let raw = raw_profiles::bind(source, p.z_max, &p.loops)?;
    let mut middle = Vec::new();
    for (i, c) in raw[0].iter().enumerate() {
        if c.belongs_to_face(source, faces[1])? {
            middle.push(i);
        }
    }
    if middle.len() != 1 || raw[0].len() < 4 {
        return Err(error(
            "Exact recipe middle does not own one outer source segment",
        ));
    }
    let i = middle[0];
    let prev = (i + raw[0].len() - 1) % raw[0].len();
    let next = (i + 1) % raw[0].len();
    if !((raw[0][prev].belongs_to_face(source, faces[0])?
        && raw[0][next].belongs_to_face(source, faces[2])?)
        || (raw[0][prev].belongs_to_face(source, faces[2])?
            && raw[0][next].belongs_to_face(source, faces[0])?))
    {
        return Err(error(
            "Exact recipe selected supports do not own original adjoining rims",
        ));
    }
    if recipe.contact_ranges[0][0] != raw[0][prev].full()?.parameter_range()[0]
        || recipe.contact_ranges[1][1] != raw[0][next].full()?.parameter_range()[1]
    {
        return Err(error("Exact recipe changes an unselected source interval"));
    }
    let retained = [
        exact::restrict(raw[0][prev].curve(), recipe.contact_ranges[0].clone())?,
        exact::restrict(raw[0][next].curve(), recipe.contact_ranges[1].clone())?,
    ];
    // Original directed support clocks must remain forward; no deleted/reversed
    // remainder or unrestricted source-carrier continuation is a valid recipe.
    for r in &retained {
        let d = r.parameter_range();
        let q = |r: &ExactRatio| -> Result<num_rational::BigRational> {
            Ok(num_rational::BigRational::new(
                r[0].parse()
                    .map_err(|_| error("Invalid recipe parameter"))?,
                r[1].parse()
                    .map_err(|_| error("Invalid recipe parameter"))?,
            ))
        };
        if q(&d[0])? >= q(&d[1])? {
            return Err(error("Exact recipe reverses an original retained support"));
        }
    }
    let mut sources = Vec::new();
    let mut editable = Vec::new();
    let mut crown = None;
    for j in 0..raw[0].len() {
        if j == prev || j == next {
            sources.push(retained[usize::from(j == next)].clone());
            editable.push(false);
        } else if j == i {
            for (b, curve) in recipe.bridges.iter().enumerate() {
                sources.push(exact::restrict_binary64(curve, curve.domain())?);
                editable.push(true);
                if b == 0 {
                    crown = Some(sources.len() - 1);
                }
            }
        } else {
            sources.push(raw[0][j].full()?);
            editable.push(false);
        }
    }
    let t = num_rational::BigRational::from_float(source.tolerance_mm / 32.)
        .ok_or_else(|| error("Invalid source tolerance"))?;
    let tolerance = [t.numer().to_string(), t.denom().to_string()];
    let outer = wire::sew_tangent_with_contacts(
        sources,
        editable,
        tolerance.clone(),
        vec![wire::LineContact::new(
            crown.ok_or_else(|| error("Missing recipe crown"))?,
            raw[0][i].full()?,
        )],
    )?;
    let tangency = outer.qualify_smooth_edits(["0".into(), "1".into()])?;
    let radius = outer.qualify_edited_radius(
        recipe.center.clone(),
        recipe.radius.clone(),
        tolerance.clone(),
    )?;
    let mut wires = vec![outer];
    for hole in &raw[1..] {
        wires.push(wire::sew(
            hole.iter()
                .map(raw_profiles::RawProfileCurve::full)
                .collect::<Result<Vec<_>>>()?,
            vec![false; hole.len()],
            tolerance.clone(),
        )?);
    }
    let result = prism::extrude_binary64(&wires, [p.z_min, p.z_max], source.tolerance_mm)?
        .qualify_solid(jordan::Limits::default(), nesting::Limits::default())?;
    let supports = source_supports::qualify(source, &raw, [p.z_min, p.z_max])?;
    let original = source_boundary::qualify(&supports)?.canonical_material()?;
    let material =
        material_relation::exact::inspect_owned(original, result, limits)?.require_no_addition()?;
    let admitted = AdmittedFullRound {
        material,
        radius,
        tangency,
        selected_faces: faces,
        recipe: recipe.clone(),
    };
    admitted.verify_ownership()?;
    Ok(admitted)
}
