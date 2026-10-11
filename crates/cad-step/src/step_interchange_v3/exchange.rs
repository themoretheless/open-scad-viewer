use super::*;
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
pub(crate) fn unit_scale(
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
pub(crate) fn length_scale(
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

fn part21_views(text: &str) -> (String, String) {
    let bytes = text.as_bytes();
    let mut clean = bytes.to_vec();
    let mut structural = bytes.to_vec();
    let mut index = 0;
    let mut in_string = false;
    while index < bytes.len() {
        if !in_string && index + 1 < bytes.len() && bytes[index] == b'/' && bytes[index + 1] == b'*'
        {
            clean[index] = b' ';
            clean[index + 1] = b' ';
            structural[index] = b' ';
            structural[index + 1] = b' ';
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                if bytes[index] != b'\n' {
                    clean[index] = b' ';
                    structural[index] = b' ';
                }
                index += 1;
            }
            if index + 1 < bytes.len() {
                clean[index] = b' ';
                clean[index + 1] = b' ';
                structural[index] = b' ';
                structural[index + 1] = b' ';
                index += 2
            }
            continue;
        }
        if bytes[index] == b'\'' {
            if in_string && index + 1 < bytes.len() && bytes[index + 1] == b'\'' {
                structural[index] = b' ';
                structural[index + 1] = b' ';
                index += 2;
                continue;
            }
            in_string = !in_string;
            structural[index] = b' ';
            index += 1;
            continue;
        }
        if in_string && bytes[index] != b'\n' {
            structural[index] = b' '
        }
        index += 1;
    }
    (
        String::from_utf8_lossy(&clean).to_ascii_uppercase(),
        String::from_utf8_lossy(&structural).to_ascii_uppercase(),
    )
}
pub(crate) fn validate_exchange_structure(text: &str) -> Result<()> {
    let (upper, structural) = part21_views(text);
    for section in ["ISO-10303-21;", "HEADER;", "DATA;", "END-ISO-10303-21;"] {
        if structural
            .lines()
            .filter(|line| line.trim() == section)
            .count()
            != 1
        {
            return Err(refuse(format!(
                "Part 21 requires exactly one {section} marker"
            )));
        }
    }
    if structural
        .lines()
        .filter(|line| line.trim() == "ENDSEC;")
        .count()
        != 2
    {
        return Err(refuse("Part 21 requires exactly two ENDSEC markers"));
    }
    let iso = structural
        .find("ISO-10303-21;")
        .ok_or_else(|| refuse("Missing ISO-10303-21 prologue"))?;
    let header = structural
        .find("HEADER;")
        .ok_or_else(|| refuse("Missing HEADER section"))?;
    let first_end = structural[header + 7..]
        .find("ENDSEC;")
        .map(|i| i + header + 7)
        .ok_or_else(|| refuse("Unterminated HEADER section"))?;
    let data = structural[first_end + 7..]
        .find("DATA;")
        .map(|i| i + first_end + 7)
        .ok_or_else(|| refuse("Missing DATA section"))?;
    let data_end = structural[data + 5..]
        .find("ENDSEC;")
        .map(|i| i + data + 5)
        .ok_or_else(|| refuse("Unterminated DATA section"))?;
    let end = structural
        .find("END-ISO-10303-21;")
        .ok_or_else(|| refuse("Missing exchange terminator"))?;
    if !(iso < header
        && header < first_end
        && first_end < data
        && data < data_end
        && data_end < end)
    {
        return Err(refuse("Part 21 sections are duplicated or out of order"));
    }
    let header_text = &structural[header..first_end];
    if !header_text.contains("FILE_DESCRIPTION(")
        || !header_text.contains("FILE_NAME(")
        || !header_text.contains("FILE_SCHEMA(")
    {
        return Err(refuse(
            "HEADER requires FILE_DESCRIPTION, FILE_NAME, and FILE_SCHEMA",
        ));
    }
    let schema = header_text.find("FILE_SCHEMA(").unwrap() + header;
    let schema_end = structural[schema..first_end]
        .find(';')
        .map(|offset| schema + offset)
        .unwrap_or(first_end);
    if !upper[schema..schema_end].contains("'AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'")
        && !upper[schema..schema_end].contains("'AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'")
    {
        return Err(refuse(
            "Direct retained B-rep requires declared AP242 schema",
        ));
    }
    Ok(())
}
pub(crate) fn data_payload(text: &str) -> Result<&str> {
    let (_, structural) = part21_views(text);
    let start = structural
        .find("DATA;")
        .ok_or_else(|| refuse("Missing DATA section"))?
        + 5;
    let end = structural[start..]
        .find("ENDSEC;")
        .map(|offset| start + offset)
        .ok_or_else(|| refuse("Unterminated DATA section"))?;
    Ok(&text[start..end])
}
