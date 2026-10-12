use super::*;

pub(super) fn refuse(message: impl Into<String>) -> Error {
    Error::new("BREP_IGES_V2_REFUSED", message)
}

#[derive(Clone, Debug)]
pub struct IgesV2Report {
    pub identity: crate::step_interchange::StepIdentityReport,
    pub ignored_metadata: Vec<String>,
    pub entity_count: usize,
    pub topology_count: usize,
}

#[derive(Clone, Debug)]
pub(super) struct Directory {
    pub(super) ty: usize,
    pub(super) parameter: usize,
    pub(super) transform: usize,
    pub(super) color: i32,
    pub(super) form: usize,
    pub(super) label: String,
}

#[derive(Clone, Debug)]
pub(super) struct Entity {
    pub(super) directory: Directory,
    pub(super) values: Vec<String>,
    pub(super) canonical: String,
}

fn field(line: &str, start: usize, end: usize) -> &str {
    &line[start..end]
}

fn integer(text: &str, what: &str) -> Result<i64> {
    text.trim()
        .parse()
        .map_err(|_| refuse(format!("Invalid IGES {what}")))
}

fn positive(text: &str, what: &str) -> Result<usize> {
    let value = integer(text, what)?;
    if value < 0 {
        return Err(refuse(format!("IGES {what} must be nonnegative")));
    }
    Ok(value as usize)
}

fn real(text: &str, what: &str) -> Result<f64> {
    let value: f64 = text
        .trim()
        .replace(['D', 'd'], "E")
        .parse()
        .map_err(|_| refuse(format!("Invalid IGES {what}")))?;
    if !value.is_finite() {
        return Err(refuse(format!("Non-finite IGES {what}")));
    }
    Ok(value)
}

fn tokenize(text: &str, parameter_delimiter: char, record_delimiter: char) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut current = String::new();
    let bytes = text.as_bytes();
    let mut at = 0usize;
    while at < bytes.len() {
        let ch = text[at..]
            .chars()
            .next()
            .ok_or_else(|| refuse("Invalid IGES UTF-8"))?;
        if ch.is_ascii_digit() {
            let start = at;
            while at < bytes.len() && bytes[at].is_ascii_digit() {
                at += 1;
            }
            if at < bytes.len() && matches!(bytes[at], b'H' | b'h') {
                let count: usize = text[start..at]
                    .parse()
                    .map_err(|_| refuse("Invalid Hollerith length"))?;
                at += 1;
                if at + count > bytes.len() {
                    return Err(refuse("Truncated Hollerith string"));
                }
                current.push_str(&text[at..at + count]);
                at += count;
                continue;
            }
            current.push_str(&text[start..at]);
            continue;
        }
        at += ch.len_utf8();
        if ch == parameter_delimiter || ch == record_delimiter {
            out.push(current.trim().to_string());
            current.clear();
            if out.len() > MAX_VALUES {
                return Err(refuse("IGES aggregate value budget exceeded"));
            }
            if ch == record_delimiter {
                break;
            }
        } else {
            current.push(ch);
        }
    }
    if !current.trim().is_empty() {
        return Err(refuse("IGES parameter record lacks record delimiter"));
    }
    Ok(out)
}

fn parse_global(text: &str) -> Result<(char, char, f64)> {
    let mut parameter = ',';
    let mut record = ';';
    let provisional = tokenize(text, ',', ';')?;
    if let Some(first) = provisional.first().filter(|v| v.chars().count() == 1) {
        parameter = first.chars().next().unwrap();
    }
    if let Some(second) = provisional.get(1).filter(|v| v.chars().count() == 1) {
        record = second.chars().next().unwrap();
    }
    if parameter == record || parameter.is_ascii_alphanumeric() || record.is_ascii_alphanumeric() {
        return Err(refuse("Invalid IGES global delimiters"));
    }
    let values = tokenize(text, parameter, record)?;
    let unit_flag = values
        .get(13)
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(2);
    let scale = match unit_flag {
        1 => 25.4,
        2 => 1.,
        4 => 304.8,
        5 => 1609344.,
        6 => 1000.,
        7 => 1_000_000.,
        8 => 0.0254,
        9 => 0.001,
        10 => 10.,
        11 => 1e-3,
        _ => return Err(refuse(format!("Unsupported IGES unit flag {unit_flag}"))),
    };
    Ok((parameter, record, scale))
}

