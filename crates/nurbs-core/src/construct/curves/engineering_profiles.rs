//! Exact rational constructions in real arithmetic, with binary64 controls.
//! Closed profiles have clamped bases; no offset, sewing or rounding certificate.
use crate::{check, curve::Curve, surface::Surface, Result};

/// XY rounded rectangle centered at `center`, with full width/height and
/// four circular corners. Positive straight edges are required.
pub fn rounded_rectangle(center: [f64; 3], width: f64, height: f64, radius: f64) -> Result<Curve> {
    check(
        center.iter().all(|v| v.is_finite())
            && [width, height, radius].iter().all(|v| v.is_finite()),
        "Profile data must be finite",
    )?;
    check(
        radius > 0. && width > 2. * radius && height > 2. * radius,
        "Rounded rectangle requires positive radius and nonzero straight edges",
    )?;
    let x = width / 2.;
    let y = height / 2.;
    let r = radius;
    let ends = [
        [x - r, y],
        [-x + r, y],
        [-x, y - r],
        [-x, -y + r],
        [-x + r, -y],
        [x - r, -y],
        [x, -y + r],
        [x, y - r],
        [x - r, y],
    ];
    let middles = [
        [0., y],
        [-x, y],
        [-x, 0.],
        [-x, -y],
        [0., -y],
        [x, -y],
        [x, 0.],
        [x, y],
    ];
    profile(center, &ends, &middles)
}

/// XY stadium (capsule), with horizontal straight length and circular ends.
/// `straight` is the distance between the semicircle centers.
pub fn capsule(center: [f64; 3], straight: f64, radius: f64) -> Result<Curve> {
    check(
        center.iter().all(|v| v.is_finite())
            && straight.is_finite()
            && radius.is_finite()
            && straight > 0.
            && radius > 0.,
        "Capsule requires finite data, positive straight length and radius",
    )?;
    let h = straight / 2.;
    let r = radius;
    let ends = [
        [h, r],
        [-h, r],
        [-h - r, 0.],
        [-h, -r],
        [h, -r],
        [h + r, 0.],
        [h, r],
    ];
    let middles = [
        [0., r],
        [-h - r, r],
        [-h - r, -r],
        [0., -r],
        [h + r, -r],
        [h + r, r],
    ];
    // Capsule arc spans 1,2,4,5; rectangle arc spans 1,3,5,7.
    let arcs = [false, true, true, false, true, true];
    compose_profile(center, &ends, &middles, &arcs)
}
/// Closed rectangular cutter cross-section in XY. `mouth` is the center of
/// its top edge; depth extends toward -Y. No corner radii or standard fit.
pub fn keyway(mouth: [f64; 3], width: f64, depth: f64) -> Result<Curve> {
    positive_dimensions(mouth, &[width, depth])?;
    let w = width / 2.;
    closed_polygon(mouth, &[[-w, 0.], [-w, -depth], [w, -depth], [w, 0.]])
}
/// Closed T-slot cutter cross-section; depth extends toward -Y. The narrow
/// neck runs from the mouth to neck_depth, then the wider flange to total_depth.
pub fn t_slot(
    mouth: [f64; 3],
    neck_width: f64,
    flange_width: f64,
    neck_depth: f64,
    total_depth: f64,
) -> Result<Curve> {
    positive_dimensions(mouth, &[neck_width, flange_width, neck_depth, total_depth])?;
    check(
        flange_width > neck_width && total_depth > neck_depth,
        "T-slot flange must be wider than the neck and total depth must exceed neck depth",
    )?;
    let n = neck_width / 2.;
    let f = flange_width / 2.;
    closed_polygon(
        mouth,
        &[
            [-n, 0.],
            [-n, -neck_depth],
            [-f, -neck_depth],
            [-f, -total_depth],
            [f, -total_depth],
            [f, -neck_depth],
            [n, -neck_depth],
            [n, 0.],
        ],
    )
}
/// Closed dovetail cutter cross-section, wider at the bottom. Widths are
/// authored directly; slope angle follows from width difference and depth.
pub fn dovetail(mouth: [f64; 3], mouth_width: f64, base_width: f64, depth: f64) -> Result<Curve> {
    positive_dimensions(mouth, &[mouth_width, base_width, depth])?;
    check(
        base_width > mouth_width,
        "Dovetail base must be wider than the mouth",
    )?;
    let m = mouth_width / 2.;
    let b = base_width / 2.;
    closed_polygon(mouth, &[[-m, 0.], [-b, -depth], [b, -depth], [m, 0.]])
}
/// Closed XY cross-section of a belt with triangular V ribs pointing toward -Y.
/// `center` is the midpoint of the flat back at y=0, not the area centroid.
/// Width = ribs * pitch. Ribs meet at valleys below the back thickness.
/// This authored profile does not assert a belt standard or groove fit.
pub fn poly_v_belt(
    center: [f64; 3],
    ribs: usize,
    pitch: f64,
    back_thickness: f64,
    rib_height: f64,
) -> Result<Curve> {
    positive_dimensions(center, &[pitch, back_thickness, rib_height])?;
    check((1..=126).contains(&ribs), "V belt requires 1..126 ribs")?;
    let width = ribs as f64 * pitch;
    let left = -width / 2.;
    let valley = -back_thickness;
    let tip = valley - rib_height;
    check(
        width.is_finite() && center[1] > center[1] + valley && center[1] + valley > center[1] + tip,
        "V belt vertical levels collapsed at coordinate precision",
    )?;
    let mut points = Vec::with_capacity(2 * ribs + 3);
    points.push([left, 0.]);
    points.push([left, valley]);
    for rib in 0..ribs {
        let start = left + rib as f64 * pitch;
        let peak = left + (rib as f64 + 0.5) * pitch;
        let end = left + (rib + 1) as f64 * pitch;
        check(
            center[0] + start < center[0] + peak && center[0] + peak < center[0] + end,
            "V belt rib width collapsed at coordinate precision",
        )?;
        points.push([peak, tip]);
        points.push([end, valley]);
    }
    points.push([width / 2., 0.]);
    closed_polygon(center, &points)
}

