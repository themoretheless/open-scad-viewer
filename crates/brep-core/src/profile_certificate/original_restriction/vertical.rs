//! Exact regular one-to-one coverage of a native vertical world edge.
use super::*;
use crate::profile_certificate::arithmetic as a;
#[derive(Clone, Debug)]
pub struct Vertical {
    source: Curve,
    xy: [ExactRatio; 2],
    height: [ExactRatio; 2],
    endpoints: [[ExactRatio; 3]; 2],
    spans: usize,
}
pub fn qualify(source: &Curve, xy: [ExactRatio; 2], height: [ExactRatio; 2]) -> Result<Vertical> {
    source.validate()?;
    let point = [decode_ratio(&xy[0])?, decode_ratio(&xy[1])?];
    let h = [decode_ratio(&height[0])?, decode_ratio(&height[1])?];
    if h[0] >= h[1]
        || source.control_points.iter().any(|p| {
            p.len() != 3 || (0..2).any(|k| Q::from_float(p[k]).as_ref() != Some(&point[k]))
        })
    {
        return Err(refused(
            "Native vertical edge differs from exact side position",
        ));
    }
    let mut z = source.clone();
    z.control_points = source
        .control_points
        .iter()
        .map(|p| vec![p[2], 0.])
        .collect();
    let restriction = restrict_binary64(&z, z.domain())?;
    let domain = restriction.parameter_range();
    let first = restriction.evaluate_natural(domain[0].clone())?;
    let last = restriction.evaluate_natural(domain[1].clone())?;
    let endpoints_z = [decode_ratio(&first[0])?, decode_ratio(&last[0])?];
    let forward = endpoints_z == h;
    if !forward && endpoints_z != [h[1].clone(), h[0].clone()] {
        return Err(refused(
            "Native vertical endpoints do not cover exact height",
        ));
    }
    for span in &restriction.spans {
        let w = a::power(&span.net, 2);
        let z = a::power(&span.net, 0);
        let mut derivative = a::sub(
            &a::product(&a::derivative(&z), &w),
            &a::product(&z, &a::derivative(&w)),
        );
        if !forward {
            derivative.iter_mut().for_each(|v| *v = -v.clone());
        }
        if a::at(&derivative, &Q::zero()) <= Q::zero()
            || a::at(&derivative, &Q::one()) <= Q::zero()
            || a::algebraic_events::count(&derivative, &Q::zero(), &Q::one())? > 0
        {
            return Err(refused(
                "Native vertical edge is stationary or reverses height",
            ));
        }
    }
    let endpoints = endpoints_z.map(|z| [xy[0].clone(), xy[1].clone(), ratio(&z)]);
    Ok(Vertical {
        source: source.clone(),
        xy,
        height,
        endpoints,
        spans: restriction.spans.len(),
    })
}
impl Vertical {
    pub fn source(&self) -> &Curve {
        &self.source
    }
    pub fn endpoints(&self) -> &[[ExactRatio; 3]; 2] {
        &self.endpoints
    }
    pub fn span_count(&self) -> usize {
        self.spans
    }
    pub fn reverify(&self) -> Result<()> {
        let current = qualify(&self.source, self.xy.clone(), self.height.clone())?;
        if current.endpoints != self.endpoints || current.spans != self.spans {
            return Err(refused("Native vertical world-edge replay changed"));
        }
        Ok(())
    }
}
