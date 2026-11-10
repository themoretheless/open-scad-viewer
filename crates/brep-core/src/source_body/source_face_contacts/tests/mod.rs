use super::*;
use crate::{
    source_contour_proposal::{qualify_linear_region, qualify_original_region},
    trimmed_face_recipe::{Boundary, Limits},
};
use nurbs_core::{curve::Curve, surface::Surface};
fn surface(vertical: bool) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..2)
            .map(|u| {
                (0..2)
                    .map(|v| {
                        if vertical {
                            vec![0.5, u as f64, v as f64 - 0.5]
                        } else {
                            vec![u as f64, v as f64, 0.]
                        }
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn wire(s: &Surface) -> Vec<Boundary> {
    let points = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    (0..4)
        .map(|i| {
            let a = points[i];
            let b = points[(i + 1) % 4];
            Boundary {
                curve: Curve::from_polyline(vec![
                    s.evaluate(a[0], a[1]).unwrap().point.to_vec(),
                    s.evaluate(b[0], b[1]).unwrap().point.to_vec(),
                ])
                .unwrap(),
                pcurve: Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap(),
                reversed: false,
            }
        })
        .collect()
}
#[test]
fn original_interior_contact_is_removed_only_after_root_valued_region_clipping() {
    let a = surface(false);
    let b = surface(true);
    let context = cad_predicates::ToleranceContext::default_valid();
    let limits = Limits {
        pairs: 1000,
        region_cells: 10000,
        domain_cells: 10000,
        agreement_cells: 10000,
    };
    let wa = wire(&a);
    let wb = wire(&b);
    let ra = qualify_original_region(&context, &a, &[wa.clone()], 1e-8, limits)
        .unwrap()
        .region
        .unwrap();
    let rb = qualify_original_region(&context, &b, &[wb], 1e-8, limits)
        .unwrap()
        .region
        .unwrap();
    let crossing = search([&ra, &rb], 1e-8, 1000, 100000).unwrap();
    assert!(crossing.contact.is_some());
    assert!(!crossing.absence_proven);
    let contact = Curve::from_polyline(vec![vec![0.75, 1.2], vec![0.75, -0.2]]).unwrap();
    let clipped = qualify_linear_region(
        &context,
        &a,
        &[wa],
        &contact,
        0,
        2,
        0,
        1e-8,
        limits,
        10000,
        [1e-8; 2],
        32,
        10000,
        1,
        10000,
        10000,
        10000,
    )
    .unwrap();
    assert!(
        clipped.region.is_some(),
        "{} / {}",
        clipped.reason,
        clipped.interior.reason
    );
    let clipped = clipped.region.unwrap();
    let excluded = search([&clipped, &rb], 1e-8, 10000, 1000000).unwrap();
    assert!(
        excluded.absence_proven,
        "cells {} domain {} unknown {}",
        excluded.cells,
        excluded.domain_cells,
        excluded.unresolved.len()
    );
    assert!(excluded.contact.is_none());
    let stopped = search([&clipped, &rb], 1e-8, 1, 1).unwrap();
    assert!(!stopped.absence_proven && stopped.contact.is_none());
    assert!(!stopped.unresolved.is_empty());
}
