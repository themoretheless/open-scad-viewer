//! Thin-wall detection: regions narrower than two nominal line widths cannot
//! hold an inner and an outer perimeter. A morphological opening (erode by one
//! line width, dilate back) removes them from the wall source; each removed
//! strip is printed as one centre-line bead whose per-segment width follows
//! the local wall thickness, clamped to the profile's allowed width range.

use planar_geometry::{
    path::BezierPath,
    rings::{self, Rings},
    stroke::{self, StrokeOptions},
};

use crate::{Budget, PathRole, Result, Toolpath, inset_rings, invalid, point_count};

/// Allowed extrusion width range for variable-width thin-wall beads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThinWallConfig {
    pub min_width_mm: f64,
    pub max_width_mm: f64,
}

impl ThinWallConfig {
    /// The range a printer profile accepts: 0.75..2 times the nozzle diameter.
    pub fn for_nozzle(nozzle_diameter_mm: f64) -> Self {
        Self {
            min_width_mm: nozzle_diameter_mm * 0.75,
            max_width_mm: nozzle_diameter_mm * 2.0,
        }
    }

    pub(crate) fn validate(&self, line_width_mm: f64) -> Result<()> {
        if !(self.min_width_mm.is_finite()
            && self.max_width_mm.is_finite()
            && self.min_width_mm > 0.0
            && self.min_width_mm <= self.max_width_mm
            && self.max_width_mm <= line_width_mm * 4.0)
        {
            return Err(invalid(
                "TOOLPATH_INVALID_SETTINGS",
                "Thin-wall width range must be positive, ordered and at most 4x the line width",
            ));
        }
        Ok(())
    }
}

/// One centre-line bead with one width per segment.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ThinWallPath {
    pub path: Toolpath,
    pub widths: Vec<f64>,
}

/// Union of `source` and a band of half-width `distance` around its boundary.
fn outset_rings(source: &Rings, distance: f64, budget: &mut Budget) -> Result<Rings> {
    if source.is_empty() {
        return Ok(Vec::new());
    }
    let tolerance = (distance * 0.01).min(0.01);
    let options = StrokeOptions {
        width: 2.0 * distance,
        ..Default::default()
    };
    let mut bands = Vec::with_capacity(source.len());
    for ring in source {
        budget.arrangement(ring.len().saturating_mul(6))?;
        let path = BezierPath::from_polyline(ring, true)?;
        for contour in stroke::outline_stroke_tol(&path, &options, tolerance)? {
            let band = contour.to_ring(tolerance)?;
            budget.points(band.len())?;
            bands.push(band);
        }
    }
    budget.arrangement(point_count(source).saturating_add(point_count(&bands)))?;
    let bands = rings::nonzero(&bands)?;
    let result = rings::planar(source, &bands, "union")?;
    budget.points(point_count(&result))?;
    Ok(result)
}

/// Split a normalized layer region into the part that keeps ordinary
/// perimeters and centre-line beads for the thin strips. Strips whose shape the
/// centre-line extractor cannot follow (branches, closed loops) stay in the
/// wall source and keep their ordinary perimeter.
pub(crate) fn split_thin_walls(
    source: &Rings,
    line_width_mm: f64,
    config: &ThinWallConfig,
    budget: &mut Budget,
) -> Result<(Rings, Vec<ThinWallPath>)> {
    if source.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let core = inset_rings(source, line_width_mm, budget)?;
    let opened = outset_rings(&core, line_width_mm, budget)?;
    let opened = if opened.is_empty() {
        opened
    } else {
        rings::planar(&opened, source, "intersection")?
    };
    let thin = if opened.is_empty() {
        source.clone()
    } else {
        rings::planar(source, &opened, "difference")?
    };
    let mut beads = Vec::new();
    let mut fallback: Rings = Vec::new();
    // Corner cusps and offset-tolerance slivers are far smaller than a bead.
    let min_area = line_width_mm * line_width_mm;
    let holes: Vec<&Vec<[f64; 2]>> = thin.iter().filter(|r| rings::area(r) < 0.0).collect();
    for ring in thin.iter().filter(|r| rings::area(r) >= min_area) {
        budget.points(ring.len())?;
        // A strip around a hole (a thin tube) is a loop: keep its perimeters.
        let inner: Vec<&Vec<[f64; 2]>> = holes
            .iter()
            .copied()
            .filter(|hole| ring_contains(ring, hole[0]))
            .collect();
        let bead = if inner.is_empty() {
            centre_line(ring, line_width_mm, config)
        } else {
            None
        };
        match bead {
            Some(bead) => beads.push(bead),
            None => {
                fallback.push(ring.clone());
                fallback.extend(inner.into_iter().cloned());
            }
        }
    }
    let walls = if fallback.is_empty() {
        opened
    } else if opened.is_empty() {
        rings::nonzero(&fallback)?
    } else {
        rings::planar(&opened, &rings::nonzero(&fallback)?, "union")?
    };
    Ok((walls, beads))
}

