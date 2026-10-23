//! Bind one canonical transition end curve to its patch and adjacent support.
//! This supplies owned sewing correspondence, not a complete end face/solid.
use crate::trim_sew::{
    self, BoundaryCorrespondence, BoundaryCorrespondenceInput, BoundaryUse,
    CurvePcurveCorrespondence, ParameterOrientation,
};
use cad_predicates::ToleranceContext;
use nurbs_core::{Error, Result, curve::Curve, offset_patch_boundary, surface::Surface};

pub struct QualifiedEnd {
    pub world_curve: Curve,
    pub patch_pcurve: Curve,
    pub supports: [CurvePcurveCorrespondence; 2],
    pub boundary: BoundaryCorrespondence,
}
/// The authored adjacent pcurve uses the canonical forward world parameter.
/// Owner reversal changes coedge traversal, never curve or pcurve definitions.
pub fn qualify(
    context: &ToleranceContext,
    patch: &Surface,
    adjacent: &Surface,
    adjacent_pcurve: &Curve,
    start: bool,
    shell: usize,
    owners: [BoundaryUse; 2],
) -> Result<QualifiedEnd> {
    adjacent.validate()?;
    adjacent_pcurve.validate()?;
    let boundary = offset_patch_boundary::extract(patch)?.ok_or_else(|| {
        Error::new(
            "BREP_OFFSET_END_BOUNDARY",
            "Transition patch lacks canonical endpoint boundaries",
        )
    })?;
    let edge = &boundary.coedges[if start { 3 } else { 1 }];
    if owners[0].reversed != edge.reversed
        || owners[0].cyclic_index != if start { 3 } else { 1 }
        || owners[0].face == owners[1].face
    {
        return Err(Error::new(
            "BREP_OFFSET_END_OWNERSHIP",
            "End boundary needs distinct faces and the patch boundary traversal",
        ));
    }
    let domain = edge.curve.domain();
    let uv = Curve {
        degree: 1,
        knots: vec![domain[0], domain[0], domain[1], domain[1]],
        control_points: vec![
            vec![edge.fixed_parameter, domain[0]],
            vec![edge.fixed_parameter, domain[1]],
        ],
        weights: vec![1., 1.],
        periodic: false,
    };
    let supports = [
        trim_sew::prove_curve_pcurve_correspondence(
            context,
            &edge.curve,
            patch,
            &uv,
            ParameterOrientation::Same,
            owners[0].clone(),
        )?,
        trim_sew::prove_curve_pcurve_correspondence(
            context,
            &edge.curve,
            adjacent,
            adjacent_pcurve,
            ParameterOrientation::Same,
            owners[1].clone(),
        )?,
    ];
    let first = &edge.curve.control_points[0];
    let last = edge.curve.control_points.last().unwrap();
    let points = [[first[0], first[1], first[2]], [last[0], last[1], last[2]]];
    let ordered = |reversed| {
        if reversed {
            [points[1], points[0]]
        } else {
            points
        }
    };
    let correspondence = trim_sew::prove_boundary_correspondence(
        context,
        BoundaryCorrespondenceInput {
            curve_a: &edge.curve,
            curve_b: &edge.curve,
            endpoints_a: ordered(owners[0].reversed),
            endpoints_b: ordered(owners[1].reversed),
            orientation: ParameterOrientation::Reversed,
            seam_shift: 0,
            shell,
            uses: owners,
        },
    )?;
    Ok(QualifiedEnd {
        world_curve: edge.curve.clone(),
        patch_pcurve: uv,
        supports,
        boundary: correspondence,
    })
}

