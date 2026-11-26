//! Closed material between two complete interior sections of one periodic carrier.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};

#[derive(Clone, Copy, Debug)]
pub struct RootPeriodicBandLimits {
    pub max_chart_attempts: usize,
    pub max_orientation_probes: usize,
    pub max_chart_spans: usize,
    pub max_contact_cells: usize,
    pub flux: RootShellFluxLimits,
}

impl QualifiedRootShellPositiveFlux {
    /// Trim the original side and both planes, orient and sew the three faces,
    /// then qualify their contacts and positive volume. Sections may be supplied
    /// in either order and use either ordered SS operand slot.
    pub fn qualify_periodic_band(
        mut sections: [&ss::QualifiedPeriodicPlaneSS; 2],
        tolerance_mm: f64,
        limits: RootPeriodicBandLimits,
    ) -> Result<Self> {
        let [first, second] = sections;
        if sections
            .iter()
            .any(|section| section.kind() != ss::PeriodicPlaneSSKind::ClosedContour)
            || support(first, false) != support(second, false)
            || first.periodic_axis() != second.periodic_axis()
        {
            return Err(unproved(
                "Periodic band needs two regular sections of the same carrier",
            ));
        }
        if first.transverse_parameter() == second.transverse_parameter() {
            return Err(unproved(
                "Periodic band section parameters must be distinct",
            ));
        }
        if first.transverse_parameter() > second.transverse_parameter() {
            sections.swap(0, 1);
        }
        let [first, second] = sections;
        let axial = first
            .support_proof()
            .axial_coordinate
            .ok_or_else(|| unproved("Periodic band lacks an original axial coordinate"))?;
        let [a, b] = sections.map(|section| support(section, true));
        let second_is_higher = a.control_points[0][0][axial] < b.control_points[0][0][axial];
        let geometry = limits.flux.winding.geometry;
        let wires = [
            first.to_root_wire(geometry, limits.max_chart_attempts)?,
            second.to_root_wire(geometry, limits.max_chart_attempts)?,
        ];

        // Use positive UV boundary traversal on every chart before applying
        // its shell face reversal. Opposite atlas orientations then sew alike.
        let mut caps = Vec::new();
        let mut side_reversed = false;
        let side_first_forward = first.periodic_axis() == 0;
        for (i, (section, wire)) in sections.into_iter().zip(&wires).enumerate() {
            let plane = support(section, true);
            let original = Arc::new(RootBrep::face_contours(
                plane.clone(),
                contour(wire.clone(), 1 - section.periodic_surface()),
                vec![],
                tolerance_mm,
            )?);
            let cap_forward = cap_forward(&original, limits)?;
            let upper = (i == 1) == second_is_higher;
            let reversed = normal_positive(plane, axial) != upper;
            if i == 0 {
                side_reversed = side_first_forward == (cap_forward ^ reversed);
            }
            let cap = if cap_forward {
                (*original).clone()
            } else {
                RootBrep::face_contours(
                    plane.clone(),
                    contour(reverse_wire(wire)?, 1 - section.periodic_surface()),
                    vec![],
                    tolerance_mm,
                )?
            };
            caps.push(oriented(cap, reversed)?);
        }

        let [a, b] = wires;
        let (a, b) = if side_first_forward {
            (a, reverse_wire(&b)?)
        } else {
            (reverse_wire(&a)?, b)
        };
        let side = RootBrep::periodic_face_contours(
            support(first, false).clone(),
            contour(a, first.periodic_surface()),
            vec![contour(b, second.periodic_surface())],
            tolerance_mm,
        )?;
        caps.push(oriented(side, side_reversed)?);
        let shell = Arc::new(RootBrep::sew_root_shell(&caps, tolerance_mm)?);
        let charts = QualifiedRootShellCharts::qualify(shell, limits.max_chart_spans, geometry)?;
        let contacts =
            QualifiedRootShellContacts::qualify(charts, vec![], limits.max_contact_cells)?;
        let measure = PeriodicBandMeasure::from_sections(sections, axial);
        Self::qualify_with_band_measure(contacts, limits.flux, measure)
    }
}

fn support(section: &ss::QualifiedPeriodicPlaneSS, plane: bool) -> &Surface {
    if (section.periodic_surface() == 0) != plane {
        section.source().first()
    } else {
        section.source().second()
    }
}

fn contour(wire: ss::OriginalSsRootWire, support: usize) -> RootFaceContour {
    RootFaceContour {
        supports: vec![support; wire.edges().len()],
        wire,
    }
}

fn reverse_wire(wire: &ss::OriginalSsRootWire) -> Result<ss::OriginalSsRootWire> {
    let edges = wire
        .edges()
        .iter()
        .rev()
        .map(|edge| ss::OriginalSsRootEdge::new(edge.end().clone(), edge.start().clone()))
        .collect::<nurbs_core::Result<Vec<_>>>()?;
    Ok(ss::OriginalSsRootWire::new(edges)?)
}

fn oriented(face: RootBrep, reversed: bool) -> Result<Arc<RootBrep>> {
    let mut topology = face.topology().clone();
    topology.shells[0].faces[0].reversed = reversed;
    Ok(Arc::new(RootBrep::from_topology(topology)?))
}