pub(super) fn parse(text: &str) -> Result<(BTreeMap<usize, Entity>, f64)> {
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
    let (pd, rd, scale) = parse_global(&global)?;
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
    Ok((entities, scale))
}

pub(super) fn entity(entities: &BTreeMap<usize, Entity>, de: usize, ty: usize) -> Result<&Entity> {
    let e = entities
        .get(&de)
        .ok_or_else(|| refuse(format!("Missing IGES directory entry {de}")))?;
    if e.directory.ty != ty {
        return Err(refuse(format!(
            "Expected IGES {ty}, found {}",
            e.directory.ty
        )));
    }
    Ok(e)
}
pub(super) fn u(v: &[String], at: usize, what: &str) -> Result<usize> {
    positive(
        v.get(at).ok_or_else(|| refuse(format!("Missing {what}")))?,
        what,
    )
}
pub(super) fn f(v: &[String], at: usize, what: &str) -> Result<f64> {
    real(
        v.get(at).ok_or_else(|| refuse(format!("Missing {what}")))?,
        what,
    )
}

pub(super) fn curve126(
    entities: &BTreeMap<usize, Entity>,
    de: usize,
    dim: usize,
    scale: f64,
) -> Result<Curve> {
    let v = &entity(entities, de, 126)?.values;
    let k = u(v, 1, "curve upper index")?;
    let degree = u(v, 2, "curve degree")?;
    let n = k + 1;
    let knot_count = n + degree + 1;
    if n == 0 || n > 4096 || v.len() < 8 + knot_count + n + 3 * n {
        return Err(refuse("Malformed IGES rational B-spline curve"));
    }
    let mut knots = (0..knot_count)
        .map(|i| f(v, 7 + i, "curve knot"))
        .collect::<Result<Vec<_>>>()?;
    let weights = (0..n)
        .map(|i| f(v, 7 + knot_count + i, "curve weight"))
        .collect::<Result<Vec<_>>>()?;
    let cp_at = 7 + knot_count + n;
    let mut cps = Vec::with_capacity(n);
    for i in 0..n {
        let mut p = vec![
            f(v, cp_at + 3 * i, "curve X")?,
            f(v, cp_at + 3 * i + 1, "curve Y")?,
        ];
        if dim == 3 {
            p.push(f(v, cp_at + 3 * i + 2, "curve Z")?);
            for x in &mut p {
                *x *= scale;
            }
        }
        cps.push(p);
    }
    let domain_at = cp_at + 3 * n;
    let a = f(v, domain_at, "curve domain start")?;
    let b = f(v, domain_at + 1, "curve domain end")?;
    if !a.is_finite() || !b.is_finite() || a >= b {
        return Err(refuse("IGES curve domain is not increasing"));
    }
    let old = [knots[degree], knots[knots.len() - degree - 1]];
    if (old[0] - a).abs() > 1e-10 || (old[1] - b).abs() > 1e-10 {
        let factor = (b - a) / (old[1] - old[0]);
        for x in &mut knots {
            *x = a + (*x - old[0]) * factor;
        }
    }
    let c = Curve {
        degree,
        knots,
        control_points: cps,
        weights,
        periodic: u(v, 5, "curve periodic flag")? != 0,
    };
    c.validate()?;
    Ok(c)
}

