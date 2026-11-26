//! Algebraic world-space mass properties of a projected planar face.
use super::{
    exact_affine_hull::{PlanarSurfaceProjection, validate_projection_axes},
    planar_trim_projection::ProjectedTrimmedFace,
};
use crate::{
    Result,
    core::bounded_rational::Work,
    exact_geometry::exact_derivative::{
        ExactPlanarMass, ExactPlanarMassOutcome, ExactPlanarSecondMoments, Refusal,
        exact_planar_boundary_inertia,
    },
};
use num_rational::BigRational as Q;
use num_traits::{One, Zero};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactWorldPlaneMass {
    /// Multiply area and all integral moments by sqrt(area_scale_squared).
    pub area_scale_squared: Q,
    pub signed_area: Q,
    pub first_moments: [Q; 3],
    pub centroid: Option<[Q; 3]>,
    /// Signed integrals of coordinate products, about the world origin.
    pub second_moments: [[Q; 3]; 3],
    pub centroidal_second_moments: Option<[[Q; 3]; 3]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExactWorldPlaneMassOutcome {
    Mass(ExactWorldPlaneMass),
    BoundaryRefused(ExactPlanarMassOutcome),
    TransformRefused(Refusal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactWorldPlaneMassReport {
    pub outcome: ExactWorldPlaneMassOutcome,
    pub work_used: u64,
}

/// Transform exact signed boundary moments to XYZ. The rational moments
/// share the exact square-root factor of the plane metric. The centroid is
/// already in XYZ because that factor cancels. Supported trim spans are
/// polynomial or rational-linear. Winding supplies the material signs.
pub fn exact_projected_face_mass(
    face: &ProjectedTrimmedFace,
    max_work: u64,
) -> Result<ExactWorldPlaneMassReport> {
    validate_projection_axes(&face.projection)?;
    let report = exact_planar_boundary_inertia(&face.loops, max_work)?;
    let mut work = Work {
        used: report.work_used,
        limit: max_work,
    };
    let outcome = match (report.mass, report.second_moments) {
        (ExactPlanarMassOutcome::Mass(mass), Some(inertia)) => {
            transform(&face.projection, mass, inertia, &mut work)
                .map(ExactWorldPlaneMassOutcome::Mass)
                .unwrap_or_else(ExactWorldPlaneMassOutcome::TransformRefused)
        }
        (reason, _) => ExactWorldPlaneMassOutcome::BoundaryRefused(reason),
    };
    Ok(ExactWorldPlaneMassReport {
        outcome,
        work_used: work.used,
    })
}

type Exact<T> = std::result::Result<T, Refusal>;

fn transform(
    projection: &PlanarSurfaceProjection,
    mass: ExactPlanarMass,
    inertia: ExactPlanarSecondMoments,
    work: &mut Work,
) -> Exact<ExactWorldPlaneMass> {
    let [x, y] = projection.retained_axes;
    let z = projection.omitted_axis;
    let mut rows = std::array::from_fn(|_| std::array::from_fn(|_| Q::zero()));
    rows[x][1] = Q::one();
    rows[y][2] = Q::one();
    rows[z] = [
        projection.offset.clone(),
        projection.slopes[0].clone(),
        projection.slopes[1].clone(),
    ];
    let [a, b] = &projection.slopes;
    let a2 = work.op(a, a, |a, b| a * b)?;
    let b2 = work.op(b, b, |a, b| a * b)?;
    let area_scale_squared = work.op(&a2, &b2, |a, b| a + b + Q::one())?;

    let [mx, my] = &mass.first_moments;
    let area_vector = [&mass.signed_area, mx, my];
    let first_moments = [
        dot(&rows[0], area_vector, work)?,
        dot(&rows[1], area_vector, work)?,
        dot(&rows[2], area_vector, work)?,
    ];
    let centroid = match &mass.centroid {
        Some([cx, cy]) => {
            let one = Q::one();
            let center = [&one, cx, cy];
            Some([
                dot(&rows[0], center, work)?,
                dot(&rows[1], center, work)?,
                dot(&rows[2], center, work)?,
            ])
        }
        None => None,
    };
    let [xx, xy, yy] = &inertia.about_origin;
    let second_moments = transform_tensor(&rows, [area_vector, [mx, xx, xy], [my, xy, yy]], work)?;
    let centroidal_second_moments = match &inertia.about_centroid {
        Some([xx, xy, yy]) => {
            let zero = Q::zero();
            Some(transform_tensor(
                &rows,
                [[&zero; 3], [&zero, xx, xy], [&zero, xy, yy]],
                work,
            )?)
        }
        None => None,
    };
    Ok(ExactWorldPlaneMass {
        area_scale_squared,
        signed_area: mass.signed_area,
        first_moments,
        centroid,
        second_moments,
        centroidal_second_moments,
    })
}

fn transform_tensor(
    rows: &[[Q; 3]; 3],
    tensor: [[&Q; 3]; 3],
    work: &mut Work,
) -> Exact<[[Q; 3]; 3]> {
    let mut result = std::array::from_fn(|_| std::array::from_fn(|_| Q::zero()));
    for (i, row) in rows.iter().enumerate() {
        let projected = [
            dot(row, tensor[0], work)?,
            dot(row, tensor[1], work)?,
            dot(row, tensor[2], work)?,
        ];
        for j in i..3 {
            let value = dot(&rows[j], projected.each_ref(), work)?;
            result[i][j] = value.clone();
            result[j][i] = value;
        }
    }
    Ok(result)
}

fn dot(row: &[Q; 3], values: [&Q; 3], work: &mut Work) -> Exact<Q> {
    row.iter().zip(values).try_fold(Q::zero(), |sum, (a, b)| {
        let product = work.op(a, b, |a, b| a * b)?;
        Ok(work.op(&sum, &product, |a, b| a + b)?)
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPrincipalPlaneMoments {
    /// The signed principal pair in ascending order is
    /// (midpoint ± sqrt(half_gap_squared)) * sqrt(area_scale_squared).
    pub midpoint: Q,
    pub half_gap_squared: Q,
    pub area_scale_squared: Q,
}

impl ExactPrincipalPlaneMoments {
    /// True when the two physical in-plane moments are identical.
    pub fn is_isotropic(&self) -> bool {
        self.half_gap_squared.is_zero()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExactPrincipalPlaneOutcome {
    Moments(ExactPrincipalPlaneMoments),
    ZeroSignedArea,
    MassRefused(ExactWorldPlaneMassOutcome),
    SpectrumRefused(Refusal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPrincipalPlaneReport {
    pub outcome: ExactPrincipalPlaneOutcome,
    pub work_used: u64,
}

/// Derive the exact signed principal moments in the supporting plane.
/// One world tensor eigenvalue belongs to the plane normal and is zero.
/// The remaining pair has rational invariants and is retained in algebraic
/// form so close eigenvalues and distinct isotropy are resolved exactly.
pub fn exact_projected_face_principal_moments(
    face: &ProjectedTrimmedFace,
    max_work: u64,
) -> Result<ExactPrincipalPlaneReport> {
    let report = exact_projected_face_mass(face, max_work)?;
    let mut work = Work {
        used: report.work_used,
        limit: max_work,
    };
    let outcome = match report.outcome {
        ExactWorldPlaneMassOutcome::Mass(mass) => match mass.centroidal_second_moments {
            Some(tensor) => principal_pair(&tensor, mass.area_scale_squared, &mut work)
                .map(ExactPrincipalPlaneOutcome::Moments)
                .unwrap_or_else(ExactPrincipalPlaneOutcome::SpectrumRefused),
            None => ExactPrincipalPlaneOutcome::ZeroSignedArea,
        },
        refusal => ExactPrincipalPlaneOutcome::MassRefused(refusal),
    };
    Ok(ExactPrincipalPlaneReport {
        outcome,
        work_used: work.used,
    })
}

fn principal_pair(
    tensor: &[[Q; 3]; 3],
    area_scale_squared: Q,
    work: &mut Work,
) -> Exact<ExactPrincipalPlaneMoments> {
    use num_traits::Signed;
    let two = Q::from_integer(2.into());
    let trace = work.op(&tensor[0][0], &tensor[1][1], |a, b| a + b)?;
    let trace = work.op(&trace, &tensor[2][2], |a, b| a + b)?;
    let midpoint = work.op(&trace, &two, |a, b| a / b)?;
    // The second elementary symmetric polynomial is the product of the
    // two in-plane eigenvalues because the normal eigenvalue is zero.
    let mut pair_product = Q::zero();
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        let diagonal = work.op(&tensor[i][i], &tensor[j][j], |a, b| a * b)?;
        let off_diagonal = work.op(&tensor[i][j], &tensor[i][j], |a, b| a * b)?;
        let minor = work.op(&diagonal, &off_diagonal, |a, b| a - b)?;
        pair_product = work.op(&pair_product, &minor, |a, b| a + b)?;
    }
    let midpoint_squared = work.op(&midpoint, &midpoint, |a, b| a * b)?;
    let half_gap_squared = work.op(&midpoint_squared, &pair_product, |a, b| a - b)?;
    if half_gap_squared.is_negative() {
        return Err(Refusal::UnprovedIdentity);
    }
    Ok(ExactPrincipalPlaneMoments {
        midpoint,
        half_gap_squared,
        area_scale_squared,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPrincipalPlaneDirections {
    pub moments: ExactPrincipalPlaneMoments,
    /// Orthogonal projector onto the supporting plane.
    pub plane_projector: [[Q; 3]; 3],
    /// The signed axis projectors are
    /// (plane_projector ± deviator / sqrt(moments.half_gap_squared)) / 2.
    /// Minus corresponds to the smaller signed moment. Isotropy leaves this absent.
    pub deviator: Option<[[Q; 3]; 3]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExactPrincipalDirectionOutcome {
    Directions(ExactPrincipalPlaneDirections),
    ZeroSignedArea,
    MassRefused(ExactWorldPlaneMassOutcome),
    DirectionRefused(Refusal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPrincipalDirectionReport {
    pub outcome: ExactPrincipalDirectionOutcome,
    pub work_used: u64,
}

/// Represent the principal axes as exact algebraic orthogonal projectors.
/// Axis lines have no arbitrary vector sign. Equal moments leave the whole
/// supporting plane as the eigenspace. Signed zero area has no centroidal axes.
pub fn exact_projected_face_principal_directions(
    face: &ProjectedTrimmedFace,
    max_work: u64,
) -> Result<ExactPrincipalDirectionReport> {
    let report = exact_projected_face_mass(face, max_work)?;
    let mut work = Work {
        used: report.work_used,
        limit: max_work,
    };
    let outcome = match report.outcome {
        ExactWorldPlaneMassOutcome::Mass(mass) => match mass.centroidal_second_moments {
            Some(tensor) => principal_directions(
                &face.projection,
                &tensor,
                mass.area_scale_squared,
                &mut work,
            )
            .map(ExactPrincipalDirectionOutcome::Directions)
            .unwrap_or_else(ExactPrincipalDirectionOutcome::DirectionRefused),
            None => ExactPrincipalDirectionOutcome::ZeroSignedArea,
        },
        refusal => ExactPrincipalDirectionOutcome::MassRefused(refusal),
    };
    Ok(ExactPrincipalDirectionReport {
        outcome,
        work_used: work.used,
    })
}

fn principal_directions(
    projection: &PlanarSurfaceProjection,
    tensor: &[[Q; 3]; 3],
    area_scale_squared: Q,
    work: &mut Work,
) -> Exact<ExactPrincipalPlaneDirections> {
    let moments = principal_pair(tensor, area_scale_squared, work)?;
    let [x, y] = projection.retained_axes;
    let mut n = std::array::from_fn::<_, 3, _>(|_| Q::zero());
    n[x] = -&projection.slopes[0];
    n[y] = -&projection.slopes[1];
    n[projection.omitted_axis] = Q::one();
    let mut plane_projector = std::array::from_fn(|_| std::array::from_fn(|_| Q::zero()));
    let mut deviator = (!moments.is_isotropic()).then(|| plane_projector.clone());
    for i in 0..3 {
        for j in i..3 {
            let product = work.op(&n[i], &n[j], |a, b| a * b)?;
            let normal = work.op(&product, &moments.area_scale_squared, |a, b| a / b)?;
            let identity = Q::from_integer(usize::from(i == j).into());
            let plane = work.op(&identity, &normal, |a, b| a - b)?;
            if let Some(d) = deviator.as_mut() {
                let center = work.op(&moments.midpoint, &plane, |a, b| a * b)?;
                let remainder = work.op(&tensor[i][j], &center, |a, b| a - b)?;
                d[i][j] = remainder.clone();
                d[j][i] = remainder;
            }
            plane_projector[i][j] = plane.clone();
            plane_projector[j][i] = plane;
        }
    }
    Ok(ExactPrincipalPlaneDirections {
        moments,
        plane_projector,
        deviator,
    })
}
