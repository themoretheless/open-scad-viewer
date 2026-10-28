//! Rebase original region inputs, preserving fresh inverse-composition admission.
use super::*;
use crate::source_inverse_shear::{Certificate, RegionReport};
impl SourceRegion {
    pub(crate) fn inverse_requalify(
        &self,
        certificate: &Certificate,
        limits: &ReplayLimits,
        max_work: u64,
        out: &mut RegionReport,
    ) -> Result<Option<Self>> {
        let Some(recipe) = rewrite(
            certificate,
            &self.recipe,
            limits.steps,
            limits.mapping_cells,
            max_work,
            out,
        )?
        else {
            return Ok(None);
        };
        let mut candidate = self.clone();
        candidate.recipe = recipe;
        Ok(Some(candidate.requalify(limits)?))
    }
}
fn rewrite(
    certificate: &Certificate,
    recipe: &Recipe,
    remaining: usize,
    max_mapping: usize,
    max_work: u64,
    out: &mut RegionReport,
) -> Result<Option<Recipe>> {
    if remaining == 0 {
        return Ok(None);
    }
    Ok(Some(match recipe {
        Recipe::SplitParameter {
            parent,
            address,
            parameter,
        } => {
            let Some(parent) = rewrite(
                certificate,
                parent,
                remaining - 1,
                max_mapping,
                max_work,
                out,
            )?
            else {
                return Ok(None);
            };
            Recipe::SplitParameter {
                parent: Box::new(parent),
                address: *address,
                parameter: *parameter,
            }
        }
        Recipe::SplitRoot {
            parent,
            address,
            point,
            role,
        } => {
            let Some(parent) = rewrite(
                certificate,
                parent,
                remaining - 1,
                max_mapping,
                max_work,
                out,
            )?
            else {
                return Ok(None);
            };
            let source = point.requalify(max_mapping)?;
            let Some(point) = source.point else {
                return Ok(None);
            };
            if point.surface() != certificate.source() || out.mapping_cells == max_mapping {
                return Ok(None);
            }
            let inverse = crate::source_contact_point::qualify(
                certificate.inverse(),
                point.boundary(),
                point.contact(),
                point.selector(),
                max_mapping - out.mapping_cells,
            )?;
            out.mapping_cells += inverse.mapping_cells;
            out.root_checks += 1;
            let Some(point) = inverse.point else {
                return Ok(None);
            };
            Recipe::SplitRoot {
                parent: Box::new(parent),
                address: *address,
                point,
                role: *role,
            }
        }
        Recipe::Original(original) => {
            if &original.surface != certificate.source() {
                return Ok(None);
            }
            let mut original = original.clone();
            for wire in &mut original.wires {
                for boundary in wire {
                    let pcurve = &boundary.pcurve;
                    let mut world = pcurve.clone();
                    world.control_points = pcurve
                        .control_points
                        .iter()
                        .map(|p| {
                            certificate
                                .inverse()
                                .evaluate(p[0], p[1])
                                .map(|e| e.point.to_vec())
                        })
                        .collect::<Result<Vec<_>>>()?;
                    if out.exact_work == max_work {
                        return Ok(None);
                    }
                    let Some(proof) = nurbs_core::curve_surface_agreement::verify_exact_algebraic(
                        &world,
                        pcurve,
                        certificate.inverse(),
                        false,
                        (max_work - out.exact_work).min(cad_predicates::MAX_WORK),
                    )?
                    else {
                        return Ok(None);
                    };
                    out.exact_work += proof.work_used;
                    if proof.outcome != cad_predicates::BezierIdentity::Equal {
                        return Ok(None);
                    }
                    boundary.curve = world;
                }
            }
            original.surface = certificate.inverse().clone();
            Recipe::Original(original)
        }
        Recipe::Cut { .. } => return Ok(None),
    }))
}
