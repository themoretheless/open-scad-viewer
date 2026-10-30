//! Compare the actual original strip to the cylinder over its exact trace.
//! No nearly extruded control or nearly constant pcurve is snapped to an axis.
use super::*;
use polynomial::Homogeneous;
pub(super) struct Source<'a> {
    _source: &'a Surface,
    net: Vec<Vec<[I; 4]>>,
    uv: [I; 2],
    velocity: [I; 2],
    transverse: usize,
    pub trace: Homogeneous,
}
impl<'a> Source<'a> {
    pub fn new(s: &'a Surface, pc: &Curve, origin: [f64; 3]) -> Result<Self> {
        check(
            pc.weights[0] == pc.weights[1]
                && (1..=16).contains(&s.degree_u)
                && (1..=16).contains(&s.degree_v)
                && s.degree_u + s.degree_v <= 16,
            "Source cylindrical comparison needs an affine original clock and bounded trace degree",
        )?;
        let (mut spans, mut domain) = ([0; 2], [[0.; 2]; 2]);
        for k in 0..2 {
            let (knots, degree, n) = if k == 0 {
                (&s.knots_u, s.degree_u, s.control_points.len())
            } else {
                (&s.knots_v, s.degree_v, s.control_points[0].len())
            };
            let a = pc.control_points[0][k];
            let b = pc.control_points[1][k];
            spans[k] = (degree..n)
                .find(|&i| {
                    knots[i] < knots[i + 1] && a.min(b) >= knots[i] && a.max(b) <= knots[i + 1]
                })
                .ok_or_else(|| crate::input("Source seam must remain on one original knot cell"))?;
            domain[k] = [knots[spans[k]], knots[spans[k] + 1]];
        }
        let net =
            crate::curve_surface_composition::surface_net_on_centered(s, spans, domain, origin)?;
        let homogeneous = (0..3)
            .map(|k| {
                (0..2)
                    .map(|i| I::point(if k == 2 { 1. } else { pc.control_points[i][k] }))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let trace = crate::curve_surface_composition::lift_coefficients(
            &homogeneous,
            &net,
            [s.degree_u, s.degree_v],
            domain,
        )?;
        let trace: Homogeneous = trace
            .try_into()
            .map_err(|_| crate::input("Invalid original trace dimension"))?;
        let mut uv = [I::point(0.); 2];
        let mut velocity = uv;
        for k in 0..2 {
            let width = I::point(domain[k][1]).sub(I::point(domain[k][0]))?;
            uv[k] = I::point(pc.control_points[0][k])
                .sub(I::point(domain[k][0]))?
                .div(width)?;
            velocity[k] = I::point(pc.control_points[1][k])
                .sub(I::point(pc.control_points[0][k]))?
                .div(width)?;
        }
        let dominant = if (pc.control_points[1][0] - pc.control_points[0][0]).abs()
            >= (pc.control_points[1][1] - pc.control_points[0][1]).abs()
        {
            0
        } else {
            1
        };
        check(
            velocity[dominant].lo > 0. || velocity[dominant].hi < 0.,
            "Source trace lacks a regular affine coordinate",
        )?;
        Ok(Self {
            _source: s,
            net,
            uv,
            velocity,
            transverse: 1 - dominant,
            trace,
        })
    }
    pub fn bounds(
        &self,
        ends: [I; 2],
        t: [f64; 2],
        axis: usize,
    ) -> Result<Option<([[f64; 2]; 2], i8)>> {
        let t = I::new(t[0], t[1])?;
        let dt = ends[1].sub(ends[0])?;
        let fraction = ends[0].add(dt.mul(t)?)?.intersect(0., 1.)?;
        let mut uv = [I::point(0.); 2];
        let mut d = uv;
        for k in 0..2 {
            uv[k] = self.uv[k]
                .add(self.velocity[k].mul(fraction)?)?
                .intersect(0., 1.)?;
            d[k] = self.velocity[k].mul(dt)?;
        }
        let original = jets::evaluate_homogeneous_jets(&self.net, uv, 2)?;
        let mut a = [[[I::point(0.); 3]; 4]; 4];
        for k in 0..3 {
            a[1][0][k] = original[1][0][k]
                .mul(d[0])?
                .add(original[0][1][k].mul(d[1])?)?;
            a[2][0][k] = original[2][0][k]
                .mul(d[0])?
                .mul(d[0])?
                .add(I::point(2.).mul(original[1][1][k])?.mul(d[0])?.mul(d[1])?)?
                .add(original[0][2][k].mul(d[1])?.mul(d[1])?)?;
            if self.transverse == 1 {
                a[0][1][k] = original[0][1][k];
                a[1][1][k] = original[1][1][k]
                    .mul(d[0])?
                    .add(original[0][2][k].mul(d[1])?)?;
                a[0][2][k] = original[0][2][k];
            } else {
                a[0][1][k] = original[1][0][k];
                a[1][1][k] = original[2][0][k]
                    .mul(d[0])?
                    .add(original[1][1][k].mul(d[1])?)?;
                a[0][2][k] = original[2][0][k];
            }
        }
        let det = if self.transverse == 1 {
            d[0]
        } else {
            I::point(0.).sub(d[1])?
        };
        let sign = |x: I| {
            if x.lo > 0. {
                Some(1)
            } else if x.hi < 0. {
                Some(-1)
            } else {
                None
            }
        };
        let (Some(det_sign), Some(axial_sign)) = (sign(det), sign(a[0][1][axis])) else {
            return Ok(None);
        };
        let mut b = a;
        let mut delta = [[I::point(0.); 3]; 5];
        for (slot, (u, v)) in [(1, 0), (2, 0), (1, 1), (0, 1), (0, 2)]
            .into_iter()
            .enumerate()
        {
            for k in 0..3 {
                let remove = if v == 0 { k == axis } else { k != axis };
                if remove {
                    b[u][v][k] = I::point(0.);
                    delta[slot][k] = I::point(0.).sub(a[u][v][k])?;
                }
            }
        }
        Ok(super::super::paired::bounds_full(&a, &b, delta)?
            .map(|bounds| (bounds, det_sign * axial_sign)))
    }
}
