//! Exact source-expression restrictions with root-valued endpoints.
//! No rounded Curve::trim, Cartesian vertex welding or Model admission.
use crate::source_contact_point::{self, SourcePoint};
use nurbs_core::{curve::Curve, surface::Surface, Error, Result};
use value_codec::{json, Deserialize, Serialize, Value};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Boundary,
    Contact,
}
#[derive(Clone)]
pub enum Endpoint {
    Parameter(f64),
    Crossing { point: SourcePoint, role: Role },
}
#[derive(Clone)]
pub struct Fragment {
    surface: Surface,
    curve: Curve,
    endpoints: [Endpoint; 2],
    parameter_bounds: [[f64; 2]; 2],
    reversed: bool,
}
fn error(message: &str) -> Error {
    Error::new("BREP_SOURCE_FRAGMENT", message)
}
fn parameter(surface: &Surface, curve: &Curve, end: &Endpoint) -> Result<[f64; 2]> {
    let domain = curve.domain();
    match end {
        Endpoint::Parameter(t) => {
            if !t.is_finite() || *t < domain[0] || *t > domain[1] {
                return Err(error("Endpoint parameter is outside the original curve"));
            }
            Ok([*t, *t])
        }
        Endpoint::Crossing { point, role } => {
            let (source, index) = match role {
                Role::Boundary => (point.boundary(), 0),
                Role::Contact => (point.contact(), 1),
            };
            if point.surface() != surface || source != curve {
                return Err(error(
                    "Crossing endpoint must refer to this original surface and curve",
                ));
            }
            Ok(point.root_parameters()[index])
        }
    }
}
impl Fragment {
    pub fn new(surface: &Surface, curve: &Curve, start: Endpoint, end: Endpoint) -> Result<Self> {
        surface.validate()?;
        curve.validate()?;
        if surface.periodic_u
            || surface.periodic_v
            || curve.periodic
            || surface.control_points[0][0].len() != 3
            || curve.control_points[0].len() != 2
        {
            return Err(error(
                "Source fragment requires a nonperiodic 3D surface and UV curve",
            ));
        }
        let chart = [
            [
                surface.knots_u[surface.degree_u],
                surface.knots_u[surface.control_points.len()],
            ],
            [
                surface.knots_v[surface.degree_v],
                surface.knots_v[surface.control_points[0].len()],
            ],
        ];
        // Restriction must be a continuous source path, without manufacturing
        // connecting segments across a discontinuity of either source definition.
        for (knots, degree, n) in [
            (&curve.knots, curve.degree, curve.control_points.len()),
            (
                &surface.knots_u,
                surface.degree_u,
                surface.control_points.len(),
            ),
            (
                &surface.knots_v,
                surface.degree_v,
                surface.control_points[0].len(),
            ),
        ] {
            for &k in knots {
                if k > knots[degree]
                    && k < knots[n]
                    && knots.iter().filter(|&&x| x == k).count() > degree
                {
                    return Err(error(
                        "Source path continuity is unproven at an original knot",
                    ));
                }
            }
        }
        let parameter_bounds = [
            parameter(surface, curve, &start)?,
            parameter(surface, curve, &end)?,
        ];
        let reversed = if parameter_bounds[0][1] < parameter_bounds[1][0] {
            false
        } else if parameter_bounds[1][1] < parameter_bounds[0][0] {
            true
        } else {
            return Err(error("Source endpoint order is not strictly proven"));
        };
        let in_chart = curve
            .control_points
            .iter()
            .all(|p| (0..2).all(|i| chart[i][0] <= p[i] && p[i] <= chart[i][1]));
        if !in_chart {
            let d = curve.domain();
            let single_line = curve.degree == 1
                && curve.control_points.len() == 2
                && curve.knots[..=1].iter().all(|&k| k == d[0])
                && curve.knots[2..].iter().all(|&k| k == d[1]);
            let endpoint_inside = |end: &Endpoint| -> Result<bool> {
                match end {
                    Endpoint::Crossing { point, .. } => Ok((0..2).all(|i| {
                        point.uv_box()[i][0] >= chart[i][0] && point.uv_box()[i][1] <= chart[i][1]
                    })),
                    Endpoint::Parameter(t) => {
                        if *t == d[0] || *t == d[1] {
                            let p = &curve.control_points[usize::from(*t == d[1])];
                            return Ok((0..2).all(|i| p[i] >= chart[i][0] && p[i] <= chart[i][1]));
                        }
                        let p = nurbs_core::interval_eval::evaluate_interval(
                            curve,
                            nurbs_core::interval_eval::Interval::point(*t),
                        )?;
                        Ok((0..2).all(|i| p[i].lo >= chart[i][0] && p[i].hi <= chart[i][1]))
                    }
                }
            };
            // Positive rational linear Bezier restriction traces the segment
            // between its exact endpoints. The natural chart rectangle is convex.
            let line_inside = single_line && endpoint_inside(&start)? && endpoint_inside(&end)?;
            if !line_inside {
                let lo = parameter_bounds[0][0].min(parameter_bounds[1][0]);
                let hi = parameter_bounds[0][1].max(parameter_bounds[1][1]);
                let image = nurbs_core::interval_eval::evaluate_interval(
                    curve,
                    nurbs_core::interval_eval::Interval::new(lo, hi)?,
                )?;
                if !(0..2).all(|i| image[i].lo >= chart[i][0] && image[i].hi <= chart[i][1]) {
                    return Err(error(
                        "Restricted source curve chart membership is unproven",
                    ));
                }
            }
        }
        Ok(Self {
            surface: surface.clone(),
            curve: curve.clone(),
            endpoints: [start, end],
            parameter_bounds,
            reversed,
        })
    }
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn curve(&self) -> &Curve {
        &self.curve
    }
    pub fn endpoints(&self) -> &[Endpoint; 2] {
        &self.endpoints
    }
    pub fn parameter_bounds(&self) -> [[f64; 2]; 2] {
        self.parameter_bounds
    }
    pub fn reversed(&self) -> bool {
        self.reversed
    }
    pub fn split_at(&self, point: &SourcePoint, role: Role) -> Result<[Self; 2]> {
        let end = Endpoint::Crossing {
            point: point.clone(),
            role,
        };
        let bounds = parameter(&self.surface, &self.curve, &end)?;
        let [a, b] = self.parameter_bounds;
        let inside = if self.reversed {
            b[1] < bounds[0] && bounds[1] < a[0]
        } else {
            a[1] < bounds[0] && bounds[1] < b[0]
        };
        if !inside {
            return Err(error(
                "Split crossing must lie strictly inside the source fragment",
            ));
        }
        Ok([
            Self::new(
                &self.surface,
                &self.curve,
                self.endpoints[0].clone(),
                end.clone(),
            )?,
            Self::new(&self.surface, &self.curve, end, self.endpoints[1].clone())?,
        ])
    }
    /// Identity by shared source expression, never coordinate proximity.
    pub fn joins(&self, next: &Self) -> bool {
        if self.surface != next.surface {
            return false;
        }
        match (&self.endpoints[1], &next.endpoints[0]) {
            (Endpoint::Crossing { point: a, .. }, Endpoint::Crossing { point: b, .. }) => {
                a.definition() == b.definition()
            }
            (Endpoint::Parameter(a), Endpoint::Parameter(b)) => {
                if self.curve == next.curve && a.to_bits() == b.to_bits() {
                    return true;
                }
                let clamped_point = |c: &Curve, t: f64| -> Option<Vec<u64>> {
                    let d = c.domain();
                    let n = c.control_points.len();
                    if t == d[0] && c.knots[..=c.degree].iter().all(|&x| x == t) {
                        Some(c.control_points[0].iter().map(|x| x.to_bits()).collect())
                    } else if t == d[1] && c.knots[n..].iter().all(|&x| x == t) {
                        Some(
                            c.control_points[n - 1]
                                .iter()
                                .map(|x| x.to_bits())
                                .collect(),
                        )
                    } else {
                        None
                    }
                };
                clamped_point(&self.curve, *a)
                    .zip(clamped_point(&next.curve, *b))
                    .is_some_and(|(a, b)| a == b)
            }
            _ => false,
        }
    }
    pub fn definition(&self) -> Value {
        let encode = |end: &Endpoint| match end {
            Endpoint::Parameter(t) => json!({"kind":"parameter","value":t}),
            Endpoint::Crossing { point, role } => {
                json!({"kind":"crossing","point":point.definition(),
                "role":match role {Role::Boundary=>"boundary",Role::Contact=>"contact"}})
            }
        };
        json!({"version":1,"surface":self.surface.to_value(),"curve":self.curve.to_value(),
            "endpoints":[encode(&self.endpoints[0]),encode(&self.endpoints[1])]})
    }
}
/// Restore each root by fresh original-definition qualification. Mapping work is
/// shared across both endpoints; cached direction/order/bounds are ignored.
pub fn restore(value: Value, max_mapping_cells: usize) -> Result<Fragment> {
    if value["version"].as_u64() != Some(1) || !(1..=100000).contains(&max_mapping_cells) {
        return Err(error("Unsupported fragment definition or mapping budget"));
    }
    let decode = |e: value_codec::Error| error(&e.to_string());
    let surface = Surface::from_value(value["surface"].clone()).map_err(decode)?;
    let curve = Curve::from_value(value["curve"].clone()).map_err(decode)?;
    let ends = value["endpoints"]
        .as_array()
        .filter(|a| a.len() == 2)
        .ok_or_else(|| error("Fragment needs two endpoints"))?;
    let mut remaining = max_mapping_cells;
    let mut endpoints = vec![];
    for end in ends {
        let endpoint = match end["kind"].as_str() {
            Some("parameter") => Endpoint::Parameter(
                end["value"]
                    .as_f64()
                    .ok_or_else(|| error("Missing original parameter"))?,
            ),
            Some("crossing") => {
                if remaining == 0 {
                    return Err(error("Source endpoint mapping work exhausted"));
                }
                let report = source_contact_point::restore(end["point"].clone(), remaining)?;
                remaining -= report.mapping_cells;
                let point = report
                    .point
                    .ok_or_else(|| error("Imported crossing endpoint is unqualified"))?;
                let role = match end["role"].as_str() {
                    Some("boundary") => Role::Boundary,
                    Some("contact") => Role::Contact,
                    _ => return Err(error("Unknown crossing source role")),
                };
                Endpoint::Crossing { point, role }
            }
            _ => return Err(error("Unknown endpoint definition")),
        };
        endpoints.push(endpoint);
    }
    Fragment::new(&surface, &curve, endpoints.remove(0), endpoints.remove(0))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Surface, Curve, Curve, SourcePoint) {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![0., 1.], vec![1., 0.]]).unwrap();
        let p = source_contact_point::qualify(&s, &a, &b, [[0., 1.]; 2], 16)
            .unwrap()
            .point
            .unwrap();
        (s, a, b, p)
    }
    #[test]
    fn root_valued_split_preserves_sources_and_joins_across_original_curves() {
        let (s, a, b, p) = fixture();
        let full = Fragment::new(&s, &a, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
        let parts = full.split_at(&p, Role::Boundary).unwrap();
        assert!(parts[0].joins(&parts[1]));
        assert_eq!(parts[0].curve(), &a);
        assert_eq!(parts[1].curve(), &a);
        let contact = Fragment::new(
            &s,
            &b,
            Endpoint::Crossing {
                point: p.clone(),
                role: Role::Contact,
            },
            Endpoint::Parameter(1.),
        )
        .unwrap();
        assert!(parts[0].joins(&contact));
        let reversed =
            Fragment::new(&s, &a, Endpoint::Parameter(1.), Endpoint::Parameter(0.)).unwrap();
        let reversed = reversed.split_at(&p, Role::Boundary).unwrap();
        assert!(reversed[0].reversed() && reversed[0].joins(&reversed[1]));
        assert!(full.split_at(&p, Role::Contact).is_err());
    }
    #[test]
    fn restored_fragment_rechecks_source_binding_and_root_order() {
        let (s, a, _, p) = fixture();
        let f = Fragment::new(
            &s,
            &a,
            Endpoint::Parameter(0.),
            Endpoint::Crossing {
                point: p,
                role: Role::Boundary,
            },
        )
        .unwrap();
        let restored = restore(f.definition(), 16).unwrap();
        assert_eq!(restored.definition(), f.definition());
        let mut forged = f.definition();
        forged["curve"]["controlPoints"][0][0] = json!(0.1);
        assert!(restore(forged, 16).is_err());
        let mut forged = f.definition();
        forged["endpoints"][0]["value"] = json!(0.5);
        assert!(restore(forged, 16).is_err());
    }
    #[test]
    fn source_chart_containment_is_not_inferred_from_two_inside_endpoints() {
        let (s, _, _, _) = fixture();
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0.2, 0.5], vec![3., 0.5], vec![0.8, 0.5]],
            weights: vec![1.; 3],
            periodic: false,
        };
        assert!(Fragment::new(&s, &c, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).is_err());
        let c = Curve::from_polyline(vec![vec![-0.2, 0.3], vec![1.2, 0.3]]).unwrap();
        assert!(Fragment::new(&s, &c, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).is_err());
    }
}
