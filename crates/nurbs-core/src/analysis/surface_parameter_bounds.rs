//! Continuous derivative norm bounds on all original knot rectangles.
use crate::{
    Result, check,
    distance_bounds::{Interval, box_distance},
    surface::Surface,
    surface_measure::jets,
};
#[derive(Clone, Debug)]
pub struct Bounds {
    pub derivative_norm_upper_bounds: [f64; 2],
    pub rectangles: usize,
}
/// Full-domain coordinatewise Lipschitz bound, including both one-sided jets
/// at C0 knots. No regularity, injectivity or periodic seam claim is made.
pub fn inspect(surface: &Surface, max_rectangles: usize) -> Result<Bounds> {
    surface.validate()?;
    check(
        (1..=10000).contains(&max_rectangles),
        "Parameter bound budget must be 1..10000 rectangles",
    )?;
    let spans = |degree: usize, knots: &[f64], count: usize| {
        (degree..count)
            .filter(|&i| knots[i] < knots[i + 1])
            .collect::<Vec<_>>()
    };
    let u = spans(
        surface.degree_u,
        &surface.knots_u,
        surface.control_points.len(),
    );
    let v = spans(
        surface.degree_v,
        &surface.knots_v,
        surface.control_points[0].len(),
    );
    let rectangles = u.len() * v.len();
    if rectangles > max_rectangles {
        return Err(crate::resource(
            "Full surface parameter bounds exceed the rectangle budget",
        ));
    }
    let mut upper = [0_f64; 2];
    for &i in &u {
        for &j in &v {
            let domain = [
                [surface.knots_u[i], surface.knots_u[i + 1]],
                [surface.knots_v[j], surface.knots_v[j + 1]],
            ];
            let jet = jets::calculate_partial_stable(surface, [i, j], domain, [None; 2])?;
            upper[0] = upper[0].max(box_distance(&jet[1][0], &[Interval::point(0.); 3])?.1);
            upper[1] = upper[1].max(box_distance(&jet[0][1], &[Interval::point(0.); 3])?.1);
        }
    }
    Ok(Bounds {
        derivative_norm_upper_bounds: upper,
        rectangles,
    })
}
