//! STEP length-unit resolution, including bounded conversion chains.
use super::*;
use crate::step_part21::refs;
fn si_scale(args: &[Value]) -> Result<f64> {
    if !args
        .iter()
        .any(|v| matches!(v, Value::Enum(e) if e == "METRE"))
    {
        return Err(refuse("SI unit is not a length unit"));
    }
    let prefix = args.iter().find_map(|v| {
        if let Value::Enum(e) = v {
            (e != "METRE").then_some(e.as_str())
        } else {
            None
        }
    });
    Ok(match prefix {
        None => 1000.,
        Some("DECI") => 100.,
        Some("CENTI") => 10.,
        Some("MILLI") => 1.,
        Some("MICRO") => 1e-3,
        Some("NANO") => 1e-6,
        Some("KILO") => 1e6,
        _ => return Err(refuse("Unsupported SI length prefix")),
    })
}
pub(super) fn unit_scale(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    depth: usize,
    active: &mut BTreeSet<usize>,
) -> Result<f64> {
    if depth > 8 {
        return Err(refuse("CONVERSION_BASED_UNIT chain depth exceeds eight"));
    }
    if !active.insert(id) {
        return Err(refuse("Cycle in CONVERSION_BASED_UNIT chain"));
    }
    let entity = entities
        .get(&id)
        .ok_or_else(|| refuse("Missing unit reference"))?;
    let result = match &entity.value {
        Value::List(parts) => {
            let mut answer = None;
            for p in parts {
                if let Value::Call(n, a) = p {
                    if n == "SI_UNIT" {
                        answer = Some(si_scale(a)?)
                    } else if n == "CONVERSION_BASED_UNIT" {
                        answer = Some(conversion_scale(entities, a, depth, active)?)
                    }
                }
            }
            answer.ok_or_else(|| refuse("Complex unit has no admitted length component"))?
        }
        Value::Call(n, a) if n == "SI_UNIT" => si_scale(a)?,
        Value::Call(n, a) if n == "CONVERSION_BASED_UNIT" => {
            conversion_scale(entities, a, depth, active)?
        }
        _ => return Err(refuse("Unsupported length unit entity")),
    };
    active.remove(&id);
    Ok(result)
}
fn conversion_scale(
    entities: &BTreeMap<usize, Entity>,
    args: &[Value],
    depth: usize,
    active: &mut BTreeSet<usize>,
) -> Result<f64> {
    if args.len() < 2 {
        return Err(refuse("CONVERSION_BASED_UNIT argument count mismatch"));
    }
    let factor_id = one_ref(args.last().unwrap(), "CONVERSION_BASED_UNIT factor")?;
    let (ty, a) = call(entities, factor_id)?;
    if !matches!(ty, "LENGTH_MEASURE_WITH_UNIT" | "MEASURE_WITH_UNIT") || a.len() != 2 {
        return Err(refuse("Conversion factor must be LENGTH_MEASURE_WITH_UNIT"));
    }
    let factor = match &a[0] {
        Value::Call(n, v) if n == "LENGTH_MEASURE" && v.len() == 1 => {
            number(&v[0], "LENGTH_MEASURE")?
        }
        v => number(v, "conversion factor")?,
    };
    if !(factor > 0. && factor <= 1e9) {
        return Err(refuse("Conversion factor must be positive and bounded"));
    }
    Ok(factor
        * unit_scale(
            entities,
            one_ref(&a[1], "conversion base unit")?,
            depth + 1,
            active,
        )?)
}
pub(super) fn length_scale(
    entities: &BTreeMap<usize, Entity>,
    reachable: &BTreeSet<usize>,
    allow_orphan_fallback: bool,
) -> Result<f64> {
    let mut unit_ids = Vec::new();
    for id in reachable {
        let entity = &entities[id];
        match &entity.value {
            Value::Call(n, a) if n == "GLOBAL_UNIT_ASSIGNED_CONTEXT" => {
                for v in a {
                    refs(v, &mut unit_ids);
                }
            }
            Value::List(parts) => {
                for part in parts {
                    if let Value::Call(n, a) = part
                        && n == "GLOBAL_UNIT_ASSIGNED_CONTEXT"
                    {
                        for v in a {
                            refs(v, &mut unit_ids);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if unit_ids.is_empty() && allow_orphan_fallback {
        unit_ids.extend(entities.iter().filter_map(|(id, e)| match &e.value {
            Value::List(parts)
                if parts.iter().any(
                    |p| matches!(p,Value::Call(n,_)if n=="SI_UNIT"||n=="CONVERSION_BASED_UNIT"),
                ) =>
            {
                Some(*id)
            }
            Value::Call(n, _) if n == "SI_UNIT" || n == "CONVERSION_BASED_UNIT" => Some(*id),
            _ => None,
        }));
    }
    let mut found = Vec::new();
    for id in unit_ids {
        if let Ok(scale) = unit_scale(entities, id, 0, &mut BTreeSet::new()) {
            found.push(scale);
        }
    }
    if found.is_empty() {
        return Err(refuse("STEP /3 requires an SI length unit context"));
    }
    if found
        .iter()
        .any(|v| !v.is_finite() || (*v - found[0]).abs() > f64::EPSILON)
    {
        return Err(refuse("Unsupported or conflicting STEP length units"));
    }
    Ok(found[0])
}

/// Resolve reachable uncertainty contexts to millimetres. Each measure uses
/// its own explicit length unit; multiple measures select the tightest value.
pub(super) fn uncertainty_mm(
    entities: &BTreeMap<usize, Entity>,
    reachable: &BTreeSet<usize>,
) -> Result<Option<f64>> {
    let mut uncertainty_ids = BTreeSet::new();
    for id in reachable {
        let entity = &entities[id];
        match &entity.value {
            Value::Call(n, args) if n == "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT" => {
                for value in args {
                    let mut references = Vec::new();
                    refs(value, &mut references);
                    uncertainty_ids.extend(references);
                }
            }
            Value::List(parts) => {
                for part in parts {
                    if let Value::Call(n, args) = part
                        && n == "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT"
                    {
                        for value in args {
                            let mut references = Vec::new();
                            refs(value, &mut references);
                            uncertainty_ids.extend(references);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let mut tolerance = None::<f64>;
    for id in uncertainty_ids {
        let entity = entities
            .get(&id)
            .ok_or_else(|| refuse("Missing uncertainty reference"))?;
        let Value::Call(name, args) = &entity.value else {
            return Err(refuse("Uncertainty context must reference an entity call"));
        };
        if name != "UNCERTAINTY_MEASURE_WITH_UNIT" || args.len() != 4 {
            return Err(refuse(
                "UNCERTAINTY_MEASURE_WITH_UNIT argument count mismatch",
            ));
        }
        let value = match &args[0] {
            Value::Call(name, values) if name == "LENGTH_MEASURE" && values.len() == 1 => {
                number(&values[0], "uncertainty LENGTH_MEASURE")?
            }
            value => number(value, "uncertainty measure")?,
        };
        let scale = unit_scale(
            entities,
            one_ref(&args[1], "uncertainty length unit")?,
            0,
            &mut BTreeSet::new(),
        )?;
        let value_mm = value * scale;
        if !value_mm.is_finite() || !(1e-10..=1e-2).contains(&value_mm) {
            return Err(refuse(
                "STEP uncertainty must convert to a finite B-rep tolerance in [1e-10, 1e-2] mm",
            ));
        }
        tolerance = Some(tolerance.map_or(value_mm, |current| current.min(value_mm)));
    }
    Ok(tolerance)
}
