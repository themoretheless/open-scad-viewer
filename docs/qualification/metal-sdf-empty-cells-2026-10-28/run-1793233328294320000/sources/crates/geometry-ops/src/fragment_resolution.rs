//! Full-circle and bounded sweep subdivision shared by geometry execution and language adapters.
#[derive(Debug, Clone, Copy)]
pub enum WarningKind {
    NonFiniteFn,
    NegativeFn,
    NanFa,
    NanFs,
    SmallFa,
    SmallFs,
    NonFiniteResolution,
    Clamped,
}
#[derive(Debug, Clone, Copy)]
pub struct Warning {
    pub kind: WarningKind,
    pub value: f64,
}
impl Warning {
    pub fn message(&self, maximum: usize) -> String {
        use WarningKind::*;
        match self.kind {
            NonFiniteFn => "Non-finite $fn selected the minimum fragment count".into(),
            NegativeFn => "Negative $fn was replaced with the automatic fragment mode".into(),
            NanFa => "Non-finite $fa was replaced with its default".into(),
            NanFs => "Non-finite $fs was replaced with its default".into(),
            SmallFa => "$fa was raised to the OpenSCAD minimum 0.01".into(),
            SmallFs => "$fs was raised to the OpenSCAD minimum 0.01".into(),
            NonFiniteResolution => {
                "Non-finite fragment resolution was bounded by the engine safety limit".into()
            }
            Clamped => format!("Fragment count was clamped to the engine safety limit {maximum}"),
        }
    }
}
#[derive(Debug)]
pub struct Resolution {
    pub fragments: f64,
    pub unbounded: f64,
    pub source: &'static str,
    pub effective_fn: f64,
    pub fa: f64,
    pub fs: f64,
    pub reduced: bool,
    pub warnings: Vec<Warning>,
}
/// The caller supplies a finite integral maximum >=3. Nonfinite authored values retain JS semantics.
pub fn resolve(radius: f64, fn_: f64, mut fa: f64, mut fs: f64, maximum: f64) -> Resolution {
    use WarningKind::*;
    let mut warnings = Vec::new();
    let mut emit = |kind, value| warnings.push(Warning { kind, value });
    let nonfinite = !fn_.is_finite();
    let mut effective_fn = fn_;
    if nonfinite {
        emit(NonFiniteFn, fn_)
    } else if fn_ < 0. {
        emit(NegativeFn, fn_);
        effective_fn = 0.
    }
    if fa.is_nan() {
        emit(NanFa, fa);
        fa = 12.
    }
    if fs.is_nan() {
        emit(NanFs, fs);
        fs = 2.
    }
    if fa < 0.01 {
        emit(SmallFa, fa);
        fa = 0.01
    }
    if fs < 0.01 {
        emit(SmallFs, fs);
        fs = 0.01
    }
    let radius = if radius.is_nan() { 0. } else { radius.abs() };
    let source;
    let mut unbounded;
    if radius < 0.00000095367431640625 || nonfinite {
        source = "geometry-epsilon";
        unbounded = 3.
    } else if effective_fn > 0. {
        source = "$fn";
        unbounded = effective_fn.max(3.).trunc()
    } else {
        source = "$fa/$fs";
        let angle = 360. / fa;
        let chord = radius * 2. * std::f64::consts::PI / fs;
        // f64::min/max discard NaN; OpenSCAD's host contract propagates it.
        unbounded = if angle.is_nan() || chord.is_nan() {
            f64::NAN
        } else {
            angle.min(chord).max(5.).ceil()
        };
        if !unbounded.is_finite() {
            emit(NonFiniteResolution, unbounded);
            unbounded = maximum
        }
    }
    let fragments = unbounded.min(maximum);
    let reduced = fragments != unbounded;
    if reduced {
        emit(Clamped, unbounded)
    }
    Resolution {
        fragments,
        unbounded,
        source,
        effective_fn,
        fa,
        fs,
        reduced,
        warnings,
    }
}
pub fn sweep(fragments: f64, degrees: f64) -> (f64, f64) {
    let degrees = if degrees.is_finite() {
        degrees.abs().min(360.)
    } else {
        360.
    };
    let count = if degrees == 0. {
        0.
    } else if degrees == 360. {
        fragments
    } else {
        (fragments * degrees / 360.).floor().max(1.)
    };
    (degrees, count)
}
#[derive(Debug)]
pub struct SweepResolution {
    pub circle: Resolution,
    pub segments: usize,
}
/// Deferred sweep subdivision, independent of a language frontend.
#[derive(Debug, Clone, PartialEq)]
pub struct Policy {
    pub fragments: [f64; 3],
    pub maximum: usize,
}
impl Policy {
    pub fn validate(&self) -> crate::Result<()> {
        if !(3..=512).contains(&self.maximum) {
            return Err(crate::fail("Invalid bounded sweep maximum"));
        }
        Ok(())
    }
    pub fn resolve(&self, radius: f64, degrees: f64) -> crate::Result<SweepResolution> {
        self.validate()?;
        resolve_sweep(radius, degrees, self.fragments, self.maximum)
    }
}
#[cfg(feature = "codec")]
mod policy_codec {
    use super::*;
    use crate::codec::{decode_number, encode_number};
    use value_codec::{Deserialize, Serialize, Value, json};
    impl Serialize for Policy {
        fn to_value(&self) -> Value {
            json!({"fragments":self.fragments.map(encode_number),"maximum":self.maximum})
        }
    }
    impl<'de> Deserialize<'de> for Policy {
        fn from_value(value: Value) -> value_codec::Result<Self> {
            let mut object = crate::codec::object(value)?;
            let fragments: [Value; 3] = crate::codec::required(&mut object, "fragments")?;
            Ok(Self {
                fragments: [
                    decode_number(&fragments[0])?,
                    decode_number(&fragments[1])?,
                    decode_number(&fragments[2])?,
                ],
                maximum: crate::codec::required(&mut object, "maximum")?,
            })
        }
    }
    #[test]
    fn policy_transport_retains_nonfinite_parameters_and_rejects_bad_shape() {
        let policy = Policy {
            fragments: [f64::INFINITY, f64::NAN, f64::NEG_INFINITY],
            maximum: 128,
        };
        let decoded = Policy::from_value(policy.to_value()).unwrap();
        assert!(decoded.fragments[0].is_infinite() && decoded.fragments[0] > 0.);
        assert!(decoded.fragments[1].is_nan());
        assert!(decoded.fragments[2].is_infinite() && decoded.fragments[2] < 0.);
        assert_eq!(decoded.maximum, 128);
        assert_eq!(decoded.resolve(4., 360.).unwrap().segments, 3);
        assert!(Policy::from_value(json!({"fragments":[1,2],"maximum":128})).is_err());
        assert!(Policy::from_value(json!({"fragments":["bad",12,2],"maximum":128})).is_err());
        assert!(
            Policy {
                maximum: 2,
                ..decoded
            }
            .validate()
            .is_err()
        );
    }
}
/// Resolve an engine-bounded sweep after its profile radius is known.
pub fn resolve_sweep(
    radius: f64,
    degrees: f64,
    fragments: [f64; 3],
    maximum: usize,
) -> crate::Result<SweepResolution> {
    if !(3..=512).contains(&maximum)
        || !radius.is_finite()
        || radius < 0.
        || !degrees.is_finite()
        || degrees.abs() > 360.
    {
        return Err(crate::fail("Invalid bounded sweep resolution"));
    }
    let circle = resolve(
        radius,
        fragments[0],
        fragments[1],
        fragments[2],
        maximum as f64,
    );
    let (_, segments) = sweep(circle.fragments, degrees);
    Ok(SweepResolution {
        circle,
        segments: segments as usize,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_sweeps_preserve_limits_and_signed_angles() {
        for angle in [90., -90.] {
            let resolution = resolve_sweep(4., angle, [12., 12., 2.], 8).unwrap();
            assert_eq!(resolution.segments, 2);
            assert!(resolution.circle.reduced);
            assert!(matches!(
                resolution.circle.warnings[0].kind,
                WarningKind::Clamped
            ));
        }
        assert_eq!(
            resolve_sweep(0., 360., [100., 12., 2.], 512)
                .unwrap()
                .segments,
            3
        );
        assert_eq!(
            resolve_sweep(4., 0., [12., 12., 2.], 512).unwrap().segments,
            0
        );
        assert!(resolve_sweep(4., 90., [0., 12., 2.], 2).is_err());
        assert!(resolve_sweep(f64::NAN, 90., [0., 12., 2.], 512).is_err());
    }
    #[test]
    fn formulas_and_nonfinite_rules() {
        assert_eq!(resolve(10., 0., 12., 2., 256.).fragments, 30.);
        assert_eq!(resolve(10., 10.9, 12., 2., 256.).fragments, 10.);
        assert_eq!(resolve(f64::NAN, 100., 12., 2., 256.).fragments, 3.);
        let r = resolve(f64::INFINITY, 0., 12., f64::INFINITY, 256.);
        assert_eq!(r.fragments, 256.);
        assert!(matches!(
            r.warnings[0].kind,
            WarningKind::NonFiniteResolution
        ));
        assert_eq!(resolve(10., f64::INFINITY, 12., 2., 256.).fragments, 3.);
        assert_eq!(sweep(30., 90.), (90., 7.));
        assert_eq!(sweep(30., 0.), (0., 0.));
        assert_eq!(sweep(30., f64::NAN), (360., 30.));
    }
}