fn ring_contains(ring: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut inside = false;
    let n = ring.len();
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        if (a[1] > p[1]) != (b[1] > p[1]) {
            let x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
            if x > p[0] {
                inside = !inside;
            }
        }
    }
    inside
}

/// Centre line of a strip that is monotone along its principal axis: every
/// cross-section perpendicular to that axis meets the boundary exactly twice.
fn centre_line(
    ring: &[[f64; 2]],
    line_width_mm: f64,
    config: &ThinWallConfig,
) -> Option<ThinWallPath> {
    let n = ring.len() as f64;
    let c = ring
        .iter()
        .fold([0.0, 0.0], |acc, p| [acc[0] + p[0] / n, acc[1] + p[1] / n]);
    let (mut sxx, mut sxy, mut syy) = (0.0, 0.0, 0.0);
    for p in ring {
        let (dx, dy) = (p[0] - c[0], p[1] - c[1]);
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
    }
    let angle = 0.5 * (2.0 * sxy).atan2(sxx - syy);
    let u = [angle.cos(), angle.sin()];
    let v = [-u[1], u[0]];
    let proj = |p: &[f64; 2], axis: [f64; 2]| (p[0] - c[0]) * axis[0] + (p[1] - c[1]) * axis[1];
    let (smin, smax) = ring.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
        let s = proj(p, u);
        (lo.min(s), hi.max(s))
    });
    let length = smax - smin;
    if length < 2.0 * line_width_mm {
        return None;
    }
    let mean_width = rings::area(ring) / length;
    // Bead ends sit half a width inside the strip ends.
    let margin = (0.5 * mean_width).min(0.5 * length);
    let span = length - 2.0 * margin;
    let steps = ((span / (0.5 * line_width_mm)).ceil() as usize).clamp(1, 4096);
    let mut points = Vec::with_capacity(steps + 1);
    let mut local = Vec::with_capacity(steps + 1);
    for k in 0..=steps {
        let s = smin + margin + span * k as f64 / steps as f64;
        let mut hits = Vec::with_capacity(2);
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let (sa, sb) = (proj(&a, u), proj(&b, u));
            if (sa - s) * (sb - s) < 0.0 || (sa == s && sb != s) {
                let t = (s - sa) / (sb - sa);
                let q = [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
                hits.push(proj(&q, v));
            }
        }
        if hits.len() != 2 {
            return None;
        }
        let (lo, hi) = (hits[0].min(hits[1]), hits[0].max(hits[1]));
        let mid = 0.5 * (lo + hi);
        points.push([c[0] + s * u[0] + mid * v[0], c[1] + s * u[1] + mid * v[1]]);
        local.push(hi - lo);
    }
    let widths = local
        .windows(2)
        .map(|w| (0.5 * (w[0] + w[1])).clamp(config.min_width_mm, config.max_width_mm))
        .collect();
    Some(ThinWallPath {
        path: Toolpath {
            role: PathRole::ThinWall,
            points,
            closed: false,
        },
        widths,
    })
}
