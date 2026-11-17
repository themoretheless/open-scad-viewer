use crate::{Error, Result, closed_solid::QualifiedClosedSolid};
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};
use nurbs_core::analysis::exact_affine_hull::{SurfaceAffineHull, classify_surface_affine_hull};

#[derive(Clone, Debug)]
pub struct ExactPolyhedralVolume {
    /// Total material volume, subtracting inward cavity boundaries.
    pub volume_mm3: Q,
    pub exact_work: u64,
}

/// Integrate a qualified solid with planar faces and full piecewise-linear
/// edges. Exact oriented boundary flux counts holes and concavities without
/// triangulating the retained regions or integrating their unused supports.
pub fn exact_polyhedral_volume(
    solid: &QualifiedClosedSolid,
    max_exact_work: u64,
) -> Result<ExactPolyhedralVolume> {
    let (six_volume, exact_work) =
        integrate_polyhedral_boundary(solid, max_exact_work, |_, _, _, _, _| Ok(()))?;
    Ok(ExactPolyhedralVolume {
        volume_mm3: six_volume / Q::from_integer(6.into()),
        exact_work,
    })
}

#[derive(Clone, Debug)]
pub struct ExactPolyhedralCentroid {
    pub volume_mm3: Q,
    /// Center of volume in world coordinates, for uniform density.
    pub centroid_mm: [Q; 3],
    pub exact_work: u64,
}

/// Integrate first moments from the oriented polygonal boundary of a
/// qualified planar solid. Holes and inward cavity shells subtract material.
pub fn exact_polyhedral_centroid(
    solid: &QualifiedClosedSolid,
    max_exact_work: u64,
) -> Result<ExactPolyhedralCentroid> {
    let mut moments = std::array::from_fn(|_| Q::zero());
    let (six_volume, exact_work) =
        integrate_polyhedral_boundary(solid, max_exact_work, |a, b, c, signed, remaining| {
            spend(remaining, 12)?;
            for i in 0..3 {
                moments[i] += signed * (&a[i] + &b[i] + &c[i]);
            }
            Ok(())
        })?;
    // The fourth tetrahedron vertex is the world origin.
    let denominator = &six_volume * Q::from_integer(4.into());
    Ok(ExactPolyhedralCentroid {
        volume_mm3: six_volume / Q::from_integer(6.into()),
        centroid_mm: moments.map(|moment| moment / &denominator),
        exact_work,
    })
}

fn integrate_polyhedral_boundary(
    solid: &QualifiedClosedSolid,
    max_exact_work: u64,
    mut integrate: impl FnMut(&[Q; 3], &[Q; 3], &[Q; 3], &Q, &mut u64) -> Result<()>,
) -> Result<(Q, u64)> {
    if !(1..=1_000_000).contains(&max_exact_work) {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Exact volume work must be in 1..1000000",
        ));
    }
    let agreement = &solid.geometry().boundary.agreement;
    if !agreement.all_equal || !agreement.all_joins_exact {
        return Err(refuse(
            "Exact volume requires exact boundary identity and joins",
        ));
    }
    let model = solid.model();
    let mut remaining = max_exact_work;
    let mut six_volume = Q::zero();
    for body in &model.bodies {
        for shell in std::iter::once(&body.outer_shell).chain(&body.inner_shells) {
            for face_use in &model.shells[*shell].faces {
                let face = &model.faces[face_use.face];
                spend(&mut remaining, 1)?;
                if remaining == 0 {
                    return Err(work_exhausted());
                }
                let affine = classify_surface_affine_hull(&face.surface, remaining)?;
                remaining -= affine.work_used;
                match affine.hull {
                    SurfaceAffineHull::Plane(_) => {}
                    SurfaceAffineHull::ResourceLimit => return Err(work_exhausted()),
                    _ => return Err(refuse("Exact polyhedral volume requires planar supports")),
                }

                let outer = &model.loops[face.outer];
                let first = &outer.coedges[0];
                let origin = lift(&model.edges[first.edge].curve.control_points[0]);
                for wire in std::iter::once(&face.outer).chain(&face.holes) {
                    for coedge in &model.loops[*wire].coedges {
                        let edge = &model.edges[coedge.edge];
                        let curve = &edge.curve;
                        let knots = &curve.knots;
                        let n = curve.control_points.len();
                        if edge.degenerate
                            || coedge.trim().is_some()
                            || curve.degree != 1
                            || curve.periodic
                            || knots[0] != knots[1]
                            || knots[n] != knots[n + 1]
                        {
                            return Err(refuse(
                                "Exact volume requires full clamped degree-one edges",
                            ));
                        }

                        let reversed = coedge.reversed ^ face_use.reversed;
                        spend(&mut remaining, 3)?;
                        let mut previous = lift(&curve.control_points[0]);
                        for (i, pole) in curve.control_points.iter().enumerate().skip(1) {
                            spend(&mut remaining, 24)?;
                            if knots[i] >= knots[i + 1] {
                                return Err(refuse("Exact volume requires nonzero linear spans"));
                            }
                            let current = lift(pole);
                            let contribution = determinant(&origin, &previous, &current);
                            let signed = if reversed {
                                -contribution
                            } else {
                                contribution
                            };
                            integrate(&origin, &previous, &current, &signed, &mut remaining)?;
                            six_volume += signed;
                            previous = current;
                        }
                    }
                }
            }
        }
    }
    if !six_volume.is_positive() {
        return Err(refuse("Exact material volume must be positive"));
    }
    Ok((six_volume, max_exact_work - remaining))
}

fn lift(p: &[f64]) -> [Q; 3] {
    std::array::from_fn(|i| Q::from_float(p[i]).unwrap())
}

fn determinant(a: &[Q; 3], b: &[Q; 3], c: &[Q; 3]) -> Q {
    [(0, 1, 2), (1, 2, 0), (2, 0, 1)]
        .into_iter()
        .map(|(i, j, k)| &a[i] * (&b[j] * &c[k] - &b[k] * &c[j]))
        .sum()
}

fn spend(remaining: &mut u64, cost: u64) -> Result<()> {
    *remaining = remaining.checked_sub(cost).ok_or_else(work_exhausted)?;
    Ok(())
}

fn work_exhausted() -> Error {
    Error::new(
        "BREP_RESOURCE_LIMIT",
        "Exact material volume work exhausted",
    )
}

fn refuse(message: &str) -> Error {
    Error::new("BREP_CERTIFIED_MASS_REFUSED", message)
}
