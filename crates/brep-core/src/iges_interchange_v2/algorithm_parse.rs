//! iges interchange v2 algorithm: parse.
use super::*;
pub fn parse(text: &str) -> Result<(BTreeMap<usize, Entity>, IgesLengthUnit, f64)> {
    if text.len() > MAX_BYTES {
        return Err(refuse("IGES payload exceeds 16 MiB"));
    }
    let mut sections: BTreeMap<char, Vec<(usize, String)>> = BTreeMap::new();
    let mut last = BTreeMap::<char, usize>::new();
    for raw in text.lines() {
        let raw = raw.trim_end_matches('\r');
        if raw.len() != 80 || !raw.is_ascii() {
            return Err(refuse(
                "Every IGES physical record must be 80 ASCII columns",
            ));
        }
        let section = raw.as_bytes()[72] as char;
        if !matches!(section, 'S' | 'G' | 'D' | 'P' | 'T') {
            return Err(refuse("Unknown IGES section marker"));
        }
        let sequence = positive(field(raw, 73, 80), "record sequence")?;
        if sequence != last.get(&section).copied().unwrap_or(0) + 1 {
            return Err(refuse(format!("Non-contiguous IGES {section} sequence")));
        }
        last.insert(section, sequence);
        sections
            .entry(section)
            .or_default()
            .push((sequence, raw.to_string()));
        if last.values().sum::<usize>() > MAX_RECORDS {
            return Err(refuse("IGES physical record budget exceeded"));
        }
    }
    for required in ['S', 'G', 'D', 'P', 'T'] {
        if !sections.contains_key(&required) {
            return Err(refuse(format!("IGES missing {required} section")));
        }
    }
    if sections[&'T'].len() != 1 {
        return Err(refuse("IGES must have one terminate record"));
    }
    let order = text
        .lines()
        .map(|l| l.as_bytes()[72] as char)
        .collect::<Vec<_>>();
    if !order
        .windows(2)
        .all(|w| "SGDPT".find(w[0]).unwrap() <= "SGDPT".find(w[1]).unwrap())
    {
        return Err(refuse("IGES sections are out of order"));
    }
    let global = sections[&'G']
        .iter()
        .map(|(_, l)| field(l, 0, 72))
        .collect::<String>();
    let (pd, rd, unit, resolution) = parse_global(&global)?;
    let directory = &sections[&'D'];
    if !directory.len().is_multiple_of(2) {
        return Err(refuse("IGES directory section must contain record pairs"));
    }
    let mut directories = BTreeMap::new();
    for pair in directory.as_chunks::<2>().0 {
        let a = &pair[0].1;
        let b = &pair[1].1;
        let de = pair[0].0;
        if de % 2 == 0 || pair[1].0 != de + 1 {
            return Err(refuse("Malformed IGES directory pair"));
        }
        let ty = positive(field(a, 0, 8), "entity type")?;
        if positive(field(b, 0, 8), "repeated entity type")? != ty {
            return Err(refuse("IGES directory type mismatch"));
        }
        directories.insert(
            de,
            Directory {
                ty,
                parameter: positive(field(a, 8, 16), "parameter pointer")?,
                transform: positive(field(a, 48, 56), "transformation pointer")?,
                color: integer(field(b, 16, 24), "color")? as i32,
                form: positive(field(b, 32, 40), "form")?,
                label: field(b, 56, 64).trim().to_string(),
                status: parse_directory_status(de, field(a, 64, 72))?,
            },
        );
        if directories.len() > MAX_ENTITIES {
            return Err(refuse("IGES entity count exceeds 65536"));
        }
    }
    let mut parameter_rows = BTreeMap::new();
    for (sequence, row) in &sections[&'P'] {
        let owner = positive(field(row, 64, 72), "parameter owner")?;
        parameter_rows.insert(*sequence, (owner, field(row, 0, 64).to_string()));
    }
    let mut entities = BTreeMap::new();
    for (de, dir) in directories {
        let mut sequence = dir.parameter;
        let mut parameter_text = String::new();
        loop {
            let (owner, payload) = parameter_rows
                .get(&sequence)
                .ok_or_else(|| refuse(format!("Missing parameter row {sequence}")))?;
            if *owner != de {
                return Err(refuse(
                    "IGES parameter owner does not match directory entry",
                ));
            }
            parameter_text.push_str(payload);
            if payload.contains(rd) {
                break;
            }
            sequence += 1;
            if sequence - dir.parameter > 4096 {
                return Err(refuse("IGES entity parameter span exceeds 4096 records"));
            }
        }
        let values = tokenize(&parameter_text, pd, rd)?;
        if values.first().and_then(|v| v.parse::<usize>().ok()) != Some(dir.ty) {
            return Err(refuse("IGES parameter entity type mismatch"));
        }
        let canonical = values
            .iter()
            .map(|v| v.trim())
            .collect::<Vec<_>>()
            .join(",");
        entities.insert(
            de,
            Entity {
                directory: dir,
                values,
                canonical,
            },
        );
    }
    Ok((entities, unit, resolution))
}
