//! Outward world-space centroid and second moments of rational planar trims.
use super::{
    exact_affine_hull::validate_projection_axes,
    planar_face_area::{plane_metric, rational_interval},
    planar_trim_projection::ProjectedTrimmedFace,
};
use crate::{
    Result, distance_bounds::Interval, trim::planar_mass_bounds::bound_planar_boundary_moments,
};

#[derive(Clone, Debug)]
pub struct BoundedWorldCentroid {
    pub point: [[f64; 2]; 3],
    /// Signed integrals of centered coordinate products in XYZ order.
    pub second_moments: [[[f64; 2]; 3]; 3],
}

#[derive(Clone, Debug)]
pub struct BoundedWorldPlaneMass {
    pub signed_area: [f64; 2],
    /// Available when the signed area interval excludes zero.
    pub centroidal: Option<BoundedWorldCentroid>,
    /// Whether the integration met the six requested chart moment widths.
    pub projected_converged: bool,
    pub cells: usize,
}

/// Bound signed physical area, XYZ centroid and centroidal second moments
/// of rational planar trim loops. The origin and three moment-width
/// tolerances are expressed in the projected chart. World-space intervals
/// enclose the propagated uncertainty, which is amplified near zero area.
pub fn bound_projected_face_mass(
    face: &ProjectedTrimmedFace,
    origin: [f64; 2],
    tolerances: [f64; 3],
    max_cells: usize,
) -> Result<BoundedWorldPlaneMass> {
    let projection = &face.projection;
    validate_projection_axes(projection)?;
    let (scale, slopes) = plane_metric(&projection.slopes)?;
    let integrated = bound_planar_boundary_moments(&face.loops, origin, tolerances, max_cells)?;
    let [area, mx, my, xx, xy, yy] = integrated.moments.map(|[lo, hi]| Interval { lo, hi });
    let physical = area.mul(scale)?;
    let centroidal = if area.lo > 0. || area.hi < 0. {
        let [i, j] = projection.retained_axes;
        let k = projection.omitted_axis;
        let dx = mx.div_signed(area)?;
        let dy = my.div_signed(area)?;
        let mut xyz = [Interval::point(0.); 3];
        xyz[i] = dx.add(Interval::point(origin[0]))?;
        xyz[j] = dy.add(Interval::point(origin[1]))?;
        xyz[k] = rational_interval(&projection.offset)?
            .add(slopes[0].mul(xyz[i])?)?
            .add(slopes[1].mul(xyz[j])?)?;

        let xx = xx.sub(mx.mul(dx)?)?;
        let xy = xy.sub(mx.mul(dy)?)?;
        let yy = yy.sub(my.mul(dy)?)?;
        let mut rows = [[Interval::point(0.); 2]; 3];
        rows[i][0] = Interval::point(1.);
        rows[j][1] = Interval::point(1.);
        rows[k] = slopes;
        Some(BoundedWorldCentroid {
            point: xyz.map(bounds),
            second_moments: transform_tensor(rows, [xx, xy, yy], scale)?,
        })
    } else {
        None
    };
    Ok(BoundedWorldPlaneMass {
        signed_area: bounds(physical),
        centroidal,
        projected_converged: integrated.converged,
        cells: integrated.cells,
    })
}

fn bounds(interval: Interval) -> [f64; 2] {
    [interval.lo, interval.hi]
}

fn transform_tensor(
    rows: [[Interval; 2]; 3],
    [xx, xy, yy]: [Interval; 3],
    scale: Interval,
) -> Result<[[[f64; 2]; 3]; 3]> {
    let mut out = [[[0.; 2]; 3]; 3];
    for (i, [a, b]) in rows.into_iter().enumerate() {
        let x = a.mul(xx)?.add(b.mul(xy)?)?;
        let y = a.mul(xy)?.add(b.mul(yy)?)?;
        for (j, [c, d]) in rows.into_iter().enumerate().skip(i) {
            let entry = bounds(x.mul(c)?.add(y.mul(d)?)?.mul(scale)?);
            out[i][j] = entry;
            out[j][i] = entry;
        }
    }
    Ok(out)
}

#[derive(Clone, Debug)]
pub struct BoundedPrincipalPlaneMoments {
    /// Two signed in-plane moments, in ascending order.
    pub principal: [[f64; 2]; 2],
    /// Sum of the two principal moments.
    pub polar: [f64; 2],
}

#[derive(Clone, Debug)]
pub struct BoundedPrincipalPlaneReport {
    pub signed_area: [f64; 2],
    pub spectrum: Option<BoundedPrincipalPlaneMoments>,
    pub projected_converged: bool,
    pub cells: usize,
}

/// Enclose the principal second area moments of a rationally trimmed planar
/// face. The third, normal eigenvalue of the central coordinate-product
/// tensor is zero. Each bound excludes numerical eigensolver error through
/// outward arithmetic on the two nontrivial characteristic coefficients.
/// The tolerances apply to the underlying projected moment integration.
pub fn bound_projected_face_principal_moments(
    face: &ProjectedTrimmedFace,
    origin: [f64; 2],
    tolerances: [f64; 3],
    max_cells: usize,
) -> Result<BoundedPrincipalPlaneReport> {
    let mass = bound_projected_face_mass(face, origin, tolerances, max_cells)?;
    let spectrum = mass
        .centroidal
        .map(|data| principal_spectrum(data.second_moments))
        .transpose()?;
    Ok(BoundedPrincipalPlaneReport {
        signed_area: mass.signed_area,
        spectrum,
        projected_converged: mass.projected_converged,
        cells: mass.cells,
    })
}

fn principal_spectrum(data: [[[f64; 2]; 3]; 3]) -> Result<BoundedPrincipalPlaneMoments> {
    let t = data.map(|row| row.map(|[lo, hi]| Interval { lo, hi }));
    let trace = t[0][0].add(t[1][1])?.add(t[2][2])?;
    let mut product = Interval::point(0.);
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        let minor = t[i][i].mul(t[j][j])?.sub(square(t[i][j])?)?;
        product = product.add(minor)?;
    }
    let center = trace.mul(Interval::point(0.5))?;
    let gap_squared = square(center)?.sub(product)?;
    // A real symmetric tensor has real roots, so the true discriminant
    // is nonnegative even when the interval arithmetic extends below zero.
    let gap = Interval::new(
        gap_squared.lo.max(0.).sqrt().next_down().max(0.),
        gap_squared.hi.sqrt().next_up(),
    )?;
    Ok(BoundedPrincipalPlaneMoments {
        principal: [bounds(center.sub(gap)?), bounds(center.add(gap)?)],
        polar: bounds(trace),
    })
}

fn square(value: Interval) -> Result<Interval> {
    let result = value.mul(value)?;
    Interval::new(result.lo.max(0.), result.hi)
}
