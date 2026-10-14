//! One closed prismatic rack body from the native exact polygonal NURBS profile.
use crate::Model;
use nurbs_core::{
    Result,
    curve::Curve,
    rack::{self, Profile, Spec},
};
#[derive(Clone, Debug)]
pub struct Rack {
    pub model: Model,
    pub profile: Profile,
    pub z_bounds: [f64; 2],
}
/// Extrude the authored rack along Z. Uses the ordinary prism topology path,
/// with one body and two planar caps; no union of overlapping tooth solids.
/// The prism's height, coordinate and topology resource limits apply.
pub fn extrude(spec: Spec, z_min: f64, z_max: f64) -> Result<Rack> {
    let profile = rack::profile(spec)?;
    let wire = profile
        .boundary
        .control_points
        .windows(2)
        .map(|pair| Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![pair[0][..2].to_vec(), pair[1][..2].to_vec()],
            weights: vec![1., 1.],
            periodic: false,
        })
        .collect();
    let model = crate::prism::extrude(&[wire], z_min, z_max)?;
    Ok(Rack {
        model,
        profile,
        z_bounds: [z_min, z_max],
    })
}
