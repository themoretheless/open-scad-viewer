//! Original-parameter patch ownership without re-fitting or reparameterization.
use super::*;
use crate::{
    curve::Curve,
    distance_bounds::{Interval as I, box_distance},
};

#[derive(Clone, Debug)]
pub struct BevelPatchCertificate {
    pub outer_profile: SphericalInvoluteCertificate,
    pub radial_control_difference_upper: f64,
    pub error_upper: f64,
    pub patches: usize,
}
/// Reconstruct the single original cubic chain from actual patch nets. Exact
/// common radial rows and original U intervals are required at every join.
/// No surface injectivity, G1 continuity or face sewing is asserted.
pub fn certify_straight_bevel_patches(
    patches: &[Surface],
    spec: SphericalInvoluteSpec,
    distances: [f64; 2],
) -> Result<BevelPatchCertificate> {
    check(
        !patches.is_empty()
            && patches.len() <= 32
            && distances.iter().all(|x| x.is_finite())
            && distances[0] > 0.
            && distances[0] < distances[1],
        "Bevel patches need 1..32 surfaces and finite ordered cone distances",
    )?;
    let ratio = I::point(distances[0]).div(I::point(distances[1]))?;
    let mut points = Vec::new();
    let mut breaks = Vec::new();
    let mut difference = 0_f64;
    for (index, surface) in patches.iter().enumerate() {
        surface.validate()?;
        let count = surface.control_points.len();
        check(
            surface.degree_u == 3
                && surface.degree_v == 1
                && !surface.periodic_u
                && !surface.periodic_v
                && surface.knots_v == [0., 0., 1., 1.]
                && count >= 4
                && (count - 1) % 3 == 0
                && surface.control_points.iter().all(|r| r.len() == 2)
                && surface.weights.iter().all(|r| r == &[1., 1.]),
            "Bevel patch must be an original unit-weight radial cubic Bernstein chain",
        )?;
        let n = (count - 1) / 3;
        let local: Vec<_> = (0..=n).map(|i| surface.knots_u[3 + 3 * i]).collect();
        check(
            local.windows(2).all(|w| w[0] < w[1]),
            "Bevel patch parameters must increase",
        )?;
        let mut expected = vec![local[0]; 4];
        for &u in &local[1..n] {
            expected.extend(std::iter::repeat_n(u, 3));
        }
        expected.extend(std::iter::repeat_n(local[n], 4));
        check(
            surface.knots_u == expected,
            "Bevel patch knot ownership differs from source chain",
        )?;
        if index > 0 {
            let previous = &patches[index - 1];
            check(
                breaks.last() == Some(&local[0])
                    && previous.control_points.last() == surface.control_points.first()
                    && previous.weights.last() == surface.weights.first(),
                "Bevel patches need exact adjoining original domains and common radial rows",
            )?;
        }
        breaks.extend(local.into_iter().skip(usize::from(index > 0)));
        points.extend(
            surface
                .control_points
                .iter()
                .skip(usize::from(index > 0))
                .map(|r| r[1].clone()),
        );
        for row in &surface.control_points {
            let ideal = [
                I::point(row[1][0]).mul(ratio)?,
                I::point(row[1][1]).mul(ratio)?,
                I::point(row[1][2]).mul(ratio)?,
            ];
            let saved = [
                I::point(row[0][0]),
                I::point(row[0][1]),
                I::point(row[0][2]),
            ];
            difference = difference.max(box_distance(&saved, &ideal)?.1);
        }
    }
    check(
        breaks.first() == Some(&0.) && breaks.last() == Some(&1.),
        "Bevel patch union must cover the complete original normalized source",
    )?;
    let mut knots = vec![0.; 4];
    for &u in &breaks[1..breaks.len() - 1] {
        knots.extend(std::iter::repeat_n(u, 3));
    }
    knots.extend(std::iter::repeat_n(1., 4));
    let curve = Curve {
        degree: 3,
        knots,
        weights: vec![1.; points.len()],
        control_points: points,
        periodic: false,
    };
    let outer = certify_spherical_involute(&curve, spec, distances[1])?;
    let error_upper = I::point(outer.error_upper).add(I::point(difference))?.hi;
    Ok(BevelPatchCertificate {
        outer_profile: outer,
        radial_control_difference_upper: difference,
        error_upper,
        patches: patches.len(),
    })
}

