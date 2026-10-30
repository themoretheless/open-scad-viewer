//! Exact zero-row factorization, flat G2 limits and a local pole quotient.
use super::*;
use crate::check;
#[path = "flat_pole/bounds.rs"]
mod bounds;
#[path = "flat_pole/fan.rs"]
mod fan;
#[path = "flat_pole/prepared.rs"]
mod prepared;
pub struct FlatPoleReport {
    pub proven: bool,
    pub differential_proven: bool,
    pub quotient_proven: bool,
    pub reason: &'static str,
    pub cells: usize,
    pub pending: usize,
    pub normal_coefficient: Option<f64>,
    pub tensor_coefficient: Option<f64>,
    pub normal_axis: usize,
    pub normal_sign: Option<i8>,
    pub pole: [f64; 3],
    pub original_u_span: [f64; 2],
    pub band_normalized: f64,
    pub fan_angle_upper: Option<f64>,
    pub reference_radius: Option<f64>,
    pub quotient_lower: Option<f64>,
}
pub struct FlatPoleCertificate<'a> {
    source: &'a Surface,
    end: bool,
    axis: usize,
    band: f64,
    max_cells: usize,
    normal: f64,
    tensor: f64,
    sign: i8,
    pole: [f64; 3],
    cells: usize,
    angle: f64,
    radius: f64,
    quotient_lower: f64,
}
pub struct FlatPoleInspection<'a> {
    pub report: FlatPoleReport,
    pub certificate: Option<FlatPoleCertificate<'a>>,
}
impl FlatPoleCertificate<'_> {
    pub fn is_end(&self) -> bool {
        self.end
    }
    pub fn source(&self) -> &Surface {
        self.source
    }
    pub fn pole(&self) -> [f64; 3] {
        self.pole
    }
    pub fn normal(&self) -> [f64; 3] {
        let mut n = [0.; 3];
        n[self.axis] = self.sign as f64;
        n
    }
    /// Whole normalized pole band: normal error <= u² times this coefficient.
    pub fn normal_coefficient(&self) -> f64 {
        self.normal
    }
    /// Whole normalized pole band: tensor norm <= u times this coefficient.
    pub fn tensor_coefficient(&self) -> f64 {
        self.tensor
    }
    pub fn band_normalized(&self) -> f64 {
        self.band
    }
    /// Radius in the leading conic reference plane, not the whole surface.
    pub fn reference_radius(&self) -> f64 {
        self.radius
    }
    pub fn quotient_lower(&self) -> f64 {
        self.quotient_lower
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = inspect_original_flat_pole(
            self.source,
            self.end,
            self.axis,
            self.band,
            self.max_cells,
        )?;
        let c = fresh
            .certificate
            .ok_or_else(|| crate::numeric_err("Flat pole proof no longer holds"))?;
        check(
            c.normal == self.normal
                && c.tensor == self.tensor
                && c.sign == self.sign
                && c.pole == self.pole
                && c.cells == self.cells
                && c.angle == self.angle
                && c.radius == self.radius
                && c.quotient_lower == self.quotient_lower,
            "Flat pole source/proof changed",
        )
    }
}
struct Cell {
    v: [f64; 2],
    bound: Option<bounds::Bounds>,
}
struct Summary {
    normal: f64,
    tensor: f64,
    sign: i8,
    angle: f64,
    radial: f64,
    remainder: f64,
}
fn summary(cells: &[Cell]) -> Result<Option<Summary>> {
    if cells.iter().any(|c| c.bound.is_none()) {
        return Ok(None);
    }
    let sign = cells[0].bound.as_ref().unwrap().sign;
    if !cells.iter().all(|c| c.bound.as_ref().unwrap().sign == sign) {
        return Ok(None);
    }
    let mut s = Summary {
        normal: 0.,
        tensor: 0.,
        sign,
        angle: 0.,
        radial: f64::INFINITY,
        remainder: 0.,
    };
    let mut angle = I::point(0.);
    for c in cells {
        let b = c.bound.as_ref().unwrap();
        s.normal = s.normal.max(b.normal);
        s.tensor = s.tensor.max(b.tensor);
        angle = angle.add(I::point(b.fan.angle))?;
        s.radial = s.radial.min(b.fan.radial);
        s.remainder = s.remainder.max(b.fan.remainder);
    }
    s.angle = angle.hi;
    Ok(Some(s))
}
/// Prove flat G2 limits and a local boundary-pole quotient from literal
/// original zeros. The leading fan must have signed angular speed and total
/// angle <=4 radians. Its sector is 2-quasiconvex; a perturbation with derivative
/// norm <1/4 is injective on a positive-radius reference neighborhood.
/// This does NOT prove whole-surface embedding, surrounding seams or volume.
pub fn inspect_original_flat_pole(
    s: &Surface,
    end: bool,
    height_axis: usize,
    band: f64,
    max_cells: usize,
) -> Result<FlatPoleInspection<'_>> {
    s.validate()?;
    check(
        height_axis < 3 && band.is_finite() && band > 0. && band <= 1. && max_cells <= 100000,
        "Invalid original flat pole query",
    )?;
    let row = if end { s.control_points.len() - 1 } else { 0 };
    let pole = std::array::from_fn(|k| s.control_points[row][0][k]);
    let empty =
        |reason, cells, pending, data: Option<&Summary>, span: [f64; 2]| FlatPoleInspection {
            report: FlatPoleReport {
                proven: false,
                differential_proven: data.is_some(),
                quotient_proven: false,
                reason,
                cells,
                pending,
                normal_coefficient: data.map(|s| s.normal),
                tensor_coefficient: data.map(|s| s.tensor),
                normal_axis: height_axis,
                normal_sign: data.map(|s| if end { -s.sign } else { s.sign }),
                pole,
                original_u_span: span,
                band_normalized: band,
                fan_angle_upper: data.map(|s| s.angle),
                reference_radius: None,
                quotient_lower: None,
            },
            certificate: None,
        };
    let Some(prepared) = prepared::Prepared::new(s, end, height_axis)? else {
        return Ok(empty(
            "literal-flat-pole-factors-unproven",
            0,
            1,
            None,
            [0.; 2],
        ));
    };
    let mut work = 0;
    let first = if work < max_cells {
        work += 1;
        bounds::calculate(&prepared, band, [0., 1.])?
    } else {
        None
    };
    let mut cells = vec![Cell {
        v: [0., 1.],
        bound: first,
    }];
    loop {
        let sum = summary(&cells)?;
        if let Some(data) = &sum {
            if data.angle <= 4. {
                let delta = if data.remainder == 0. {
                    band
                } else {
                    band.min(I::point(0.125).div(I::point(data.remainder))?.lo)
                };
                let radius = I::point(delta.max(0.)).mul(I::point(data.radial))?.lo;
                let eta = I::point(delta.max(0.)).mul(I::point(data.remainder))?.hi;
                let lower = I::point(1.).sub(I::point(2.).mul(I::point(eta))?)?.lo;
                if delta > 0. && radius > 0. && eta < 0.25 && lower > 0. {
                    let sign = if end { -data.sign } else { data.sign };
                    return Ok(FlatPoleInspection {
                        report: FlatPoleReport {
                            proven: true,
                            differential_proven: true,
                            quotient_proven: true,
                            reason: "original-flat-pole-quotient-and-limits",
                            cells: work,
                            pending: 0,
                            normal_coefficient: Some(data.normal),
                            tensor_coefficient: Some(data.tensor),
                            normal_axis: height_axis,
                            normal_sign: Some(sign),
                            pole: prepared.pole,
                            original_u_span: prepared.source_u,
                            band_normalized: band,
                            fan_angle_upper: Some(data.angle),
                            reference_radius: Some(radius),
                            quotient_lower: Some(lower),
                        },
                        certificate: Some(FlatPoleCertificate {
                            source: s,
                            end,
                            axis: height_axis,
                            band,
                            max_cells,
                            normal: data.normal,
                            tensor: data.tensor,
                            sign,
                            pole: prepared.pole,
                            cells: work,
                            angle: data.angle,
                            radius,
                            quotient_lower: lower,
                        }),
                    });
                }
                return Ok(empty(
                    "pole-quotient-radius-unrepresentable",
                    work,
                    1,
                    Some(data),
                    prepared.source_u,
                ));
            }
        }
        if work == max_cells {
            return Ok(empty(
                "pole-band-work-exhausted",
                work,
                cells.iter().filter(|c| c.bound.is_none()).count().max(1),
                sum.as_ref(),
                prepared.source_u,
            ));
        }
        let at = if cells.iter().any(|c| c.bound.is_none()) {
            cells
                .iter()
                .enumerate()
                .filter(|(_, c)| c.bound.is_none())
                .max_by(|(_, a), (_, b)| (a.v[1] - a.v[0]).total_cmp(&(b.v[1] - b.v[0])))
                .unwrap()
                .0
        } else {
            cells
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| {
                    a.bound
                        .as_ref()
                        .unwrap()
                        .fan
                        .angle
                        .total_cmp(&b.bound.as_ref().unwrap().fan.angle)
                })
                .unwrap()
                .0
        };
        let old = cells[at].v;
        let mid = old[0] * 0.5 + old[1] * 0.5;
        if !(old[0] < mid && mid < old[1]) {
            return Ok(empty(
                "pole-band-subdivision-unrepresentable",
                work,
                1,
                sum.as_ref(),
                prepared.source_u,
            ));
        }
        let mut children = Vec::new();
        for v in [[old[0], mid], [mid, old[1]]] {
            if work == max_cells {
                break;
            }
            work += 1;
            children.push(Cell {
                v,
                bound: bounds::calculate(&prepared, band, v)?,
            });
        }
        if children.len() == 2 {
            cells.swap_remove(at);
            cells.extend(children);
        }
    }
}
#[cfg(test)]
#[path = "flat_pole/tests.rs"]
mod tests;
