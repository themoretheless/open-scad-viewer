//! Rational spine transition proposals with authored neighbouring support jets.
//! Fixed cross-section directions, not a Frenet sweep or arbitrary face trimming.
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};

pub struct NurbsEdgeTransition {
    pub blend: Surface,
    /// Support 0 meets blend U=0 at its U=1; support 1 meets U=1 at U=0.
    pub supports: [Surface; 2],
}
fn require(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::new("BREP_NURBS_EDGE_PROPOSAL", message))
    }
}
/// Preserve the original rational V clock/weights. Contact extents are scalar
/// poles in the spine basis (not interpolated physical curvature radii).
/// The two neighbouring supports are rebuilt with the transition. Endpoint
/// cross-section second jets vanish, allowing independent whole-seam G2 checks.
/// No curvature, injectivity, global embedding or solid admission is inherited.
pub fn propose_nurbs_edge_transition(
    spine: &Curve,
    contact_extents: &[f64],
    directions: [[f64; 3]; 2],
    handle_fraction: f64,
) -> Result<NurbsEdgeTransition> {
    spine.validate()?;
    let n = spine.control_points.len();
    require(
        !spine.periodic
            && spine.degree > 0
            && spine.degree <= 8
            && n <= 256
            && spine.control_points.iter().all(|p| p.len() == 3)
            && spine.weights.iter().all(|w| w.is_finite() && *w > 0.)
            && contact_extents.len() == n
            && contact_extents
                .iter()
                .all(|r| r.is_finite() && *r > 0. && *r <= 1e6)
            && directions
                .iter()
                .flatten()
                .all(|v| v.is_finite() && v.abs() <= 1e6)
            && handle_fraction.is_finite()
            && (0.1..=0.3).contains(&handle_fraction),
        "Invalid spine, positive extents, directions or handle",
    )?;
    let cross: [f64; 3] = std::array::from_fn(|k| {
        directions[0][(k + 1) % 3] * directions[1][(k + 2) % 3]
            - directions[0][(k + 2) % 3] * directions[1][(k + 1) % 3]
    });
    require(
        cross.iter().any(|v| v.abs() > 1e-12),
        "Dependent cross-section directions",
    )?;
    let domain = spine.domain();
    require(
        spine.knots[..=spine.degree].iter().all(|v| *v == domain[0])
            && spine.knots[n..].iter().all(|v| *v == domain[1]),
        "Clamped nonperiodic spine required",
    )?;
    // Interior C0/C1 knot lines cannot silently become a whole-length G2 claim.
    let mut i = spine.degree + 1;
    while i < n {
        let mut j = i + 1;
        while j < n && spine.knots[j] == spine.knots[i] {
            j += 1;
        }
        require(
            spine.degree >= 3 && j - i <= spine.degree - 2,
            "Spine must be C2 at every interior knot",
        )?;
        i = j;
    }
    let h = handle_fraction * 2f64.sqrt();
    let f = [0., 0., 0., 1. - 2. * h, 1. - h, 1.];
    let mut cp = vec![vec![vec![0.; 3]; n]; 6];
    for u in 0..6 {
        for v in 0..n {
            for k in 0..3 {
                cp[u][v][k] = spine.control_points[v][k]
                    + contact_extents[v] * (f[5 - u] * directions[0][k] + f[u] * directions[1][k]);
            }
        }
    }
    let surface = |points: Vec<Vec<Vec<f64>>>| -> Result<Surface> {
        let degree = points.len() - 1;
        let out = Surface {
            degree_u: degree,
            degree_v: spine.degree,
            knots_u: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
            knots_v: spine.knots.clone(),
            weights: vec![spine.weights.clone(); degree + 1],
            control_points: points,
            periodic_u: false,
            periodic_v: false,
        };
        out.validate()?;
        Ok(out)
    };
    // Derive support boundary poles literally from the blend; never re-evaluate
    // rational endpoints or choose neighbouring faces by spatial proximity.
    let mut left = cp[0].clone();
    let mut right = cp[5].clone();
    for v in 0..n {
        for k in 0..3 {
            left[v][k] += contact_extents[v] * directions[0][k];
            right[v][k] += contact_extents[v] * directions[1][k];
        }
    }
    let supports = [
        surface(vec![left, cp[0].clone()])?,
        surface(vec![cp[5].clone(), right])?,
    ];
    Ok(NurbsEdgeTransition {
        blend: surface(cp)?,
        supports,
    })
}
