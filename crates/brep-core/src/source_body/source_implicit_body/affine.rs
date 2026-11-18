use super::{Attempt, Error, Limits, Model, Report, Result, verify_sphere};
use num_rational::BigRational;
use num_traits::{Signed, ToPrimitive, Zero};

/// The exact source solid is the image of the radius-`radius` ball under
/// `x ↦ center + linear * x`. Matrix entries are stored by row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AffineSphere {
    pub center: [f64; 3],
    pub linear: [[f64; 3]; 3],
    pub radius: f64,
}

pub struct AffineSphereBody {
    model: Model,
    source: AffineSphere,
    mapped_points: usize,
    normalized: Report,
}

impl AffineSphereBody {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn source(&self) -> AffineSphere {
        self.source
    }
    pub fn mapped_points(&self) -> usize {
        self.mapped_points
    }
    /// Proof in normalized sphere coordinates. Metric quantities in this
    /// report are in those coordinates. It also records the exact seam proof.
    pub fn normalized_proof(&self) -> &Report {
        &self.normalized
    }
}

pub enum AffineSphereAttempt {
    Certified(Box<AffineSphereBody>),
    Refused {
        reason: &'static str,
        mapped_points: usize,
        normalized: Option<Box<Report>>,
    },
}

fn cross(a: &[BigRational; 3], b: &[BigRational; 3]) -> [BigRational; 3] {
    std::array::from_fn(|i| {
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        &a[j] * &b[k] - &a[k] * &b[j]
    })
}

fn dot(a: &[BigRational; 3], b: &[BigRational; 3]) -> BigRational {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

fn exact_vector(point: [f64; 3]) -> Option<[BigRational; 3]> {
    let [x, y, z] = point.map(BigRational::from_float);
    Some([x?, y?, z?])
}

fn exact_float(value: &BigRational) -> Option<f64> {
    let proposal = value.to_f64()?;
    (BigRational::from_float(proposal).as_ref() == Some(value)).then_some(proposal)
}

/// Certifies the original, unmodified shell through one exact affine change
/// of coordinates. Whole-shell embedding, pole quotients, topology, outward
/// orientation and geometric G1/G2 transfer through a positive-determinant
/// affine diffeomorphism. The face trims, knots, weights and topology are
/// identical in both coordinate systems. Each Cartesian vertex or control
/// point consumes one unit of `max_points`. Matrix work has constant size.
/// For a reflected source frame, reflect the first normalized coordinate.
/// This sphere symmetry makes the proof map orientation-preserving.
/// Only exactly representable inverse control points enter the sphere proof.
pub fn verify_affine_sphere(
    model: &Model,
    source: AffineSphere,
    tolerance_uv: f64,
    max_points: usize,
    limits: Limits,
) -> Result<AffineSphereAttempt> {
    let bad_source = || {
        Error::new(
            "BREP_IMPLICIT_SOURCE_INVALID",
            "Invalid affine sphere or point budget",
        )
    };
    let center = exact_vector(source.center).ok_or_else(bad_source)?;
    let [a, b, c] = source.linear.map(exact_vector);
    let (a, b, c) = (
        a.ok_or_else(bad_source)?,
        b.ok_or_else(bad_source)?,
        c.ok_or_else(bad_source)?,
    );
    if !source.radius.is_finite()
        || source.radius <= 0.
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || max_points > 1_000_000
    {
        return Err(bad_source());
    }
    model.validate()?;

    let cofactors = [cross(&b, &c), cross(&c, &a), cross(&a, &b)];
    let determinant = dot(&a, &cofactors[0]);
    let refused = |reason, mapped_points| AffineSphereAttempt::Refused {
        reason,
        mapped_points,
        normalized: None,
    };
    if determinant.is_zero() {
        return Ok(refused("original-affine-sphere-singular-map", 0));
    }
    let reflected = determinant.is_negative();
    let inverse: [[BigRational; 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let coefficient = &cofactors[j][i] / &determinant;
            if reflected && i == 0 {
                -coefficient
            } else {
                coefficient
            }
        })
    });
    let mut normalized = model.clone();
    let points = normalized
        .0
        .vertices
        .iter_mut()
        .map(|v| v.point.as_mut_slice())
        .chain(
            normalized
                .0
                .edges
                .iter_mut()
                .flat_map(|edge| edge.curve.control_points.iter_mut().map(Vec::as_mut_slice)),
        )
        .chain(normalized.0.faces.iter_mut().flat_map(|face| {
            face.surface
                .control_points
                .iter_mut()
                .flatten()
                .map(Vec::as_mut_slice)
        }));
    let mut mapped_points = 0;
    for point in points {
        if mapped_points == max_points {
            return Ok(refused(
                "original-affine-sphere-point-budget-exhausted",
                mapped_points,
            ));
        }
        mapped_points += 1;
        let position = <[f64; 3]>::try_from(&*point).ok().and_then(exact_vector);
        let Some(position) = position else {
            return Err(bad_source());
        };
        let delta = std::array::from_fn(|i| &position[i] - &center[i]);
        let [x, y, z] = inverse.each_ref().map(|row| exact_float(&dot(row, &delta)));
        let (Some(x), Some(y), Some(z)) = (x, y, z) else {
            return Ok(refused(
                "original-affine-sphere-exact-inverse-unproved",
                mapped_points,
            ));
        };
        point.copy_from_slice(&[x, y, z]);
    }
    match verify_sphere(&normalized, [0.; 3], source.radius, tolerance_uv, limits)? {
        Attempt::Certified(body) => {
            Ok(AffineSphereAttempt::Certified(Box::new(AffineSphereBody {
                model: model.clone(),
                source,
                mapped_points,
                normalized: body.report,
            })))
        }
        Attempt::Refused(report) => Ok(AffineSphereAttempt::Refused {
            reason: "original-affine-sphere-normalized-proof-refused",
            mapped_points,
            normalized: Some(report),
        }),
    }
}