// The section proof restricts the target to an axis-aligned affine rectangle.
// Exactly one product of coordinate differences contributes to the cross product.
fn normal_positive(plane: &Surface, axis: usize) -> bool {
    let a = (axis + 1) % 3;
    let b = (axis + 2) % 3;
    let p = &plane.control_points[0][0];
    let u = &plane.control_points[1][0];
    let v = &plane.control_points[0][1];
    if u[a] != p[a] {
        (u[a] > p[a]) == (v[b] > p[b])
    } else {
        (u[b] > p[b]) != (v[a] > p[a])
    }
}

fn cap_forward(cap: &Arc<RootBrep>, limits: RootPeriodicBandLimits) -> Result<bool> {
    let bounds = winding::lifted_loop_bounds(cap, 0, 0, &mut limits.flux.winding.geometry.clone())?;
    let mut remaining = limits.max_orientation_probes;
    // Try a bounded sequence of dense dyadic centers; only a certified
    // nonzero winding decides the cap orientation.
    for level in 0..=16 {
        let cells = 1 << level;
        for i in 0..cells {
            for j in 0..cells {
                if remaining == 0 {
                    return Err(unproved("Periodic cap orientation probe budget exhausted"));
                }
                remaining -= 1;
                let q = [i, j].map(|k| (f64::from(k) + 0.5) / f64::from(cells));
                let query = std::array::from_fn(|k| {
                    let [lo, hi] = bounds[k];
                    [lo * (1. - q[k]) + hi * q[k]; 2]
                });
                if let Ok(proof) =
                    QualifiedRootLoopWinding::qualify(cap.clone(), 0, 0, query, limits.flux.winding)
                    && proof.winding().unsigned_abs() == 1
                {
                    return Ok(proof.winding() > 0);
                }
            }
        }
    }
    Err(unproved("Periodic cap orientation remains unproved"))
}

/// Cavalieri integration of a positive homothetic family. The original
/// extrusion certificate establishes the closed Jordan profile and the
/// affine relation between world height and the homothety parameter.
#[derive(Clone, Debug)]
pub(super) struct PeriodicBandMeasure {
    profile: crate::Curve,
    factor: Q,
}

impl PeriodicBandMeasure {
    fn from_sections(sections: [&ss::QualifiedPeriodicPlaneSS; 2], axial: usize) -> Option<Self> {
        let [first, second] = sections;
        let side = support(first, false);
        let base = first.support_proof().base_end?;
        let [u, v] = [(axial + 1) % 3, (axial + 2) % 3];
        let (knots, degree, along_u) = if first.periodic_axis() == 0 {
            (&side.knots_u, side.degree_u, true)
        } else {
            (&side.knots_v, side.degree_v, false)
        };
        // Reuse the original binary coefficients of the proved Jordan profile.
        // The section coordinates and scale ratios stay exact rationals.
        let (points, weights) = (0..first.level_control_points().len())
            .map(|i| {
                let (row, col) = if along_u { (i, base) } else { (base, i) };
                let p = &side.control_points[row][col];
                (vec![p[u], p[v]], side.weights[row][col])
            })
            .unzip::<_, _, Vec<_>, Vec<_>>();
        let base_origin = points.first()?;
        let (index, axis) = points.iter().enumerate().find_map(|(index, p)| {
            (0..2)
                .find(|&axis| p[axis] != base_origin[axis])
                .map(|axis| (index, axis))
        })?;
        let world = [u, v][axis];
        let delta = Q::from_float(points[index][axis])? - Q::from_float(base_origin[axis])?;
        let [a, b] = sections.map(|section| {
            let poles = section.level_control_points();
            (&poles[index][world] - &poles[0][world]) / &delta
        });
        if !a.is_positive() || !b.is_positive() {
            return None;
        }
        let height = (&first.level_control_points()[0][axial]
            - &second.level_control_points()[0][axial])
            .abs();
        let factor = height * (a.pow(2) + &a * &b + b.pow(2)) / Q::from_integer(3.into());
        Some(Self {
            profile: crate::Curve {
                degree,
                knots: knots.clone(),
                control_points: points,
                weights,
                periodic: true,
            },
            factor,
        })
    }

    pub(super) fn report(&self, limits: RootShellFluxLimits) -> Result<RootShellFluxReport> {
        // The existing exact spline extractor uses small Bernstein degrees.
        if self.profile.degree > 12 {
            return Err(unproved(
                "Periodic profile degree exceeds the exact area path",
            ));
        }
        let spans = (self.profile.degree..self.profile.control_points.len())
            .filter(|&i| self.profile.knots[i] < self.profile.knots[i + 1])
            .count();
        let mut cells = 0;
        let mut used_spans = 0;
        let mut report = None;
        for depth in 0..=limits.max_depth.min(12) {
            let next_cells = spans << depth;
            if next_cells > limits.max_cells - cells || spans > limits.max_spans - used_spans {
                break;
            }
            let [mut lo, mut hi] = crate::profile::profile_certificate::signed_curve_area_bounds(
                &self.profile,
                depth as u8,
            )?;
            cells += next_cells;
            used_spans += spans;
            if hi < Q::zero() {
                (lo, hi) = (-hi, -lo);
            }
            if lo > Q::zero() {
                report = Some(RootShellFluxReport {
                    signed_bounds: [
                        winding::outward(&(&lo * &self.factor), true)?,
                        winding::outward(&(&hi * &self.factor), false)?,
                    ],
                    cells,
                    spans: used_spans,
                });
                if (&hi - &lo) * Q::from_integer(1000.into()) <= lo {
                    break;
                }
            }
        }
        report
            .filter(RootShellFluxReport::has_positive_lower_bound)
            .ok_or_else(|| unproved("Periodic band area lacks a positive bound"))
    }
}
