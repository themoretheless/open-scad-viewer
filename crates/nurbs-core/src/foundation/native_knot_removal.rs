use super::{Curve, Result, ToleranceContext, check, refit_curve};
pub struct KnotRemoval { pub curve:Curve, pub accepted:bool }
pub fn remove_curve_knot_report(source:&Curve,knot:f64,max_error:f64,_tolerance:Option<ToleranceContext>)->Result<KnotRemoval> {
 source.validate()?;
 check(!source.periodic,"Certified removal currently requires non-periodic storage")?;
 check(max_error.is_finite() && max_error>=0.,"Maximum error must be finite and nonnegative")?;
 let domain=source.domain();check(knot>domain[0] && knot<domain[1],"Only interior knots can be removed")?;
 let index=source.knots.iter().position(|k|*k==knot).ok_or_else(||crate::input("Knot is absent"))?;
 let mut knots=source.knots.clone();knots.remove(index);
 let candidate=refit_curve(source,source.degree,knots)?;
 let report=crate::curve_deviation::inspect(source,&candidate,(max_error*0.01).max(1e-12),4096)?;
 let accepted=report.bounds[1]<=max_error;
 Ok(KnotRemoval{curve:if accepted {candidate} else {source.clone()},accepted})
}