pub(super) fn surface128(entities: &BTreeMap<usize, Entity>, de: usize, scale: f64) -> Result<Surface> {
    let v = &entity(entities, de, 128)?.values;
    let ku = u(v, 1, "surface U upper index")?;
    let kv = u(v, 2, "surface V upper index")?;
    let du = u(v, 3, "surface U degree")?;
    let dv = u(v, 4, "surface V degree")?;
    let nu = ku + 1;
    let nv = kv + 1;
    let n = nu
        .checked_mul(nv)
        .ok_or_else(|| refuse("Surface size overflow"))?;
    if n == 0 || n > 16384 {
        return Err(refuse("IGES surface control net exceeds 16384"));
    }
    let ku_count = nu + du + 1;
    let kv_count = nv + dv + 1;
    let mut at = 10;
    let knots_u = (0..ku_count)
        .map(|i| f(v, at + i, "surface U knot"))
        .collect::<Result<Vec<_>>>()?;
    at += ku_count;
    let knots_v = (0..kv_count)
        .map(|i| f(v, at + i, "surface V knot"))
        .collect::<Result<Vec<_>>>()?;
    at += kv_count;
    let flat_w = (0..n)
        .map(|i| f(v, at + i, "surface weight"))
        .collect::<Result<Vec<_>>>()?;
    at += n;
    let mut flat_p = Vec::with_capacity(n);
    for i in 0..n {
        flat_p.push(vec![
            f(v, at + 3 * i, "surface X")? * scale,
            f(v, at + 3 * i + 1, "surface Y")? * scale,
            f(v, at + 3 * i + 2, "surface Z")? * scale,
        ]);
    }
    let control_points = flat_p.chunks(nv).map(|r| r.to_vec()).collect();
    let weights = flat_w.chunks(nv).map(|r| r.to_vec()).collect();
    let s = Surface {
        degree_u: du,
        degree_v: dv,
        knots_u,
        knots_v,
        control_points,
        weights,
        periodic_u: u(v, 7, "surface U periodic flag")? != 0,
        periodic_v: u(v, 8, "surface V periodic flag")? != 0,
    };
    s.validate()?;
    Ok(s)
}

pub(super) fn topology_digest(entity: &Entity) -> String {
    let mut h = 0xcbf29ce484222325u64;
    for b in entity.canonical.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}
pub(super) fn verify_correspondence(
    edge: &Edge,
    pcurve: &Curve,
    surface: &Surface,
    reversed: bool,
) -> Result<()> {
    let d3 = edge.curve.domain();
    let d2 = pcurve.domain();
    for sample in 0..=8 {
        let q = sample as f64 / 8.;
        let t3 = if reversed {
            d3[1] - q * (d3[1] - d3[0])
        } else {
            d3[0] + q * (d3[1] - d3[0])
        };
        let uv = pcurve.evaluate(d2[0] + q * (d2[1] - d2[0]))?.point;
        if uv.len() != 2 {
            return Err(refuse("IGES pcurve is not two-dimensional"));
        }
        let a = edge.curve.evaluate(t3)?.point;
        let b = surface.evaluate(uv[0], uv[1])?.point;
        let error = (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]);
        if !error.is_finite() || error > 1e-5 {
            return Err(refuse(format!(
                "Independent IGES curve-pcurve correspondence failed ({error:.3e})"
            )));
        }
    }
    Ok(())
}

pub(super) struct Builder<'a> {
    pub(super) e: &'a BTreeMap<usize, Entity>,
    pub(super) scale: f64,
    pub(super) vertices: Vec<Vertex>,
    pub(super) edges: Vec<Edge>,
    pub(super) loops: Vec<Loop>,
    pub(super) faces: Vec<Face>,
    pub(super) shells: Vec<Shell>,
    pub(super) bodies: Vec<Body>,
    pub(super) vertex_de: Vec<usize>,
    pub(super) edge_de: Vec<usize>,
    pub(super) loop_de: Vec<usize>,
    pub(super) face_de: Vec<usize>,
    pub(super) shell_de: Vec<usize>,
    pub(super) body_de: Vec<usize>,
}
