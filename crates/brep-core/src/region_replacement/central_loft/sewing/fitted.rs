//! Structured generalized-cylinder strips with literal shared rational profiles.
use super::*;
use topology::{edge, endpoint, vertex};
pub(super) struct Patch {
    pub model: Model,
    pub near: usize,
    pub far: usize,
    pub contacts: [usize; 2],
}
pub(super) fn prepare(
    mut profile: Curve,
    original: &Curve,
    reverse: bool,
    axis: usize,
    fixed: f64,
    tolerance: f64,
) -> Result<Patch> {
    if reverse {
        profile = profile.reverse()?;
    }
    for end in 0..2 {
        let a = endpoint(&profile, end);
        let b = endpoint(original, end);
        require(
            crate::distance(&a, &b) <= tolerance,
            "Fitted profile changed its owned endpoint beyond tolerance",
        )?;
    }
    let d = profile.domain();
    require(
        profile.knots.iter().all(|u| u.fract() == 0.) && d[0] >= 0. && d[1] <= 256.,
        "Structured profile reversal needs exact integer clocks",
    )?;
    let mut points = Vec::new();
    for p in &profile.control_points {
        let mut middle = p.clone();
        let mut far = p.clone();
        middle[axis] = (p[axis] + fixed) * 0.5;
        far[axis] = fixed;
        require(
            (p[axis] < middle[axis] && middle[axis] < fixed)
                || (p[axis] > middle[axis] && middle[axis] > fixed),
            "Structured strip station controls are not strictly monotone",
        )?;
        points.push(vec![p.clone(), middle, far]);
    }
    let surface = Surface {
        degree_u: profile.degree,
        degree_v: 2,
        knots_u: profile.knots.clone(),
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: points,
        weights: profile.weights.iter().map(|&w| vec![w; 3]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    let far = Curve {
        control_points: surface
            .control_points
            .iter()
            .map(|r| r[2].clone())
            .collect(),
        ..profile.clone()
    };
    let contact = |i: usize| Curve {
        degree: 2,
        knots: surface.knots_v.clone(),
        control_points: surface.control_points[i].clone(),
        weights: surface.weights[i].clone(),
        periodic: false,
    };
    let contacts = [contact(0), contact(surface.control_points.len() - 1)];
    let mut m = Model::empty(tolerance)?;
    let a = vertex(&mut m, endpoint(&profile, 0));
    let b = vertex(&mut m, endpoint(&profile, 1));
    let c = vertex(&mut m, endpoint(&far, 1));
    let e = vertex(&mut m, endpoint(&far, 0));
    let near = edge(&mut m, profile.clone(), [a, b], false);
    let right = edge(&mut m, contacts[1].clone(), [b, c], false);
    let far = edge(&mut m, far, [e, c], false);
    let left = edge(&mut m, contacts[0].clone(), [a, e], false);
    m.loops.push(crate::Loop {
        coedges: vec![
            crate::Coedge::new(
                near,
                false,
                Curve::from_polyline(vec![vec![d[0], 0.], vec![d[1], 0.]])?,
            ),
            crate::Coedge::new(
                right,
                false,
                Curve::from_polyline(vec![vec![d[1], 0.], vec![d[1], 1.]])?,
            ),
            crate::Coedge::new(
                far,
                true,
                Curve::from_polyline(vec![vec![d[1], 1.], vec![d[0], 1.]])?,
            ),
            crate::Coedge::new(
                left,
                true,
                Curve::from_polyline(vec![vec![d[0], 1.], vec![d[0], 0.]])?,
            ),
        ],
    });
    m.faces.push(crate::Face {
        surface,
        outer: 0,
        holes: vec![],
    });
    m.shells.push(crate::Shell {
        faces: vec![crate::FaceUse {
            face: 0,
            reversed: false,
        }],
        closed: false,
    });
    m.rebuild_topology_ids();
    m.validate()?;
    Ok(Patch {
        model: m,
        near,
        far,
        contacts: [left, right],
    })
}
