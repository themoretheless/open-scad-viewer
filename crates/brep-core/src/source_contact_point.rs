//! Immutable source-expression point for a certified original UV crossing.
//! This is not a Cartesian Model vertex or exact world-edge endpoint admission.
use nurbs_core::{
    curve::Curve,
    interval_eval::{self, Interval as I},
    surface::Surface,
    uv_curve_crossings as crossings, Error, Result,
};
use value_codec::{json, Deserialize, Serialize, Value};
#[derive(Clone)]
pub struct SourcePoint {
    surface: Surface,
    boundary: Curve,
    contact: Curve,
    selector: [[f64; 2]; 2],
    root_parameters: [[f64; 2]; 2],
    uv_box: [[f64; 2]; 2],
    world_box: [[f64; 2]; 3],
}
impl SourcePoint {
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn boundary(&self) -> &Curve {
        &self.boundary
    }
    pub fn contact(&self) -> &Curve {
        &self.contact
    }
    pub fn selector(&self) -> [[f64; 2]; 2] {
        self.selector
    }
    pub fn root_parameters(&self) -> [[f64; 2]; 2] {
        self.root_parameters
    }
    pub fn uv_box(&self) -> [[f64; 2]; 2] {
        self.uv_box
    }
    pub fn world_box(&self) -> [[f64; 2]; 3] {
        self.world_box
    }
    /// Persist only original definitions and root selector, never cached proof.
    pub fn definition(&self) -> Value {
        json!({"version":1,"surface":self.surface.to_value(),"boundary":self.boundary.to_value(),
            "contact":self.contact.to_value(),"selector":self.selector})
    }
}
pub struct Report {
    pub crossing: crossings::Cell,
    pub point: Option<SourcePoint>,
    pub mapping_cells: usize,
    pub reason: &'static str,
}
/// Independently requalify the supplied original root box and world expression.
/// Surface span work is bounded separately from the one-box crossing check.
pub fn qualify(
    surface: &Surface,
    boundary: &Curve,
    contact: &Curve,
    selector: [[f64; 2]; 2],
    max_mapping_cells: usize,
) -> Result<Report> {
    surface.validate()?;
    if surface.periodic_u
        || surface.periodic_v
        || surface.control_points[0][0].len() != 3
        || !(1..=100000).contains(&max_mapping_cells)
    {
        return Err(Error::new(
            "BREP_SOURCE_POINT_INPUT",
            "Choose a nonperiodic 3D source surface and bounded mapping work",
        ));
    }
    let crossing = crossings::certify_box(boundary, contact, selector)?;
    let mut out = Report {
        crossing,
        point: None,
        mapping_cells: 0,
        reason: "source-crossing-unqualified",
    };
    if out.crossing.state != crossings::State::Unique {
        return Ok(out);
    }
    let root = out.crossing.root.unwrap();
    let du = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
    ];
    let dv = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.control_points[0].len()],
    ];
    let chart = [du, dv];
    // Positive rational B-spline basis puts the entire original boundary image
    // inside its Cartesian control hull. This separately justifies chart clipping
    // of outward point enclosures at original natural-domain boundaries.
    if !boundary
        .control_points
        .iter()
        .all(|p| (0..2).all(|i| p[i] >= chart[i][0] && p[i] <= chart[i][1]))
    {
        out.reason = "source-chart-membership-unproven";
        return Ok(out);
    }
    let a = interval_eval::evaluate_interval(boundary, I::new(root[0][0], root[0][1])?)?;
    let b = interval_eval::evaluate_interval(contact, I::new(root[1][0], root[1][1])?)?;
    let mut uv = [[0.; 2]; 2];
    for i in 0..2 {
        let lo = a[i].lo.max(b[i].lo).max(chart[i][0]);
        let hi = a[i].hi.min(b[i].hi).min(chart[i][1]);
        if lo > hi {
            return Err(Error::new(
                "BREP_SOURCE_POINT_NUMERIC",
                "Certified source crossing point enclosures became disjoint",
            ));
        }
        uv[i] = [lo, hi];
    }
    let count = |knots: &[f64], degree: usize, n: usize, range: [f64; 2]| {
        (degree..n)
            .filter(|&i| {
                knots[i] < knots[i + 1] && knots[i] <= range[1] && knots[i + 1] >= range[0]
            })
            .count()
    };
    let n = count(
        &surface.knots_u,
        surface.degree_u,
        surface.control_points.len(),
        uv[0],
    )
    .checked_mul(count(
        &surface.knots_v,
        surface.degree_v,
        surface.control_points[0].len(),
        uv[1],
    ));
    if n.is_none_or(|n| n > max_mapping_cells) {
        out.reason = "source-mapping-work-limit";
        return Ok(out);
    }
    out.mapping_cells = n.unwrap();
    let world = interval_eval::evaluate_surface_interval(
        surface,
        I::new(uv[0][0], uv[0][1])?,
        I::new(uv[1][0], uv[1][1])?,
    )?;
    out.point = Some(SourcePoint {
        surface: surface.clone(),
        boundary: boundary.clone(),
        contact: contact.clone(),
        selector,
        root_parameters: root,
        uv_box: uv,
        world_box: std::array::from_fn(|i| [world[i].lo, world[i].hi]),
    });
    out.reason = "source-expression-point-qualified";
    Ok(out)
}
/// Imported data supplies definitions/selector only. Proof and bounds are fresh.
pub fn restore(value: Value, max_mapping_cells: usize) -> Result<Report> {
    if value["version"].as_u64() != Some(1) {
        return Err(Error::new(
            "BREP_SOURCE_POINT_VERSION",
            "Unsupported source point definition",
        ));
    }
    let decode = |e: value_codec::Error| Error::new("BREP_SOURCE_POINT_DEFINITION", &e.to_string());
    let surface = Surface::from_value(value["surface"].clone()).map_err(decode)?;
    let boundary = Curve::from_value(value["boundary"].clone()).map_err(decode)?;
    let contact = Curve::from_value(value["contact"].clone()).map_err(decode)?;
    let selector = <[[f64; 2]; 2]>::from_value(value["selector"].clone()).map_err(decode)?;
    qualify(&surface, &boundary, &contact, selector, max_mapping_cells)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Surface, Curve, Curve) {
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![1., 2., 3.], vec![1., 5., 3.]],
                vec![vec![3., 2., 3.], vec![3., 5., 3.]],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![0., 1.], vec![1., 0.]]).unwrap();
        (surface, a, b)
    }
    #[test]
    fn immutable_source_point_roundtrip_and_forged_claims_are_rechecked() {
        let (s, a, b) = fixture();
        let r = qualify(&s, &a, &b, [[0., 1.]; 2], 16).unwrap();
        let p = r.point.unwrap();
        for (i, x) in [2., 3.5, 3.].into_iter().enumerate() {
            assert!(p.world_box()[i][0] <= x && p.world_box()[i][1] >= x);
        }
        let mut value = p.definition();
        value["claimedQualified"] = json!(true);
        value["worldBounds"] = [[999., 999.]; 3].to_value();
        let restored = restore(value.clone(), 16).unwrap().point.unwrap();
        assert_eq!(restored.definition(), p.definition());
        assert_eq!(restored.world_box(), p.world_box());
        value["selector"] = [[0., 0.25]; 2].to_value();
        assert!(restore(value, 16).unwrap().point.is_none());
    }
    #[test]
    fn original_chart_membership_is_required_before_surface_enclosure_clipping() {
        let (s, mut a, b) = fixture();
        a.control_points[0] = vec![-0.1, -0.1];
        let r = qualify(&s, &a, &b, [[0., 1.]; 2], 16).unwrap();
        assert_eq!(r.crossing.state, crossings::State::Unique);
        assert!(r.point.is_none());
        assert_eq!(r.reason, "source-chart-membership-unproven");
    }
}