/// A qualified shared end curve and an audited replacement adjacent UV region.
/// Contour work stops retain an unqualified report; callers must inspect it.
/// This remains a face recipe; no model mutation or closed-shell claim occurs.
pub struct ClippedEndReport {
    pub edge: QualifiedEnd,
    pub contours: nurbs_core::offset_face_loops::Report,
    pub contour_support: CurvePcurveCorrespondence,
}
pub fn qualify_clipped_end(
    context: &ToleranceContext,
    patch: &Surface,
    adjacent: &Surface,
    adjacent_pcurve: &Curve,
    loop_contact: &Curve,
    original_loops: &[Vec<Curve>],
    loop_index: usize,
    arc_start: usize,
    arc_count: usize,
    start: bool,
    shell: usize,
    owners: [BoundaryUse; 2],
    tolerance_uv: f64,
    limits: nurbs_core::offset_face_loops::Limits,
) -> Result<ClippedEndReport> {
    if owners[1].cyclic_index != 0 {
        return Err(Error::new(
            "BREP_OFFSET_END_OWNERSHIP",
            "Replacement contact starts the adjacent face contour at cyclic index zero",
        ));
    }
    let contour_owner = owners[1].clone();
    let edge = qualify(
        context,
        patch,
        adjacent,
        adjacent_pcurve,
        start,
        shell,
        owners,
    )?;
    let contour_support = trim_sew::prove_curve_pcurve_correspondence(
        context,
        &edge.world_curve,
        adjacent,
        loop_contact,
        if contour_owner.reversed {
            ParameterOrientation::Reversed
        } else {
            ParameterOrientation::Same
        },
        contour_owner,
    )?;
    let contours = nurbs_core::offset_face_loops::replace_boundary_arc(
        original_loops,
        loop_index,
        arc_start,
        arc_count,
        loop_contact,
        tolerance_uv,
        limits,
    )?;
    if contours.region_subset_proven {
        let winding = &contours.replacement.as_ref().unwrap().winding;
        if winding[0] != Some(1) || winding[1..].iter().any(|w| *w != Some(-1)) {
            return Err(Error::new(
                "BREP_OFFSET_END_WINDING",
                "B-rep face contours require a counterclockwise outer loop and clockwise holes",
            ));
        }
    }
    Ok(ClippedEndReport {
        edge,
        contours,
        contour_support,
    })
}

