//! Retained spline geometry for regular rectangular quad control cages.
//! This is not an extraordinary-vertex/T-junction subdivision limit solver.
use crate::{Error, Result, surface::Surface};
#[derive(Clone, Debug, PartialEq)]
pub struct PolyhedralSpline {
    surface: Surface,
}
#[derive(Clone, Copy, Debug)]
pub enum Boundary {
    UMin,
    UMax,
    VMin,
    VMax,
}
#[derive(Clone, Copy, Debug)]
pub enum Continuity {
    G0,
    ParametricC1,
}
#[derive(Clone, Debug)]
pub struct Reconciliation {
    pub first: PolyhedralSpline,
    pub second: PolyhedralSpline,
    pub max_control_displacement: f64,
    /// A coefficient construction, not proof of a globally regular embedding.
    pub regularity_certified: bool,
}
fn error(message: &str) -> Error {
    Error::new("NURBS_POLYSPLINE_REFUSED", message)
}
/// Retain an extracted Catmull–Clark regular patch as a NURBS surface on [0,1]^2.
/// The uniform basis is deliberately not clamped: clamping the same controls
/// would change the limit surface.
pub fn from_uniform_patch(controls: [[[f64; 3]; 4]; 4]) -> Result<Surface> {
    let knots = (-3..=4).map(f64::from).collect::<Vec<_>>();
    let surface = Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: knots.clone(),
        knots_v: knots,
        control_points: controls
            .into_iter()
            .map(|row| row.into_iter().map(|p| p.to_vec()).collect())
            .collect(),
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    Ok(surface)
}
fn knots(n: usize) -> Vec<f64> {
    let mut k = vec![0.; 4];
    k.extend((1..n - 3).map(|i| i as f64 / (n - 3) as f64));
    k.extend([1.; 4]);
    k
}
impl PolyhedralSpline {
    pub fn from_regular_grid(controls: Vec<Vec<[f64; 3]>>) -> Result<Self> {
        let n = controls.len();
        let m = controls.first().map_or(0, Vec::len);
        if n < 4 || m < 4 || n * m > 256 || controls.iter().any(|row| row.len() != m) {
            return Err(error(
                "Regular quad cage requires a rectangular >=4x4 grid with <=256 controls",
            ));
        }
        let surface = Surface {
            degree_u: 3,
            degree_v: 3,
            knots_u: knots(n),
            knots_v: knots(m),
            control_points: controls
                .into_iter()
                .map(|r| r.into_iter().map(|p| p.to_vec()).collect())
                .collect(),
            weights: vec![vec![1.; m]; n],
            periodic_u: false,
            periodic_v: false,
        };
        surface.validate()?;
        Ok(Self { surface })
    }
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn control_point(&self, i: usize, j: usize) -> Option<[f64; 3]> {
        self.surface
            .control_points
            .get(i)?
            .get(j)
            .map(|p| [p[0], p[1], p[2]])
    }
    pub fn with_control_point(&self, i: usize, j: usize, point: [f64; 3]) -> Result<Self> {
        let mut result = self.clone();
        let p = result
            .surface
            .control_points
            .get_mut(i)
            .and_then(|row| row.get_mut(j))
            .ok_or_else(|| error("Unknown cage control point"))?;
        *p = point.to_vec();
        result.surface.validate()?;
        Ok(result)
    }
    /// Exact rational Bezier decomposition of the retained spline definition.
    pub fn patches(&self) -> Result<Vec<Surface>> {
        crate::surface_edit::decompose(&self.surface)
    }
}
fn layout(s: &Surface, b: Boundary) -> (usize, f64) {
    let n = s.control_points.len();
    let m = s.control_points[0].len();
    match b {
        Boundary::UMin => (m, 3. / (s.knots_u[4] - s.knots_u[3])),
        Boundary::UMax => (m, 3. / (s.knots_u[n] - s.knots_u[n - 1])),
        Boundary::VMin => (n, 3. / (s.knots_v[4] - s.knots_v[3])),
        Boundary::VMax => (n, 3. / (s.knots_v[m] - s.knots_v[m - 1])),
    }
}
fn indices(s: &Surface, b: Boundary, k: usize) -> ([usize; 2], [usize; 2]) {
    let n = s.control_points.len();
    let m = s.control_points[0].len();
    match b {
        Boundary::UMin => ([0, k], [1, k]),
        Boundary::UMax => ([n - 1, k], [n - 2, k]),
        Boundary::VMin => ([k, 0], [k, 1]),
        Boundary::VMax => ([k, m - 1], [k, m - 2]),
    }
}
fn get(s: &Surface, p: [usize; 2]) -> [f64; 3] {
    let v = &s.control_points[p[0]][p[1]];
    [v[0], v[1], v[2]]
}
fn set(s: &mut Surface, p: [usize; 2], v: [f64; 3]) {
    s.control_points[p[0]][p[1]] = v.to_vec();
}
/// Reconciles matching regular boundary bases. C1 matches opposite inward
/// derivative fields along the full edge; global regularity is not certified.
pub fn reconcile(
    a: &PolyhedralSpline,
    edge_a: Boundary,
    b: &PolyhedralSpline,
    edge_b: Boundary,
    reverse: bool,
    continuity: Continuity,
) -> Result<Reconciliation> {
    let (n, fa) = layout(&a.surface, edge_a);
    let (m, fb) = layout(&b.surface, edge_b);
    if n != m {
        return Err(error("Matching regular boundary control counts required"));
    }
    let mut first = a.clone();
    let mut second = b.clone();
    let mut maximum: f64 = 0.;
    for k in 0..n {
        let (ea, ia) = indices(&a.surface, edge_a, k);
        let (eb, ib) = indices(&b.surface, edge_b, if reverse { n - 1 - k } else { k });
        let pa = get(&a.surface, ea);
        let pb = get(&b.surface, eb);
        let qa = get(&a.surface, ia);
        let qb = get(&b.surface, ib);
        let seam = std::array::from_fn::<_, 3, _>(|i| pa[i] / 2. + pb[i] / 2.);
        let da = std::array::from_fn::<_, 3, _>(|i| (qa[i] - pa[i]) * fa);
        let db = std::array::from_fn::<_, 3, _>(|i| (qb[i] - pb[i]) * fb);
        set(&mut first.surface, ea, seam);
        set(&mut second.surface, eb, seam);
        if matches!(continuity, Continuity::ParametricC1) {
            let mean = std::array::from_fn::<_, 3, _>(|i| (da[i] - db[i]) / 2.);
            set(
                &mut first.surface,
                ia,
                std::array::from_fn(|i| seam[i] + mean[i] / fa),
            );
            set(
                &mut second.surface,
                ib,
                std::array::from_fn(|i| seam[i] - mean[i] / fb),
            );
        }
    }
    first.surface.validate()?;
    second.surface.validate()?;
    for (old, new) in [(&a.surface, &first.surface), (&b.surface, &second.surface)] {
        for (p, q) in old
            .control_points
            .iter()
            .flatten()
            .zip(new.control_points.iter().flatten())
        {
            let distance = p.iter().zip(q).fold(0f64, |r, (p, q)| r.hypot(p - q));
            maximum = maximum.max(distance);
        }
    }
    Ok(Reconciliation {
        first,
        second,
        max_control_displacement: maximum,
        regularity_certified: false,
    })
}
