//! Complete periodic side network. Planar caps and solid admission belong to B-rep.
use super::*;
use crate::curve::Curve;
use crate::surface::Surface;

const JOINS: [(usize, bool, usize, bool); 5] = [
    (0, true, 1, false),
    (1, true, 2, false),
    (2, true, 3, true),
    (3, false, 4, true),
    (4, false, 5, false),
];

/// Six families per period: descending root/working, tip, ascending
/// working/root, crest cylinder. Ascending families run opposite to the contour.
/// Every exposed grid retains its original analytic source and actual controls.
pub struct AxialSideNetwork {
    grids: Vec<QualifiedAxialGrid>,
    teeth: usize,
}
impl AxialSideNetwork {
    pub fn grids(&self) -> &[QualifiedAxialGrid] {
        &self.grids
    }
    pub fn teeth(&self) -> usize {
        self.teeth
    }
    /// Exact actual-pole end contours, clockwise in XZ, at the two physical
    /// axial planes. No evaluation or refitting occurs. These curves are cap
    /// proposals; planar trim-region admission remains a separate operation.
    pub fn end_contours(&self) -> [Vec<Curve>; 2] {
        std::array::from_fn(|end| {
            let mut result = Vec::with_capacity(self.grids.len());
            for (i, grid) in self.grids.iter().enumerate() {
                let nv = (grid.y_breaks().len() - 1).div_ceil(6);
                let u_count = grid.packed_surfaces().len() / nv;
                let mut family = Vec::with_capacity(u_count);
                for iu in 0..u_count {
                    let s = &grid.packed_surfaces()[iu * nv + if end == 0 { 0 } else { nv - 1 }];
                    let j = if end == 0 {
                        0
                    } else {
                        s.control_points[0].len() - 1
                    };
                    family.push(Curve {
                        degree: s.degree_u,
                        knots: s.knots_u.clone(),
                        control_points: s.control_points.iter().map(|r| r[j].clone()).collect(),
                        weights: s.weights.iter().map(|r| r[j]).collect(),
                        periodic: false,
                    });
                }
                if i % 6 == 3 || i % 6 == 4 {
                    family.reverse();
                    for c in &mut family {
                        c.control_points.reverse();
                        c.weights.reverse();
                        c.knots = c.knots.iter().rev().map(|x| -x).collect();
                    }
                }
                result.extend(family);
            }
            result
        })
    }
    pub fn reverify(&self) -> Result<()> {
        check(
            self.grids.len() == self.teeth * 6,
            "Incomplete wheel side census",
        )?;
        for (i, grid) in self.grids.iter().enumerate() {
            check(
                grid.instance().period() == i / 6,
                "Incorrect wheel period ownership",
            )?;
            grid.reverify()?;
        }
        self.verify_stitches()
    }
    fn verify_stitches(&self) -> Result<()> {
        let nv = self.grids[0].y_breaks().len() - 1;
        for j in 0..self.teeth {
            for (l, le, r, re) in JOINS {
                same_boundary(
                    self.grids[j * 6 + l].surfaces(),
                    le,
                    self.grids[j * 6 + r].surfaces(),
                    re,
                    nv,
                )?;
            }
            same_boundary(
                self.grids[j * 6 + 5].surfaces(),
                true,
                self.grids[((j + 1) % self.teeth) * 6].surfaces(),
                false,
                nv,
            )?;
        }
        Ok(())
    }
}
fn row(s: &[Surface], end: bool, iv: usize, nv: usize) -> &Vec<Vec<f64>> {
    let iu = if end { s.len() / nv - 1 } else { 0 };
    &s[iu * nv + iv].control_points[if end { 5 } else { 0 }]
}
fn same_boundary(a: &[Surface], ae: bool, b: &[Surface], be: bool, nv: usize) -> Result<()> {
    for iv in 0..nv {
        check(
            row(a, ae, iv, nv)
                .iter()
                .flatten()
                .map(|x| x.to_bits())
                .eq(row(b, be, iv, nv).iter().flatten().map(|x| x.to_bits())),
            "Wheel side boundary controls disagree",
        )?;
    }
    Ok(())
}