/// Authored trapezoidal belt geometry, without standard-specific root radii.
#[derive(Clone, Copy, Debug)]
pub struct ToothedBelt {
    pub teeth: usize,
    pub pitch: f64,
    pub base_width: f64,
    pub tip_width: f64,
    pub back_thickness: f64,
    pub tooth_height: f64,
}

/// Closed CCW XY belt profile. `center` is the midpoint of the flat back.
/// Teeth point toward -Y. Requires pitch > base_width > tip_width > 0.
pub fn toothed_belt(center: [f64; 3], shape: ToothedBelt) -> Result<Curve> {
    let ToothedBelt {
        teeth,
        pitch,
        base_width,
        tip_width,
        back_thickness,
        tooth_height,
    } = shape;
    positive_dimensions(
        center,
        &[pitch, base_width, tip_width, back_thickness, tooth_height],
    )?;
    check(
        (1..=62).contains(&teeth),
        "Toothed belt requires 1..62 teeth",
    )?;
    check(
        pitch > base_width && base_width > tip_width,
        "Toothed belt requires pitch > base width > tip width",
    )?;
    let width = teeth as f64 * pitch;
    let left = -width / 2.;
    let valley = -back_thickness;
    let tip = valley - tooth_height;
    check(
        width.is_finite() && center[1] > center[1] + valley && center[1] + valley > center[1] + tip,
        "Toothed belt vertical levels collapsed at coordinate precision",
    )?;
    let mut points = Vec::with_capacity(4 * teeth + 4);
    points.push([left, 0.]);
    points.push([left, valley]);
    let mut previous = center[0] + left;
    for tooth in 0..teeth {
        let mid = left + (tooth as f64 + 0.5) * pitch;
        let xs = [
            mid - base_width / 2.,
            mid - tip_width / 2.,
            mid + tip_width / 2.,
            mid + base_width / 2.,
        ];
        for x in xs {
            check(
                center[0] + x > previous,
                "Toothed belt feature collapsed at coordinate precision",
            )?;
            previous = center[0] + x;
        }
        points.extend_from_slice(&[[xs[0], valley], [xs[1], tip], [xs[2], tip], [xs[3], valley]]);
    }
    check(
        center[0] + width / 2. > previous,
        "Toothed belt end gap collapsed at coordinate precision",
    )?;
    points.push([width / 2., valley]);
    points.push([width / 2., 0.]);
    closed_polygon(center, &points)
}

/// Closed cutter section for an authored O-ring groove, with sharp mouth
/// corners and circular bottom corners. No seal sizing or fit standard implied.
/// `mouth` is the midpoint of the top edge; depth extends toward -Y.
pub fn o_ring_groove(mouth: [f64; 3], width: f64, depth: f64, radius: f64) -> Result<Curve> {
    positive_dimensions(mouth, &[width, depth, radius])?;
    check(
        width > 2. * radius && depth > radius,
        "O-ring groove requires width > 2*radius and depth > radius",
    )?;
    let w = width / 2.;
    let xs = [-w, -w + radius, w - radius, w];
    let ys = [-depth, -depth + radius, 0.];
    check(
        xs.windows(2).all(|p| mouth[0] + p[0] < mouth[0] + p[1])
            && ys.windows(2).all(|p| mouth[1] + p[0] < mouth[1] + p[1]),
        "O-ring groove features collapsed at coordinate precision",
    )?;
    let ends = [
        [-w, 0.],
        [-w, -depth + radius],
        [-w + radius, -depth],
        [w - radius, -depth],
        [w, -depth + radius],
        [w, 0.],
        [-w, 0.],
    ];
    let middles = [
        [-w, (-depth + radius) / 2.],
        [-w, -depth],
        [0., -depth],
        [w, -depth],
        [w, (-depth + radius) / 2.],
        [0., 0.],
    ];
    compose_profile(
        mouth,
        &ends,
        &middles,
        &[false, true, false, true, false, false],
    )
}