/// One canonical patch end bound to a fully audited replacement support face.
/// Sewing the face into a shell and geometric volume admission remain separate.
pub struct ReplacedEndFace {
    pub edge: QualifiedEnd,
    pub contour_support: CurvePcurveCorrespondence,
    pub face: crate::trimmed_face_recipe::ReplaceBoundary,
}
pub fn qualify_replaced_face(
    context: &ToleranceContext,
    patch: &Surface,
    adjacent: &Surface,
    adjacent_pcurve: &Curve,
    loop_contact: &Curve,
    original_wires: &[Vec<crate::trimmed_face_recipe::Boundary>],
    loop_index: usize,
    arc_start: usize,
    arc_count: usize,
    start: bool,
    shell: usize,
    owners: [BoundaryUse; 2],
    tolerance_uv: f64,
    limits: crate::trimmed_face_recipe::Limits,
) -> Result<ReplacedEndFace> {
    if owners[1].cyclic_index != 0 {
        return Err(Error::new(
            "BREP_OFFSET_END_OWNERSHIP",
            "Replacement end contact must start the adjacent contour",
        ));
    }
    let owner = owners[1].clone();
    let edge = qualify(
        context,
        patch,
        adjacent,
        adjacent_pcurve,
        start,
        shell,
        owners,
    )?;
    let contour_support = trim_sew::prove_curve_pcurve_correspondence(
        context,
        &edge.world_curve,
        adjacent,
        loop_contact,
        if owner.reversed {
            ParameterOrientation::Reversed
        } else {
            ParameterOrientation::Same
        },
        owner.clone(),
    )?;
    let contact = crate::trimmed_face_recipe::Boundary {
        curve: edge.world_curve.clone(),
        pcurve: loop_contact.clone(),
        reversed: owner.reversed,
    };
    let face = crate::trimmed_face_recipe::replace_boundary_arc(
        context,
        adjacent,
        original_wires,
        loop_index,
        arc_start,
        arc_count,
        &contact,
        tolerance_uv,
        limits,
    )?;
    Ok(ReplacedEndFace {
        edge,
        contour_support,
        face,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(z: f64) -> (Surface, Surface, Curve) {
        let w = vec![1., 0.5f64.sqrt(), 1.];
        let patch = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![2., 2., 3., 3.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: [0., 1.]
                .iter()
                .map(|&z| vec![vec![1., 0., z], vec![1., 1., z], vec![0., 1., z]])
                .collect(),
            weights: vec![w.clone(), w.clone()],
            periodic_u: false,
            periodic_v: false,
        };
        let mut adjacent = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., z], vec![0., 1., z]],
                vec![vec![1., 0., z], vec![1., 1., z]],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let mut uv = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: w,
            periodic: false,
        };
        if z == 1. {
            // Upper end chart has negative normal; its CCW UV loop maps to
            // the canonical opposite world traversal required by the patch.
            for row in &mut adjacent.control_points {
                for q in row {
                    q.swap(0, 1);
                }
            }
            for q in &mut uv.control_points {
                q.swap(0, 1);
            }
        }
        (patch, adjacent, uv)
    }
    fn owners(start: bool) -> [BoundaryUse; 2] {
        [
            BoundaryUse {
                face: 0,
                wire: 0,
                cyclic_index: if start { 3 } else { 1 },
                reversed: start,
            },
            BoundaryUse {
                face: 1,
                wire: 1,
                cyclic_index: 0,
                reversed: !start,
            },
        ]
    }
    #[test]
    fn both_canonical_ends_build_trimmed_support_faces_with_owned_reversal() {
        for start in [true, false] {
            let z = if start { 0. } else { 1. };
            let (patch, plane, uv) = fixture(z);
            let mut loop_contact = uv.clone();
            if !start {
                loop_contact.control_points.reverse();
                loop_contact.weights.reverse();
            }
            let points = [vec![1., 0.], vec![1., 1.], vec![0., 1.], vec![0., 0.]];
            let wires = vec![
                (0..4)
                    .map(|i| {
                        let pcurve = Curve::from_polyline(vec![
                            points[i].clone(),
                            points[(i + 1) % 4].clone(),
                        ])
                        .unwrap();
                        let mut curve = pcurve.clone();
                        for p in &mut curve.control_points {
                            if !start {
                                p.swap(0, 1);
                            }
                            p.push(z);
                        }
                        crate::trimmed_face_recipe::Boundary {
                            curve,
                            pcurve,
                            reversed: false,
                        }
                    })
                    .collect::<Vec<_>>(),
            ];
            let result = qualify_replaced_face(
                &ToleranceContext::default_valid(),
                &patch,
                &plane,
                &uv,
                &loop_contact,
                &wires,
                0,
                0,
                2,
                start,
                7,
                owners(start),
                1e-8,
                crate::trimmed_face_recipe::Limits {
                    pairs: 10000,
                    region_cells: 100000,
                    domain_cells: 100000,
                    agreement_cells: 100000,
                },
            )
            .unwrap();
            assert!(result.contour_support.permits_exact_correspondence());
            let replacement = result.face.replacement.unwrap();
            let face = replacement.face.expect(replacement.reason);
            assert_eq!(face.edges().len(), 3);
            assert_eq!(face.edges()[0].curve, result.edge.world_curve);
            assert_eq!(face.loops()[0].coedges[0].reversed, !start);
            assert_eq!(face.edges()[1].curve, wires[0][2].curve);
            assert_eq!(face.face().surface, plane);
        }
    }
    #[test]
    fn both_end_arcs_receive_original_patch_and_adjacent_plane_authority() {
        let context = ToleranceContext::default_valid();
        for start in [false, true] {
            let (patch, plane, uv) = fixture(if start { 0. } else { 1. });
            let result = qualify(&context, &patch, &plane, &uv, start, 7, owners(start)).unwrap();
            assert!(
                result
                    .supports
                    .iter()
                    .all(|p| p.permits_exact_correspondence())
            );
            assert_eq!(
                result.world_curve.control_points,
                patch.control_points[if start { 0 } else { 1 }]
            );
            assert_eq!(result.supports[1].pcurve, uv);
            assert_eq!(result.boundary.shell, 7);
            assert_eq!(result.boundary.uses, owners(start));
        }
    }
    #[test]
    fn wrong_support_parameter_or_owner_cannot_authorize_an_end() {
        let context = ToleranceContext::default_valid();
        let (patch, mut plane, mut uv) = fixture(1.);
        uv.control_points[1][0] += 0.1;
        assert!(qualify(&context, &patch, &plane, &uv, false, 7, owners(false)).is_err());
        let (_, _, uv) = fixture(1.);
        plane.weights[1][1] = 2.;
        assert!(qualify(&context, &patch, &plane, &uv, false, 7, owners(false)).is_err());
        let (_, plane, uv) = fixture(1.);
        let mut uses = owners(false);
        uses[1].reversed = false;
        assert!(qualify(&context, &patch, &plane, &uv, false, 7, uses).is_err());
        let mut uses = owners(false);
        uses[1].face = uses[0].face;
        assert!(qualify(&context, &patch, &plane, &uv, false, 7, uses).is_err());
    }
    #[test]
    fn wrong_patch_cyclic_address_cannot_authorize_the_canonical_end() {
        let context = ToleranceContext::default_valid();
        let (patch, plane, uv) = fixture(1.);
        let mut uses = owners(false);
        uses[0].cyclic_index = 3;
        assert!(qualify(&context, &patch, &plane, &uv, false, 7, uses).is_err());
    }
    #[test]
    fn shared_end_arc_and_complete_adjacent_corner_region_are_qualified_together() {
        let context = ToleranceContext::default_valid();
        let (patch, plane, uv) = fixture(1.);
        let points = [vec![1., 0.], vec![1., 1.], vec![0., 1.], vec![0., 0.]];
        let mut loop_contact = uv.clone();
        loop_contact.control_points.reverse();
        loop_contact.weights.reverse();
        let outer = (0..4)
            .map(|i| {
                Curve::from_polyline(vec![points[i].clone(), points[(i + 1) % 4].clone()]).unwrap()
            })
            .collect::<Vec<_>>();
        let before = outer.clone();
        assert!(
            qualify_clipped_end(
                &context,
                &patch,
                &plane,
                &uv,
                &uv,
                &[outer.clone()],
                0,
                0,
                2,
                false,
                7,
                owners(false),
                1e-8,
                nurbs_core::offset_face_loops::Limits {
                    pairs: 10000,
                    cells: 100000,
                    domain_cells: 100000
                }
            )
            .is_err()
        );

        let r = qualify_clipped_end(
            &context,
            &patch,
            &plane,
            &uv,
            &loop_contact,
            &[outer.clone()],
            0,
            0,
            2,
            false,
            7,
            owners(false),
            1e-8,
            nurbs_core::offset_face_loops::Limits {
                pairs: 10000,
                cells: 100000,
                domain_cells: 100000,
            },
        )
        .unwrap();
        assert!(r.contours.region_subset_proven, "{}", r.contours.reason);
        assert_eq!(
            r.contours.loops.as_ref().unwrap()[0],
            vec![loop_contact.clone(), outer[2].clone(), outer[3].clone()]
        );
        assert_eq!(
            r.contours.replacement.as_ref().unwrap().winding,
            vec![Some(1)]
        );
        assert_eq!(outer, before);
        assert!(r.contour_support.permits_exact_correspondence());
        assert_eq!(r.contour_support.pcurve, loop_contact);
        assert_eq!(r.edge.world_curve.control_points, patch.control_points[1]);
    }
}