/// Builds all tooth periods on an explicit common physical-Y grid. Breaks are
/// caller-selected resolution, never permission to omit part of a source.
/// Stitching occurs in world coordinates, followed by full source admission.
/// This certifies G0 sides and approximation error, not caps or a global solid.
pub fn axial_side_network(
    spec: Spec,
    y: Vec<f64>,
    stock_radius: f64,
    t: [Vec<f64>; 6],
    tolerance: f64,
    max_patches_per_family: usize,
) -> Result<AxialSideNetwork> {
    check(y.len() >= 2 && y.len() <= 4097, "Invalid wheel axial grid")?;
    check(
        (12..=256).contains(&spec.teeth) && tolerance.is_finite() && tolerance > 0.,
        "Invalid wheel count or tolerance",
    )?;
    check(
        y.iter().all(|x| x.is_finite()) && y.windows(2).all(|w| w[0] < w[1]),
        "Invalid common physical-Y breaks",
    )?;
    let mut cells = 0usize;
    for breaks in &t {
        check(
            breaks.len() >= 2
                && breaks.len() <= 4097
                && breaks[0] == 0.
                && breaks[breaks.len() - 1] == 1.
                && breaks.iter().all(|x| x.is_finite())
                && breaks.windows(2).all(|w| w[0] < w[1]),
            "Incomplete wheel family breaks",
        )?;
        let n = (breaks.len() - 1) * (y.len() - 1);
        check(
            n <= max_patches_per_family && max_patches_per_family <= 4096,
            "Wheel family patch budget exceeded",
        )?;
        cells += n;
    }
    check(
        cells * spec.teeth <= 16384,
        "Wheel network exceeds 16384 patches",
    )?;
    let domain = [y[0], y[y.len() - 1]];
    let d = axial_stock_intersection(spec, Flank::Descending, domain, stock_radius, 128)?;
    let a = axial_stock_intersection(spec, Flank::Ascending, domain, stock_radius, 128)?;
    let tip = axial_tip_chart(&d, &a, 32)?;
    let sources = [
        AxialSource::Root(axial_root_chart(
            spec,
            RootPart::CrestEdge(Flank::Descending),
            domain,
        )?),
        AxialSource::Working(d),
        AxialSource::Tip(tip),
        AxialSource::Working(a),
        AxialSource::Root(axial_root_chart(
            spec,
            RootPart::CrestEdge(Flank::Ascending),
            domain,
        )?),
        AxialSource::Root(axial_root_chart(spec, RootPart::CrestCylinder, domain)?),
    ];
    let mut templates = Vec::with_capacity(6);
    for i in 0..6 {
        templates.push(regrid_axial_source(
            &sources[i],
            t[i].clone(),
            y.clone(),
            tolerance,
            max_patches_per_family,
        )?);
    }
    let nv = y.len() - 1;
    let mut placed = Vec::with_capacity(spec.teeth * 6);
    for j in 0..spec.teeth {
        for i in 0..6 {
            placed.push(place_axial_surfaces(
                &axial_instance(&sources[i], j)?,
                templates[i].surfaces(),
            )?);
        }
    }
    for j in 0..spec.teeth {
        let joins = [
            (j * 6 + JOINS[0].0, JOINS[0].1, j * 6 + JOINS[0].2, JOINS[0].3),
            (j * 6 + JOINS[1].0, JOINS[1].1, j * 6 + JOINS[1].2, JOINS[1].3),
            (j * 6 + JOINS[2].0, JOINS[2].1, j * 6 + JOINS[2].2, JOINS[2].3),
            (j * 6 + JOINS[3].0, JOINS[3].1, j * 6 + JOINS[3].2, JOINS[3].3),
            (j * 6 + JOINS[4].0, JOINS[4].1, j * 6 + JOINS[4].2, JOINS[4].3),
            (j * 6 + 5, true, ((j + 1) % spec.teeth) * 6, false),
        ];
        for (l, le, r, re) in joins {
            for iv in 0..nv {
                let controls = row(&placed[l], le, iv, nv).clone();
                let iu = if re { placed[r].len() / nv - 1 } else { 0 };
                placed[r][iu * nv + iv].control_points[if re { 5 } else { 0 }] = controls;
            }
        }
    }
    let mut grids = Vec::with_capacity(placed.len());
    for (i, surfaces) in placed.into_iter().enumerate() {
        grids.push(qualify_axial_instance(
            &axial_instance(&sources[i % 6], i / 6)?,
            t[i % 6].clone(),
            y.clone(),
            surfaces,
            tolerance,
            max_patches_per_family,
        )?);
    }
    let network = AxialSideNetwork {
        grids,
        teeth: spec.teeth,
    };
    // Each actual world grid was just freshly admitted above. Check the
    // completed interface ledger here; explicit replay still rechecks sources.
    network.verify_stitches()?;
    Ok(network)
}