/// Closed outline of an open annular retaining ring in XY. Gap is centered
/// on +X, in degrees. No plier lugs, holes, spring sizing or fit standard.
pub fn retaining_ring(center: [f64; 3], inner: f64, outer: f64, gap: f64) -> Result<Curve> {
    positive_dimensions(center, &[inner, outer, gap])?;
    check(
        outer > inner && gap < 360.,
        "Retaining ring requires inner < outer and 0 < gap < 360 degrees",
    )?;
    let normal = [0., 0., 1.];
    let exterior = crate::primitives::circle_arc(center, normal, outer, gap / 2., 360. - gap)?;
    let interior =
        crate::primitives::circle_arc(center, normal, inner, gap / 2., 360. - gap)?.reverse()?;
    let point = |c: &Curve, last: bool| -> [f64; 3] {
        let p = if last {
            c.control_points.last().unwrap()
        } else {
            &c.control_points[0]
        };
        [p[0], p[1], p[2]]
    };
    let a = point(&exterior, false);
    let b = point(&exterior, true);
    let c = point(&interior, false);
    let d = point(&interior, true);
    check(
        a != b && c != d && b != c && d != a,
        "Retaining ring gap or thickness collapsed at coordinate precision",
    )?;
    let radial_end = crate::primitives::line(b, c)?;
    let radial_start = crate::primitives::line(d, a)?;
    crate::paths::compose(&[exterior, radial_end, interior, radial_start])
}

/// Closed square fastener outline in XY. `across_flats` is the wrench size.
/// This is a geometric profile, without thread, chamfer or standard tolerance.
pub fn square_fastener(center: [f64; 3], across_flats: f64) -> Result<Curve> {
    positive_dimensions(center, &[across_flats])?;
    let h = across_flats / 2.;
    closed_polygon(center, &[[-h, -h], [h, -h], [h, h], [-h, h]])
}

/// Closed regular hexagonal fastener outline in XY, with horizontal flats.
/// `across_flats` is the distance between opposite parallel sides.
/// Binary64 construction rounds the irrational vertex coordinates.
pub fn hex_fastener(center: [f64; 3], across_flats: f64) -> Result<Curve> {
    positive_dimensions(center, &[across_flats])?;
    let h = across_flats / 2.;
    let r = across_flats / 3_f64.sqrt();
    closed_polygon(
        center,
        &[
            [r, 0.],
            [r / 2., h],
            [-r / 2., h],
            [-r, 0.],
            [-r / 2., -h],
            [r / 2., -h],
        ],
    )
}

fn positive_dimensions(origin: [f64; 3], dimensions: &[f64]) -> Result<()> {
    check(
        origin.iter().all(|x| x.is_finite()) && dimensions.iter().all(|x| x.is_finite() && *x > 0.),
        "Machining profile requires a finite origin and finite positive dimensions",
    )
}
fn closed_polygon(origin: [f64; 3], points: &[[f64; 2]]) -> Result<Curve> {
    let mut p = points
        .iter()
        .map(|q| [origin[0] + q[0], origin[1] + q[1], origin[2]])
        .collect::<Vec<_>>();
    p.push(p[0]);
    check(
        p.windows(2).all(|edge| edge[0] != edge[1]),
        "Machining profile edge collapsed at coordinate precision",
    )?;
    // Translation avoids catastrophic cancellation of absolute-coordinate areas.
    let area = p
        .windows(2)
        .map(|e| {
            let a = [e[0][0] - p[0][0], e[0][1] - p[0][1]];
            let b = [e[1][0] - p[0][0], e[1][1] - p[0][1]];
            a[0] * b[1] - a[1] * b[0]
        })
        .sum::<f64>();
    check(
        area.is_finite() && area > 0.,
        "Machining profile must have representable positive area",
    )?;
    crate::primitives::polyline(&p, true)
}