/// Owned whole working flank, partitioned only at original Bernstein boundaries.
pub struct QualifiedBevelPatches {
    patches: Vec<Surface>,
    source: SphericalInvoluteSpec,
    distances: [f64; 2],
    tolerance: f64,
    certificate: BevelPatchCertificate,
}
impl QualifiedBevelPatches {
    pub fn patches(&self) -> &[Surface] {
        &self.patches
    }
    pub fn source(&self) -> SphericalInvoluteSpec {
        self.source
    }
    pub fn cone_distances(&self) -> [f64; 2] {
        self.distances
    }
    pub fn certificate(&self) -> &BevelPatchCertificate {
        &self.certificate
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = certify_straight_bevel_patches(&self.patches, self.source, self.distances)?;
        check(
            fresh.error_upper <= self.tolerance
                && fresh.error_upper == self.certificate.error_upper,
            "Owned bevel patch error receipt changed",
        )
    }
}
/// High-accuracy whole working flank, up to 85 original cubic spans, with at
/// most 10 spans per surface to respect the 32-control surface limit. The global
/// U parameter remains unchanged; splitting introduces no re-fit error.
pub fn qualified_straight_bevel_patches(
    spec: SphericalInvoluteSpec,
    distances: [f64; 2],
    tolerance: f64,
    max_patches: usize,
) -> Result<QualifiedBevelPatches> {
    check(
        tolerance.is_finite()
            && tolerance > 0.
            && (1..=32).contains(&max_patches)
            && distances.iter().all(|x| x.is_finite())
            && distances[0] > 0.
            && distances[0] < distances[1],
        "Bevel patch construction needs positive tolerance, ordered radii and 1..32 patch budget",
    )?;
    let profile = approximate_spherical_involute(spec, distances[1], tolerance * 0.5)?;
    check(
        profile.spans.div_ceil(10) <= max_patches,
        "Bevel patch budget is insufficient",
    )?;
    let curve = &profile.curve;
    let ratio = distances[0] / distances[1];
    check(
        ratio > 0. && ratio < 1.,
        "Bevel radial scale is unrepresentable",
    )?;
    let mut patches = Vec::with_capacity(profile.spans.div_ceil(10));
    for first in (0..profile.spans).step_by(10) {
        let last = (first + 10).min(profile.spans);
        let mut knots = vec![curve.knots[3 + 3 * first]; 4];
        for i in first + 1..last {
            knots.extend(std::iter::repeat_n(curve.knots[3 + 3 * i], 3));
        }
        knots.extend(std::iter::repeat_n(curve.knots[3 + 3 * last], 4));
        let points: Vec<_> = curve.control_points[3 * first..=3 * last]
            .iter()
            .map(|p| vec![p.iter().map(|x| x * ratio).collect(), p.clone()])
            .collect();
        let surface = Surface {
            degree_u: 3,
            degree_v: 1,
            knots_u: knots,
            knots_v: vec![0., 0., 1., 1.],
            weights: vec![vec![1.; 2]; points.len()],
            control_points: points,
            periodic_u: false,
            periodic_v: false,
        };
        surface.validate()?;
        patches.push(surface);
    }
    let certificate = certify_straight_bevel_patches(&patches, spec, distances)?;
    check(
        certificate.error_upper <= tolerance,
        "Bevel patch world error exceeds requested tolerance",
    )?;
    Ok(QualifiedBevelPatches {
        patches,
        source: spec,
        distances,
        tolerance,
        certificate,
    })
}
