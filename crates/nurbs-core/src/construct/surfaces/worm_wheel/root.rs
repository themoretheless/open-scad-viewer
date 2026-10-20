//! Original convex crest-edge and crest-cylinder envelopes at the wheel root.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootPart {
    /// Move the original crest edge from cylinder contact to flank contact.
    CrestEdge(Flank),
    /// Original crest cylinder between ascending and next-period descending edges.
    CrestCylinder,
}

/// Injectivity of the ORIGINAL chart. This does not qualify a saved NURBS grid.
#[derive(Clone, Debug, PartialEq)]
pub enum RootEmbedding {
    /// Y fixes beta; q^2 + t^2*Z_flank^2 strictly increases with t in [0,1].
    AxialAndStrictRadius,
    /// Y fixes beta; the remaining circle arc is monotone and shorter than 2*pi.
    AxialAndShortArc { angular_span: [f64; 2] },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RootContactCertificate {
    pub crest_radius: [f64; 2],
    pub cosine: [f64; 2],
    /// Derivative of the crest-cylinder normal velocity with respect to axial Z.
    pub cylinder_contact_derivative: [f64; 2],
    /// V_cylinder * V_outward_flank = factor * t * (1-t).
    /// Strictly negative for a crest-edge chart; not applicable to the cylinder.
    pub normal_cone_product_factor: Option<[f64; 2]>,
    /// Whole-chart lower bound for |dP/dt cross dP/dbeta|.
    pub cross_norm_lower: f64,
    pub source_embedding: RootEmbedding,
}

#[derive(Clone, Debug)]
pub struct RootChartBounds {
    pub point: [[f64; 2]; 3],
    pub dt: [[f64; 2]; 3],
    pub dbeta: [[f64; 2]; 3],
    /// Univariate sixth Taylor coefficients, f^(6)/6!, for t and beta.
    pub sixth: [[[f64; 2]; 3]; 2],
    pub regularity_proven: bool,
}

/// Immutable original-source contact proof; no fitted surface or closed wheel.
#[derive(Clone, Debug)]
pub struct RootChart {
    spec: Spec,
    part: RootPart,
    beta: [f64; 2],
    certificate: RootContactCertificate,
}
impl RootChart {
    pub fn specification(&self) -> Spec {
        self.spec
    }
    pub fn part(&self) -> RootPart {
        self.part
    }
    pub fn domain(&self) -> ChartDomain {
        ChartDomain {
            u: [0., 1.],
            beta: self.beta,
        }
    }
    pub fn certificate(&self) -> &RootContactCertificate {
        &self.certificate
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = root_chart(self.spec, self.part, self.beta)?;
        check(
            fresh.certificate == self.certificate,
            "Original root contact certificate changed",
        )
    }
    pub(super) fn validate_query(&self, domain: ChartDomain) -> Result<()> {
        check(
            domain.u.iter().chain(&domain.beta).all(|x| x.is_finite())
                && 0. <= domain.u[0]
                && domain.u[0] <= domain.u[1]
                && domain.u[1] <= 1.
                && self.beta[0] <= domain.beta[0]
                && domain.beta[0] <= domain.beta[1]
                && domain.beta[1] <= self.beta[1],
            "Root query must stay inside the original local t/beta domain",
        )
    }
    pub fn position_bounds(&self, domain: ChartDomain) -> Result<[[f64; 2]; 3]> {
        self.validate_query(domain)?;
        Ok(super::chart::root_jets(self.spec, self.part, domain)?
            .0
            .map(pair))
    }
    /// Original positions/jets and continuous sixth derivatives. Regularity is
    /// independently sufficient; contact qualification does not imply embedding.
    pub fn enclose(&self, domain: ChartDomain) -> Result<RootChartBounds> {
        self.validate_query(domain)?;
        let (point, dt, db) = super::chart::root_jets(self.spec, self.part, domain)?;
        let cross = [
            dt[1].mul(db[2])?.sub(dt[2].mul(db[1])?)?,
            dt[2].mul(db[0])?.sub(dt[0].mul(db[2])?)?,
            dt[0].mul(db[1])?.sub(dt[1].mul(db[0])?)?,
        ];
        Ok(RootChartBounds {
            point: point.map(pair),
            dt: dt.map(pair),
            dbeta: db.map(pair),
            sixth: [
                super::taylor::root_sixth(self.spec, self.part, domain, 0)?.map(pair),
                super::taylor::root_sixth(self.spec, self.part, domain, 1)?.map(pair),
            ],
            regularity_proven: self.certificate.cross_norm_lower > 0.
                || cross.iter().any(|x| x.lo > 0. || x.hi < 0.),
        })
    }
}

/// Proves local generated contact for the complete requested angle interval.
/// Convex crest edges use the original outward normal cone, not an arbitrary
/// loft. Finite cutter coverage, global visibility and wheel ownership remain
/// separate. Angle/parameter/ZA restrictions match the original envelope chart.
pub fn root_chart(spec: Spec, part: RootPart, beta: [f64; 2]) -> Result<RootChart> {
    let flank = match part {
        RootPart::CrestEdge(f) => f,
        RootPart::CrestCylinder => Flank::Descending,
    };
    super::chart::validate(spec, flank, ChartDomain { u: [0., 1.], beta })?;
    let p = worm::derive_parameters(spec.worm)?;
    let rc = iv(p.crest_radius)?;
    let rp = iv(p.pitch_radius)?;
    let (sn, cs) = super::expression::contact_trig_range(iv(beta)?)?;
    check(
        cs.lo > 0.,
        "Root cylinder contact requires positive contact-angle cosine",
    )?;
    let hand = if spec.worm.hand == worm::Hand::Right {
        1.
    } else {
        -1.
    };
    let eta = I::point(-hand * spec.worm.starts as f64).div(I::point(spec.teeth as f64))?;
    let contact_derivative = neg(eta.mul(cs)?)?;
    check(
        contact_derivative.lo > 0. || contact_derivative.hi < 0.,
        "Root cylinder meshing derivative is not separated from zero",
    )?;
    let rw = I::point(spec.worm.axial_module)
        .mul(I::point(spec.teeth as f64))?
        .div(I::point(2.))?;
    let (factor, cross_norm_lower, source_embedding) = match part {
        RootPart::CrestCylinder => {
            let q_abs = rp.add(rw)?.sub(rc.mul(cs)?)?;
            check(q_abs.lo > 0., "Root cylinder reaches the wheel axis")?;
            let lower = q_abs
                .mul(iv(p.crest_width)?)?
                .div(rw)?
                .mul(rc)?
                .mul(cs)?
                .lo
                .max(0.);
            let span = iv(p.crest_width)?.div(rw)?;
            let full_turn = crate::involute::certified_pi()?.mul(I::point(2.))?;
            check(
                span.lo > 0. && span.hi < full_turn.lo,
                "Original root cylinder arc does not prove single coverage",
            )?;
            (
                None,
                lower,
                RootEmbedding::AxialAndShortArc {
                    angular_span: pair(span),
                },
            )
        }
        RootPart::CrestEdge(f) => {
            let gap = rc.mul(cs)?.sub(rp)?;
            check(
                gap.lo > 0.,
                "Original crest edge radial gap is not positive throughout the angle domain",
            )?;
            let (s, c) = crate::involute::certified_trig(I::point(spec.worm.pressure_angle))?;
            let sign = if f == Flank::Descending { 1. } else { -1. };
            let k = I::point(-sign).mul(s.div(c)?)?;
            let h = I::point(hand)
                .mul(I::point(spec.worm.axial_module))?
                .mul(I::point(spec.worm.starts as f64))?
                .div(I::point(2.))?;
            let denominator = k.mul(cs)?.sub(h.div(rc)?.mul(sn)?)?;
            check(
                I::point(sign).mul(denominator)?.hi < 0.,
                "Original convex crest-edge normal cone is not qualified over the angle domain",
            )?;
            let factor = eta
                .mul(eta)?
                .mul(rc)?
                .mul(cs)?
                .mul(gap.mul(gap)?)?
                .mul(I::point(sign))?
                .div_signed(denominator)?;
            check(
                factor.hi < 0.,
                "Crest-edge normal velocities are not proved opposite",
            )?;
            let z = neg(gap)?.div_signed(denominator)?;
            let abs_z = if z.lo > 0. { z } else { neg(z)? };
            // dP/dt = (Z_flank/rw) R_psi[-Z,0,gap], and
            // dP_y/dbeta=rc*cos(beta), dP_y/dt=0. Orthogonal
            // rotation preserves the XZ norm, so this proves rank directly
            // without subtracting large, dependent interval jet terms.
            let lower = abs_z.mul(gap)?.div(rw)?.mul(rc)?.mul(cs)?.lo.max(0.);
            (
                Some(pair(factor)),
                lower,
                RootEmbedding::AxialAndStrictRadius,
            )
        }
    };
    Ok(RootChart {
        spec,
        part,
        beta,
        certificate: RootContactCertificate {
            crest_radius: pair(rc),
            cosine: pair(cs),
            cylinder_contact_derivative: pair(contact_derivative),
            normal_cone_product_factor: factor,
            cross_norm_lower,
            source_embedding,
        },
    })
}