fn profile(center: [f64; 3], ends: &[[f64; 2]], middles: &[[f64; 2]]) -> Result<Curve> {
    compose_profile(
        center,
        ends,
        middles,
        &[false, true, false, true, false, true, false, true],
    )
}
fn compose_profile(
    center: [f64; 3],
    ends: &[[f64; 2]],
    middles: &[[f64; 2]],
    arcs: &[bool],
) -> Result<Curve> {
    let point = |p: [f64; 2]| vec![center[0] + p[0], center[1] + p[1], center[2]];
    let pieces = (0..middles.len())
        .map(|i| {
            crate::paths::bezier(
                vec![point(ends[i]), point(middles[i]), point(ends[i + 1])],
                Some(vec![
                    1.,
                    if arcs[i] {
                        std::f64::consts::FRAC_1_SQRT_2
                    } else {
                        1.
                    },
                    1.,
                ]),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    crate::paths::compose(&pieces)
}

/// Planar annular sector with rational angular U and linear radial V.
/// Angles are degrees. Domain [0,1]^2; full turn has a coincident G0 seam.
pub fn annular_sector(
    center: [f64; 3],
    normal: [f64; 3],
    inner: f64,
    outer: f64,
    start: f64,
    sweep: f64,
) -> Result<Surface> {
    check(
        inner.is_finite() && outer.is_finite() && inner > 0. && outer > inner,
        "Annular sector requires 0 < inner < outer",
    )?;
    let a = crate::primitives::circle_arc(center, normal, inner, start, sweep)?;
    let b = crate::primitives::circle_arc(center, normal, outer, start, sweep)?;
    crate::surface::loft(&[a, b])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rounded_rectangle_matches_lines_and_corner_circles() {
        let c = rounded_rectangle([3., -2., 5.], 10., 8., 1.).unwrap();
        assert_eq!(c.control_points.first(), c.control_points.last());
        for i in 0..8 {
            for j in 0..=100 {
                let p = c.evaluate((i as f64 + j as f64 / 100.) / 8.).unwrap().point;
                let x = p[0] - 3.;
                let y = p[1] + 2.;
                assert!((p[2] - 5.).abs() < 1e-12);
                let d = match i {
                    0 => (y - 4.).abs(),
                    2 => (x + 5.).abs(),
                    4 => (y + 4.).abs(),
                    6 => (x - 5.).abs(),
                    1 => ((x + 4.).hypot(y - 3.) - 1.).abs(),
                    3 => ((x + 4.).hypot(y + 3.) - 1.).abs(),
                    5 => ((x - 4.).hypot(y + 3.) - 1.).abs(),
                    _ => ((x - 4.).hypot(y - 3.) - 1.).abs(),
                };
                assert!(d < 1e-11, "span {i}: {d}");
            }
        }
    }
    #[test]
    fn capsule_matches_distance_to_center_segment() {
        let c = capsule([0.; 3], 6., 2.).unwrap();
        assert_eq!(c.control_points.first(), c.control_points.last());
        for j in 0..=1200 {
            let p = c.evaluate(j as f64 / 1200.).unwrap().point;
            let dx = p[0] - p[0].clamp(-3., 3.);
            assert!((dx.hypot(p[1]) - 2.).abs() < 1e-11);
        }
    }
    #[test]
    fn annular_sector_radial_law_and_plane() {
        let s = annular_sector([1., 2., 3.], [0., 0., 1.], 2., 5., 15., -270.).unwrap();
        for i in 0..=90 {
            for j in 0..=10 {
                let v = j as f64 / 10.;
                let p = s.evaluate(i as f64 / 90., v).unwrap().point;
                assert!(((p[0] - 1.).hypot(p[1] - 2.) - (2. + 3. * v)).abs() < 1e-11);
                assert!((p[2] - 3.).abs() < 1e-11);
            }
        }
        let mut angle = 0.;
        let mut previous = s.evaluate(0., 1.).unwrap().point;
        for i in 1..=900 {
            let p = s.evaluate(i as f64 / 900., 1.).unwrap().point;
            let a = [previous[0] - 1., previous[1] - 2.];
            let b = [p[0] - 1., p[1] - 2.];
            angle += (a[0] * b[1] - a[1] * b[0]).atan2(a[0] * b[0] + a[1] * b[1]);
            previous = p;
        }
        assert!((angle + 1.5 * std::f64::consts::PI).abs() < 1e-11);
        assert!(annular_sector([0.; 3], [0., 0., 1.], 2., 2., 0., 90.).is_err());
        assert!(annular_sector([0.; 3], [0.; 3], 1., 2., 0., 90.).is_err());
    }
    #[test]
    fn invalid_profiles_are_refused() {
        assert!(rounded_rectangle([0.; 3], 2., 4., 1.).is_err());
        assert!(rounded_rectangle([0.; 3], 10., 8., f64::NAN).is_err());
        assert!(capsule([0.; 3], 0., 1.).is_err());
        assert!(capsule([f64::INFINITY, 0., 0.], 1., 1.).is_err());
    }
}
