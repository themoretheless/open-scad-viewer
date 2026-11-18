//! Conservative local surface-contact classification.
//!
//! Parallel normals detect a tangency candidate, not its parity. Odd/even
//! material behavior additionally requires signed side intervals on both charts.
use crate::intersection::{ContactClass, cross3, dot3, norm3};
use crate::{Error, Result, surface::Surface};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactSideEvidence {
    /// Signed first-surface offset interval over the candidate contact branch.
    pub first: [f64; 2],
    /// Signed second-surface offset interval over the same branch.
    pub second: [f64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceContactReport {
    pub class: ContactClass,
    /// Absolute cosine of the two regular unit normals.
    pub normal_alignment: f64,
    pub parity_proven: bool,
}

fn one_sided(interval: [f64; 2]) -> bool {
    interval[0] > 0. || interval[1] < 0.
}

fn crossing(interval: [f64; 2]) -> bool {
    interval[0] < 0. && interval[1] > 0.
}

pub fn classify_surface_contact(
    first: &Surface,
    first_uv: [f64; 2],
    second: &Surface,
    second_uv: [f64; 2],
    linear_tolerance: f64,
    angular_tolerance: f64,
    side_evidence: Option<ContactSideEvidence>,
) -> Result<SurfaceContactReport> {
    if !linear_tolerance.is_finite()
        || linear_tolerance <= 0.
        || !angular_tolerance.is_finite()
        || angular_tolerance <= 0.
        || side_evidence.is_some_and(|e| {
            e.first
                .iter()
                .chain(e.second.iter())
                .any(|v| !v.is_finite() || e.first[0] > e.first[1] || e.second[0] > e.second[1])
        })
    {
        return Err(Error::new(
            "NURBS_SURFACE_CONTACT_INPUT",
            "Contact classification needs finite positive tolerances and ordered finite side intervals",
        ));
    }
    let a = first.evaluate(first_uv[0], first_uv[1])?;
    let b = second.evaluate(second_uv[0], second_uv[1])?;
    let (Some((dua, dva)), Some((dub, dvb))) =
        (a.first_derivatives(), b.first_derivatives())
    else {
        return Ok(SurfaceContactReport {
            class: ContactClass::PoleOrSingular,
            normal_alignment: 0.,
            parity_proven: false,
        });
    };
    let na = cross3(dua, dva);
    let nb = cross3(dub, dvb);
    let la = norm3(na);
    let lb = norm3(nb);
    if !la.is_finite() || !lb.is_finite() || la <= linear_tolerance || lb <= linear_tolerance {
        return Ok(SurfaceContactReport {
            class: ContactClass::PoleOrSingular,
            normal_alignment: 0.,
            parity_proven: false,
        });
    }
    let na = na.map(|v| v / la);
    let nb = nb.map(|v| v / lb);
    let alignment = dot3(na, nb).abs();
    let sine = norm3(cross3(na, nb));
    if sine > angular_tolerance {
        return Ok(SurfaceContactReport {
            class: ContactClass::Transverse,
            normal_alignment: alignment,
            parity_proven: false,
        });
    }
    let Some(evidence) = side_evidence else {
        return Ok(SurfaceContactReport {
            class: ContactClass::HigherOrderContact,
            normal_alignment: alignment,
            parity_proven: false,
        });
    };
    let class = if one_sided(evidence.first) && one_sided(evidence.second) {
        ContactClass::EvenTangency
    } else if crossing(evidence.first) && crossing(evidence.second) {
        ContactClass::OddTangency
    } else {
        ContactClass::HigherOrderContact
    };
    Ok(SurfaceContactReport {
        class,
        normal_alignment: alignment,
        parity_proven: class != ContactClass::HigherOrderContact,
    })
}
