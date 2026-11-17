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
