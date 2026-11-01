//! Material-side orientation of complete exact smooth original carriers.
//! Plane/jet equality alone can admit a folded cusp. This adds the signed
//! material normal premise; face interiors and vertex graph disks remain separate.
use super::{Carrier, Class, ExactSmoothCollection};
use nurbs_core::{Error, Result, interval_eval::Interval as I};

mod anchor;
pub(crate) use anchor::inspect as inspect_anchor;

#[derive(Clone, Copy)]
pub struct Limits {
    pub steps: usize,
    pub curve_spans: usize,
    pub normal_spans: usize,
}
pub struct OrientedCollection<'e, 'c, 'b, E: Carrier> {
    exact: &'e ExactSmoothCollection<'c, 'b, E>,
    edges: Vec<usize>,
}
impl<'e, 'c, 'b, E: Carrier> OrientedCollection<'e, 'c, 'b, E> {
    pub fn exact(&self) -> &'e ExactSmoothCollection<'c, 'b, E> {
        self.exact
    }
    pub fn edges(&self) -> &[usize] {
        &self.edges
    }
}
pub struct Report<'e, 'c, 'b, E: Carrier> {
    pub certificate: Option<OrientedCollection<'e, 'c, 'b, E>>,
    pub steps: usize,
    pub curve_spans: usize,
    pub normal_spans: usize,
    pub completed_smooth_edges: usize,
    pub uncertain_edge: Option<usize>,
    pub reason: &'static str,
}
fn interior(a: I, b: I) -> Option<f64> {
    let gap = if a.hi < b.lo {
        [a.hi, b.lo]
    } else if b.hi < a.lo {
        [b.hi, a.lo]
    } else {
        return None;
    };
    let anchor = gap[0] * 0.5 + gap[1] * 0.5;
    (gap[0] < anchor && anchor < gap[1]).then_some(anchor)
}

/// Full exact identities and nonzero continuous normal/limit coverage are
/// retained by `exact`. Their relative normal sign is therefore constant on
/// the connected carrier. ONE certified interior anchor selects that discrete
/// sign; it provides no sampled equality or sampled regularity authority.
/// Each chart normal is signed by its freshly admitted material winding. Both
/// signs flip on global body reversal, so their relative sign is unchanged.
pub fn qualify<'e, 'c, 'b, E: Carrier>(
    exact: &'e ExactSmoothCollection<'c, 'b, E>,
    limits: Limits,
) -> Result<Report<'e, 'c, 'b, E>> {
    if limits.steps > 1000000 || limits.curve_spans > 100000 || limits.normal_spans > 100000 {
        return Err(Error::new(
            "BREP_SOURCE_MATERIAL_ORIENTATION",
            "Choose bounded original orientation work",
        ));
    }
    let original = exact.original();
    let shell = original.body().geometry().shell();
    let regions = shell.regions().ok_or_else(|| {
        Error::new(
            "BREP_SOURCE_MATERIAL_ORIENTATION",
            "Original material regions required",
        )
    })?;
    let mut out = Report {
        certificate: None,
        steps: 0,
        curve_spans: 0,
        normal_spans: 0,
        completed_smooth_edges: 0,
        uncertain_edge: None,
        reason: "source-material-orientation-unproven",
    };
    let mut edges = Vec::new();
    for (index, class) in original.classes().iter().enumerate() {
        out.uncertain_edge = Some(index);
        if out.steps == limits.steps {
            out.reason = "source-material-orientation-work-limit";
            return Ok(out);
        }
        out.steps += 1;
        if *class == Class::C0 {
            continue;
        }
        let edge = &shell.edges()[index];
        let pair = shell.uses()[index];
        let sign = regions[pair[0].face].chart_winding() * regions[pair[1].face].chart_winding();
        let result = inspect_anchor(
            edge,
            sign,
            Limits {
                steps: 0,
                curve_spans: limits.curve_spans - out.curve_spans,
                normal_spans: limits.normal_spans - out.normal_spans,
            },
        )?;
        out.curve_spans += result.curve_spans;
        out.normal_spans += result.normal_spans;
        if !result.positive {
            out.reason = result.reason;
            return Ok(out);
        }
        edges.push(index);
        out.completed_smooth_edges += 1;
    }
    out.uncertain_edge = None;
    out.reason = "source-exact-smooth-material-orientation-qualified";
    out.certificate = Some(OrientedCollection { exact, edges });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn root_trim_anchor_stays_strictly_inside_owned_endpoint_enclosures() {
        let a = I::new(0.7, 0.75).unwrap();
        let b = I::new(0.9, 0.95).unwrap();
        let t = interior(a, b).unwrap();
        assert!(a.hi < t && t < b.lo && t > 0.5);
        assert_eq!(interior(b, a), Some(t));
        assert!(interior(I::new(0., 0.6).unwrap(), I::new(0.4, 1.).unwrap()).is_none());
        let next = f64::from_bits(1_f64.to_bits() + 1);
        assert!(interior(I::point(1.), I::point(next)).is_none());
    }
}
