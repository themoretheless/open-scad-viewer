//! Strict finite IGES 5.3 direct B-rep interchange.
//!
//! The legacy `iges-interchange/1` walking slice remains frozen.  This
//! successor parses physical 80-column records and maps the admitted IGES
//! topology graph directly; it never recognizes constructors, derives an
//! AABB, or crosses through a mesh.

use brep_core::analytic_features::FeatureCertificate;
use brep_core::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopoId, TopoKind, TopologyIds};
use brep_topology::Vertex;
use nurbs_core::{curve::Curve, surface::{Axis, Surface}, Error, Result};
use std::collections::{BTreeMap, BTreeSet};

pub const IGES_INTERCHANGE_V2_CAPABILITY: &str = "iges-interchange/2";
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_ENTITY_PARAMETER_ROWS: usize = 4097;
const MAX_RECORDS: usize = 131_072;
const MAX_ENTITIES: usize = 65_536;
const MAX_VALUES: usize = 1_000_000;
const MAX_TOPOLOGY: usize = 32_768;
const MAX_GRAPH_WORK: usize = 4_000_000;
const MAX_COMPOSITE_DEPTH: usize = 64;
const MAX_COMPOSITE_SEGMENTS: usize = 256;
const MAX_COMPOSITE_CONTROLS: usize = 256;
const MAX_COMPOSITE_WORK: usize = 4096;

type TopologyMapping = Vec<(TopoKind, usize, usize)>;

fn refuse(message: impl Into<String>) -> Error {
    Error::new("BREP_IGES_V2_REFUSED", message)
}

fn composite_error(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

fn trimmed_error(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

fn trimmed_unsupported(de: usize, message: impl Into<String>) -> Error {
    trimmed_error(
        "BREP_IGES_V2_TRIMMED_UNSUPPORTED",
        format!("IGES trimmed/bounded DE {de}: {}", message.into()),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IgesLengthUnit {
    Inch,
    Millimeter,
    Foot,
    Mile,
    Meter,
    Kilometer,
    Mil,
    Micrometer,
    Centimeter,
    Microinch,
}

impl IgesLengthUnit {
    pub fn from_flag(flag: usize) -> Option<Self> {
        Some(match flag {
            1 => Self::Inch,
            2 => Self::Millimeter,
            4 => Self::Foot,
            5 => Self::Mile,
            6 => Self::Meter,
            7 => Self::Kilometer,
            8 => Self::Mil,
            9 => Self::Micrometer,
            10 => Self::Centimeter,
            11 => Self::Microinch,
            _ => return None,
        })
    }

    pub const fn flag(self) -> usize {
        match self {
            Self::Inch => 1,
            Self::Millimeter => 2,
            Self::Foot => 4,
            Self::Mile => 5,
            Self::Meter => 6,
            Self::Kilometer => 7,
            Self::Mil => 8,
            Self::Micrometer => 9,
            Self::Centimeter => 10,
            Self::Microinch => 11,
        }
    }

    pub const fn scale_to_mm(self) -> f64 {
        match self {
            Self::Inch => 25.4,
            Self::Millimeter => 1.,
            Self::Foot => 304.8,
            Self::Mile => 1_609_344.,
            Self::Meter => 1_000.,
            Self::Kilometer => 1_000_000.,
            Self::Mil => 0.0254,
            Self::Micrometer => 0.001,
            Self::Centimeter => 10.,
            Self::Microinch => 25.4e-6,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IgesV2ImportOptions {
    pub unit_override: Option<IgesLengthUnit>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IgesV2UnitReport {
    pub declared_unit: IgesLengthUnit,
    pub applied_unit: IgesLengthUnit,
    pub declared_flag: usize,
    pub declared_scale_to_mm: f64,
    pub applied_scale_to_mm: f64,
    pub overridden: bool,
    pub warning: Option<String>,
}

impl IgesV2UnitReport {
    fn export_millimeters() -> Self {
        Self {
            declared_unit: IgesLengthUnit::Millimeter,
            applied_unit: IgesLengthUnit::Millimeter,
            declared_flag: IgesLengthUnit::Millimeter.flag(),
            declared_scale_to_mm: 1.,
            applied_scale_to_mm: 1.,
            overridden: false,
            warning: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IgesV2DiscardReport {
    pub blanked: usize,
    pub dependent_without_owner: usize,
}

#[derive(Clone, Debug)]
pub struct IgesV2Report {
    pub identity: crate::step_interchange::StepIdentityReport,
    pub ignored_metadata: Vec<String>,
    pub entity_count: usize,
    pub topology_count: usize,
    pub units: IgesV2UnitReport,
    pub discarded: IgesV2DiscardReport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Subordinate {
    Independent,
    Physical,
    Logical,
    Both,
}

impl Subordinate {
    fn requires_physical(self) -> bool {
        matches!(self, Self::Physical | Self::Both)
    }

    fn requires_logical(self) -> bool {
        matches!(self, Self::Logical | Self::Both)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DirectoryStatus {
    blanked: bool,
    subordinate: Subordinate,
}

#[derive(Clone, Debug)]
struct Directory {
    ty: usize,
    parameter: usize,
    transform: usize,
    color: i32,
    form: usize,
    label: String,
    status: DirectoryStatus,
}

#[derive(Clone, Debug)]
struct Entity {
    directory: Directory,
    values: Vec<String>,
    canonical: String,
    legacy_surface_order: bool,
    legacy_curve_flags: bool,
    legacy_surface_flags: bool,
}

fn parse_directory_status(de: usize, text: &str) -> Result<DirectoryStatus> {
    if text.len() != 8 || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(refuse(format!(
            "Invalid IGES DE {de} status field: expected exactly 8 digits"
        )));
    }
    let blanked = match &text[0..2] {
        "00" => false,
        "01" => true,
        code => {
            return Err(refuse(format!(
                "Invalid IGES DE {de} blank status code {code}"
            )));
        }
    };
    let subordinate = match &text[2..4] {
        "00" => Subordinate::Independent,
        "01" => Subordinate::Physical,
        "02" => Subordinate::Logical,
        "03" => Subordinate::Both,
        code => {
            return Err(refuse(format!(
                "Invalid IGES DE {de} subordinate status code {code}"
            )));
        }
    };
    let entity_use = &text[4..6];
    if !matches!(entity_use, "00" | "01" | "02" | "03" | "04" | "05" | "06") {
        return Err(refuse(format!(
            "Invalid IGES DE {de} entity-use status code {entity_use}"
        )));
    }
    let hierarchy = &text[6..8];
    if !matches!(hierarchy, "00" | "01" | "02") {
        return Err(refuse(format!(
            "Invalid IGES DE {de} hierarchy status code {hierarchy}"
        )));
    }
    Ok(DirectoryStatus {
        blanked,
        subordinate,
    })
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

fn parse_global(text: &str) -> Result<(char, char, IgesLengthUnit, f64, Option<String>)> {
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
    for index in [6, 7, 8, 9, 10, 12, 15, 16, 18, 19, 22, 23] {
        if let Some(value) = values.get(index).filter(|value| !value.is_empty()) {
            real(value, &format!("global numeric value {index}"))?;
        }
    }
    let unit_flag = values
        .get(13)
        .ok_or_else(|| refuse("Missing IGES unit flag"))?;
    if unit_flag.is_empty() {
        return Err(refuse("Missing IGES unit flag"));
    }
    let unit_flag = unit_flag
        .parse::<usize>()
        .map_err(|_| refuse(format!("Invalid IGES unit flag {unit_flag}")))?;
    let unit = IgesLengthUnit::from_flag(unit_flag)
        .ok_or_else(|| refuse(format!("Unsupported IGES unit flag {unit_flag}")))?;
    let resolution = real(
        values
            .get(18)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| refuse("Missing IGES global minimum resolution (parameter 19)"))?,
        "global minimum resolution (parameter 19)",
    )?;
    if resolution <= 0. {
        return Err(refuse("IGES global minimum resolution must be positive"));
    }
    let legacy_sender = values.get(11).filter(|_| values.get(4).map(String::as_str) == Some("OpenSCAD Viewer")).cloned();
    Ok((parameter, record, unit, resolution, legacy_sender))
}

fn parse(text: &str) -> Result<(BTreeMap<usize, Entity>, IgesLengthUnit, f64)> {
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
        .all(|w| {
            let rank = |c: char| "SGDPT".find(c).expect("parse() rejects unknown section markers");
            rank(w[0]) <= rank(w[1])
        })
    {
        return Err(refuse("IGES sections are out of order"));
    }
    let global = sections[&'G']
        .iter()
        .map(|(_, l)| field(l, 0, 72))
        .collect::<String>();
    let (pd, rd, unit, resolution, sender_version) = parse_global(&global)?;
    let writer_version = sender_version.as_deref().filter(|_| {
        sections[&'S'][0].1[..72].trim_end() == "OpenSCAD Viewer direct finite B-rep"
    });
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
            if sequence - dir.parameter >= MAX_ENTITY_PARAMETER_ROWS {
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
                legacy_surface_order: writer_version == Some("IGES2"),
                legacy_curve_flags: matches!(writer_version, Some("IGES2" | "IGES2.1")),
                legacy_surface_flags: matches!(writer_version, Some("IGES2" | "IGES2.1" | "IGES2.2")),
                directory: dir,
                values,
                canonical,
            },
        );
    }
    Ok((entities, unit, resolution))
}

fn entity(entities: &BTreeMap<usize, Entity>, de: usize, ty: usize) -> Result<&Entity> {
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
fn u(v: &[String], at: usize, what: &str) -> Result<usize> {
    positive(
        v.get(at).ok_or_else(|| refuse(format!("Missing {what}")))?,
        what,
    )
}
fn f(v: &[String], at: usize, what: &str) -> Result<f64> {
    real(
        v.get(at).ok_or_else(|| refuse(format!("Missing {what}")))?,
        what,
    )
}

#[derive(Clone, Copy, Debug)]
struct EntityReference {
    target: usize,
    role: &'static str,
}

fn entity_references(de: usize, entity: &Entity) -> Result<Vec<EntityReference>> {
    let v = &entity.values;
    let exact_count = |base: usize, n: usize, stride: usize, what: &str| -> Result<()> {
        let expected = n
            .checked_mul(stride)
            .and_then(|count| count.checked_add(base))
            .ok_or_else(|| refuse(format!("IGES DE {de} {what} size overflow")))?;
        if v.len() != expected {
            return Err(refuse(format!("Malformed IGES DE {de} {what}")));
        }
        Ok(())
    };
    let mut refs = Vec::new();
    let mut push = |at: usize, role: &'static str| -> Result<()> {
        refs.push(EntityReference {
            target: u(v, at, role)?,
            role,
        });
        Ok(())
    };
    match entity.directory.ty {
        100 | 502 | 110 | 124 | 126 | 128 | 141 | 314 => {}
        142 => {
            exact_count(6, 0, 0, "curve on parametric surface")?;
            push(2, "parametric surface")?;
            if u(v, 3, "boundary pcurve")? != 0 {
                push(3, "boundary pcurve")?;
            }
            if u(v, 4, "boundary model curve")? != 0 {
                push(4, "boundary model curve")?;
            }
        }
        143 => {
            let n = u(v, 4, "bounded surface segment count")?;
            exact_count(5, n, 3, "bounded surface")?;
            push(3, "bounded surface geometry")?;
            for i in 0..n {
                let at = 5 + 3 * i;
                if u(v, at, "boundary model curve")? != 0 {
                    push(at, "boundary model curve")?;
                }
                if u(v, at + 1, "boundary pcurve")? != 0 {
                    push(at + 1, "boundary pcurve")?;
                }
            }
        }
        144 => {
            let n = u(v, 3, "trimmed surface inner boundary count")?;
            exact_count(5, n, 1, "trimmed surface")?;
            push(1, "trimmed surface geometry")?;
            if u(v, 2, "trimmed surface outer flag")? != 0 {
                push(4, "trimmed surface outer boundary")?;
            }
            for i in 0..n {
                push(5 + i, "trimmed surface inner boundary")?;
            }
        }
        102 => {
            let n = u(v, 1, "composite segment count")?;
            exact_count(2, n, 1, "composite curve")?;
            for i in 0..n {
                push(2 + i, "composite segment")?;
            }
        }
        504 => {
            let n = u(v, 1, "edge count")?;
            exact_count(2, n, 5, "edge list")?;
            for i in 0..n {
                let at = 2 + 5 * i;
                push(at, "edge curve")?;
                push(at + 1, "start vertex list")?;
                push(at + 3, "end vertex list")?;
            }
        }
        508 => {
            let n = u(v, 1, "loop member count")?;
            exact_count(2, n, 7, "loop")?;
            for i in 0..n {
                let at = 2 + 7 * i;
                push(at + 1, "loop edge list")?;
                push(at + 6, "loop pcurve")?;
            }
        }
        510 => {
            let n = u(v, 2, "face loop count")?;
            exact_count(4, n, 1, "face")?;
            push(1, "face surface")?;
            for i in 0..n {
                push(4 + i, "face loop")?;
            }
        }
        514 => {
            let n = u(v, 1, "shell face count")?;
            exact_count(2, n, 2, "shell")?;
            for i in 0..n {
                push(2 + 2 * i, "shell face")?;
            }
        }
        186 => {
            let n = u(v, 3, "solid void count")?;
            exact_count(4, n, 2, "solid")?;
            push(1, "solid outer shell")?;
            for i in 0..n {
                push(4 + 2 * i, "solid void shell")?;
            }
        }
        402 => match entity.directory.form {
            1 | 14 => {
                return Err(refuse(format!(
                    "Unsupported IGES DE {de} associativity form {}: back-pointer parameters are not safely separable",
                    entity.directory.form
                )));
            }
            7 | 15 => {
                let n = u(v, 1, "associativity member count")?;
                exact_count(2, n, 1, "associativity group")?;
                for i in 0..n {
                    push(2 + i, "associativity member")?;
                }
            }
            _ => {}
        },
        406 if entity.directory.form == 15 && v.len() == 4 => {
            push(3, "identity target")?;
        }
        _ => {}
    }
    if entity.directory.transform != 0 {
        refs.push(EntityReference { target: entity.directory.transform, role: "directory transform" });
    }
    Ok(refs)
}

fn graph_work(work: &mut usize, amount: usize) -> Result<()> {
    *work = work
        .checked_add(amount)
        .ok_or_else(|| refuse("IGES directory graph work budget overflow"))?;
    if *work > MAX_GRAPH_WORK {
        return Err(refuse("IGES directory graph work budget exceeded"));
    }
    Ok(())
}

fn filter_discarded_entities(
    entities: &mut BTreeMap<usize, Entity>,
) -> Result<IgesV2DiscardReport> {
    let mut work = 0;
    let mut live = BTreeSet::new();
    let mut blanked = 0;
    for (&de, entity) in entities.iter() {
        graph_work(&mut work, 1)?;
        if entity.directory.status.blanked {
            blanked += 1;
        } else {
            live.insert(de);
        }
    }

    let mut references = BTreeMap::new();
    let mut physical_owners = BTreeMap::<usize, BTreeSet<usize>>::new();
    let mut logical_owners = BTreeMap::<usize, BTreeSet<usize>>::new();
    let mut dependents_by_owner = BTreeMap::<usize, BTreeSet<usize>>::new();
    for &de in &live {
        let entity = &entities[&de];
        let refs = entity_references(de, entity)?;
        graph_work(&mut work, refs.len())?;
        for reference in &refs {
            let owners = if entity.directory.ty == 402 {
                Some(&mut logical_owners)
            } else if entity.directory.ty == 406 {
                None
            } else {
                Some(&mut physical_owners)
            };
            if let Some(owners) = owners {
                owners.entry(reference.target).or_default().insert(de);
                dependents_by_owner
                    .entry(de)
                    .or_default()
                    .insert(reference.target);
            }
        }
        references.insert(de, refs);
    }

    let mut worklist = live
        .iter()
        .copied()
        .filter(|&de| entities[&de].directory.status.subordinate != Subordinate::Independent)
        .collect::<BTreeSet<_>>();
    let mut dependent_without_owner = 0;
    while let Some(de) = worklist.pop_first() {
        graph_work(&mut work, 1)?;
        if !live.contains(&de) {
            continue;
        }
        let subordinate = entities[&de].directory.status.subordinate;
        let no_physical = physical_owners.get(&de).is_none_or(BTreeSet::is_empty);
        let no_logical = logical_owners.get(&de).is_none_or(BTreeSet::is_empty);
        if !(subordinate.requires_physical() && no_physical)
            && !(subordinate.requires_logical() && no_logical)
        {
            continue;
        }
        live.remove(&de);
        dependent_without_owner += 1;
        let dependents = dependents_by_owner.get(&de).cloned().unwrap_or_default();
        for dependent in dependents {
            graph_work(&mut work, 1)?;
            if let Some(owners) = physical_owners.get_mut(&dependent) {
                owners.remove(&de);
            }
            if let Some(owners) = logical_owners.get_mut(&dependent) {
                owners.remove(&de);
            }
            worklist.insert(dependent);
        }
    }

    for &de in &live {
        for reference in &references[&de] {
            graph_work(&mut work, 1)?;
            if !live.contains(&reference.target) {
                let state = if entities.contains_key(&reference.target) {
                    "discarded"
                } else {
                    "missing"
                };
                let message = format!(
                    "Retained IGES DE {de} references {state} DE {} as {}",
                    reference.target, reference.role
                );
                return Err(if entities[&de].directory.ty == 102 {
                    composite_error("BREP_IGES_V2_COMPOSITE_DANGLING", message)
                } else {
                    refuse(message)
                });
            }
        }
    }

    let group_nodes = live
        .iter()
        .copied()
        .filter(|&de| entities[&de].directory.ty == 402)
        .collect::<BTreeSet<_>>();
    let mut indegree = group_nodes
        .iter()
        .map(|&de| (de, 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = BTreeMap::<usize, Vec<usize>>::new();
    for &de in &group_nodes {
        for reference in &references[&de] {
            if group_nodes.contains(&reference.target) {
                graph_work(&mut work, 1)?;
                *indegree.get_mut(&reference.target).unwrap() += 1;
                outgoing.entry(de).or_default().push(reference.target);
            }
        }
    }
    let mut ready = indegree
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(&de, _)| de)
        .collect::<BTreeSet<_>>();
    let mut processed = 0;
    while let Some(de) = ready.pop_first() {
        graph_work(&mut work, 1)?;
        processed += 1;
        if let Some(targets) = outgoing.get(&de) {
            for &target in targets {
                graph_work(&mut work, 1)?;
                let count = indegree.get_mut(&target).unwrap();
                *count -= 1;
                if *count == 0 {
                    ready.insert(target);
                }
            }
        }
    }
    if processed != group_nodes.len() {
        let de = indegree
            .iter()
            .find(|(_, count)| **count > 0)
            .map(|(&de, _)| de)
            .expect("Kahn's sort left unprocessed nodes, so one keeps a positive indegree");
        return Err(refuse(format!(
            "IGES 402 associativity cycle involving DE {de}"
        )));
    }

    entities.retain(|de, _| live.contains(de));
    Ok(IgesV2DiscardReport {
        blanked,
        dependent_without_owner,
    })
}

fn curve126(
    entities: &BTreeMap<usize, Entity>,
    de: usize,
    dim: usize,
    scale: f64,
) -> Result<Curve> {
    let v = &entity(entities, de, 126)?.values;
    let k = u(v, 1, "curve upper index")?;
    let degree = u(v, 2, "curve degree")?;
    if k >= 4096 || degree > k {
        return Err(refuse("Malformed IGES rational B-spline curve dimensions"));
    }
    let n = k + 1;
    let knot_count = n + degree + 1;
    if v.len() < 9 + knot_count + 4 * n {
        return Err(refuse("Malformed IGES rational B-spline curve"));
    }
    let knots = (0..knot_count)
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
    let c = Curve {
        degree,
        knots,
        control_points: cps,
        weights,
        periodic: u(v, if entities[&de].legacy_curve_flags { 5 } else { 6 }, "curve periodic flag")? != 0,
    };
    c.validate()?;
    if [a, b] == c.domain() {
        Ok(c)
    } else {
        c.trim(a, b)
    }
}

#[derive(Clone)]
struct CompositeSegment {
    de: usize,
    curve: Curve,
}

enum CompositeVisit {
    Enter(usize, usize),
    Exit(usize, usize, usize),
}

mod circular_arc;
mod placement;
mod natural_boundary;
mod affine_boundary;
mod composite_join;

struct CurveDispatcher<'a> {
    entities: &'a BTreeMap<usize, Entity>,
    dim: usize,
    scale: f64,
    tolerance_mm: f64,
    work: usize,
}

impl<'a> CurveDispatcher<'a> {
    fn budget(&mut self, composite: usize, amount: usize) -> Result<()> {
        self.work = self.work.checked_add(amount).ok_or_else(|| {
            composite_error(
                "BREP_IGES_V2_COMPOSITE_BUDGET",
                format!("IGES composite DE {composite} work budget overflow"),
            )
        })?;
        if self.work > MAX_COMPOSITE_WORK {
            return Err(composite_error(
                "BREP_IGES_V2_COMPOSITE_BUDGET",
                format!("IGES composite DE {composite} work budget exceeded"),
            ));
        }
        Ok(())
    }

    fn base_curve(&mut self, composite: usize, de: usize, composite_member: bool) -> Result<Curve> {
        let entity = self.entities.get(&de).ok_or_else(|| {
            composite_error(
                "BREP_IGES_V2_COMPOSITE_DANGLING",
                format!("IGES composite DE {composite} has missing segment DE {de}"),
            )
        })?;
        let transform = entity.directory.transform;
        let source_dim = if transform == 0 { self.dim } else { 3 };
        let mut curve = match entity.directory.ty {
            100 => circular_arc::read(entity, source_dim, self.scale, self.tolerance_mm)?,
            110 => {
                let values = &entity.values;
                if values.len() != 7 {
                    return Err(composite_error(
                        "BREP_IGES_V2_COMPOSITE_DANGLING",
                        format!("Malformed IGES line DE {de} in composite DE {composite}"),
                    ));
                }
                let mut points = Vec::with_capacity(2);
                for endpoint in 0..2 {
                    let at = 1 + 3 * endpoint;
                    let mut point = vec![f(values, at, "line X")?, f(values, at + 1, "line Y")?];
                    if source_dim == 3 {
                        point.push(f(values, at + 2, "line Z")?);
                    }
                    for coordinate in &mut point {
                        *coordinate *= self.scale;
                    }
                    points.push(point);
                }
                Curve::from_polyline(points)?
            }
            126 => curve126(self.entities, de, source_dim, self.scale)?,
            102 => {
                return Err(composite_error(
                    "BREP_IGES_V2_COMPOSITE_CYCLE",
                    format!("Unexpected nested dispatch at IGES composite DE {de}"),
                ));
            }
            _ => {
                return Err(composite_error(
                    "BREP_IGES_V2_COMPOSITE_DANGLING",
                    format!(
                        "IGES composite DE {composite} segment DE {de} has non-curve type {}",
                        entity.directory.ty
                    ),
                ));
            }
        };
        if transform != 0 {
            let matrix = placement::read(self.entities, transform)?;
            for point in &mut curve.control_points {
                *point = placement::apply(&matrix, point, self.scale, self.dim)?;
            }
            curve.validate()?;
        }
        if !composite_member {
            return Ok(curve);
        }
        if curve.control_points.len() > MAX_COMPOSITE_CONTROLS {
            return Err(composite_error(
                "BREP_IGES_V2_COMPOSITE_BUDGET",
                format!("IGES composite DE {composite} control budget exceeded at DE {de}"),
            ));
        }
        Ok(curve)
    }

    fn endpoints(curve: &Curve) -> Result<(Vec<f64>, Vec<f64>)> {
        let domain = curve.domain();
        Ok((
            curve.evaluate(domain[0])?.point,
            curve.evaluate(domain[1])?.point,
        ))
    }

    fn distance(a: &[f64], b: &[f64]) -> Result<f64> {
        if a.len() != b.len() {
            return Err(composite_error(
                "BREP_IGES_V2_COMPOSITE_UNSUPPORTED",
                "IGES composite segments have inconsistent dimensions",
            ));
        }
        let distance = a
            .iter()
            .zip(b)
            .fold(0., |sum, (x, y)| sum + (x - y) * (x - y))
            .sqrt();
        if !distance.is_finite() {
            return Err(composite_error(
                "BREP_IGES_V2_COMPOSITE_UNSUPPORTED",
                "Non-finite IGES composite endpoint distance",
            ));
        }
        Ok(distance)
    }

    fn composite(&mut self, root: usize) -> Result<Curve> {
        let mut stack = vec![CompositeVisit::Enter(root, 0)];
        let mut active = BTreeSet::new();
        let mut memo = BTreeMap::<(usize, usize), Curve>::new();
        let mut segments = Vec::<CompositeSegment>::new();
        let mut result = None;
        while let Some(visit) = stack.pop() {
            self.budget(root, 1)?;
            match visit {
                CompositeVisit::Enter(de, depth) => {
                    if active.contains(&de) {
                        return Err(composite_error(
                            "BREP_IGES_V2_COMPOSITE_CYCLE",
                            format!("IGES composite cycle at DE {de}"),
                        ));
                    }
                    if let Some(curve) = memo.get(&(de, self.dim)) {
                        if segments.len() >= MAX_COMPOSITE_SEGMENTS {
                            return Err(composite_error(
                                "BREP_IGES_V2_COMPOSITE_BUDGET",
                                format!("IGES composite DE {root} segment budget exceeded"),
                            ));
                        }
                        segments.push(CompositeSegment {
                            de,
                            curve: curve.clone(),
                        });
                        continue;
                    }
                    let entity = self.entities.get(&de).ok_or_else(|| {
                        composite_error(
                            "BREP_IGES_V2_COMPOSITE_DANGLING",
                            format!("Missing IGES composite segment DE {de}"),
                        )
                    })?;
                    if entity.directory.ty != 102 {
                        let curve = self.base_curve(root, de, true)?;
                        if segments.len() >= MAX_COMPOSITE_SEGMENTS {
                            return Err(composite_error(
                                "BREP_IGES_V2_COMPOSITE_BUDGET",
                                format!("IGES composite DE {root} segment budget exceeded"),
                            ));
                        }
                        segments.push(CompositeSegment { de, curve });
                        continue;
                    }
                    if depth >= MAX_COMPOSITE_DEPTH {
                        return Err(composite_error(
                            "BREP_IGES_V2_COMPOSITE_BUDGET",
                            format!("IGES composite DE {root} depth budget exceeded at DE {de}"),
                        ));
                    }
                    let mut refs = entity_references(de, entity)?;
                    refs.retain(|reference| reference.role == "composite segment");
                    if refs.is_empty() {
                        return Err(composite_error(
                            "BREP_IGES_V2_COMPOSITE_DANGLING",
                            format!("IGES composite DE {de} has no segments"),
                        ));
                    }
                    active.insert(de);
                    stack.push(CompositeVisit::Exit(de, segments.len(), self.dim));
                    if entity.directory.transform != 0 {
                        self.dim = 3;
                    }
                    for reference in refs.iter().rev() {
                        self.budget(root, 1)?;
                        stack.push(CompositeVisit::Enter(reference.target, depth + 1));
                    }
                }
                CompositeVisit::Exit(de, start, destination_dim) => {
                    active.remove(&de);
                    let mut curve = self.join_segments(de, &segments[start..], self.tolerance_mm)?;
                    self.dim = destination_dim;
                    let transform = self.entities[&de].directory.transform;
                    if transform != 0 {
                        let matrix = placement::read(self.entities, transform)?;
                        for point in &mut curve.control_points {
                            *point = placement::apply(&matrix, point, self.scale, self.dim)?;
                        }
                        curve.validate()?;
                        segments.truncate(start);
                        segments.push(CompositeSegment { de, curve: curve.clone() });
                    }
                    memo.insert((de, self.dim), curve.clone());
                    if de == root {
                        result = Some(curve);
                    }
                }
            }
        }
        result.ok_or_else(|| {
            composite_error(
                "BREP_IGES_V2_COMPOSITE_DANGLING",
                format!("IGES composite DE {root} produced no curve"),
            )
        })
    }

    fn dispatch(mut self, de: usize) -> Result<Curve> {
        let is_composite = self
            .entities
            .get(&de)
            .ok_or_else(|| refuse(format!("Missing IGES curve DE {de}")))?
            .directory
            .ty
            == 102;
        if is_composite {
            self.composite(de)
        } else {
            self.base_curve(de, de, false)
        }
    }
}

fn surface128(entities: &BTreeMap<usize, Entity>, de: usize, scale: f64) -> Result<Surface> {
    let surface_entity = entity(entities, de, 128)?;
    let v = &surface_entity.values;
    let placement = (surface_entity.directory.transform != 0)
        .then(|| placement::read(entities, surface_entity.directory.transform))
        .transpose()?;
    let ku = u(v, 1, "surface U upper index")?;
    let kv = u(v, 2, "surface V upper index")?;
    let du = u(v, 3, "surface U degree")?;
    let dv = u(v, 4, "surface V degree")?;
    if ku >= 16384 || kv >= 16384 || du > ku || dv > kv {
        return Err(refuse("Malformed IGES rational B-spline surface dimensions"));
    }
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
        let point = vec![
            f(v, at + 3 * i, "surface X")? * scale,
            f(v, at + 3 * i + 1, "surface Y")? * scale,
            f(v, at + 3 * i + 2, "surface Z")? * scale,
        ];
        flat_p.push(match &placement {
            Some(matrix) => placement::apply(matrix, &point, scale, 3)?,
            None => point,
        });
    }
    let index = |u: usize, v: usize| if surface_entity.legacy_surface_order { u * nv + v } else { v * nu + u };
    let control_points = (0..nu).map(|u| (0..nv).map(|v| flat_p[index(u,v)].clone()).collect()).collect();
    let weights = (0..nu).map(|u| (0..nv).map(|v| flat_w[index(u,v)]).collect()).collect();
    let mut s = Surface {
        degree_u: du,
        degree_v: dv,
        knots_u,
        knots_v,
        control_points,
        weights,
        periodic_u: u(v, if surface_entity.legacy_surface_flags { 7 } else { 8 }, "surface U periodic flag")? != 0,
        periodic_v: u(v, if surface_entity.legacy_surface_flags { 8 } else { 9 }, "surface V periodic flag")? != 0,
    };
    s.validate()?;
    let at = at + 3 * n;
    let ranges = [
        (Axis::U, [s.knots_u[du], s.knots_u[nu]], [f(v, at, "surface U start")?, f(v, at + 1, "surface U end")?]),
        (Axis::V, [s.knots_v[dv], s.knots_v[nv]], [f(v, at + 2, "surface V start")?, f(v, at + 3, "surface V end")?]),
    ];
    for (axis, domain, [start, end]) in ranges {
        if [start, end] != domain {
            s = s.edit_axis(axis, |curve| curve.trim(start, end))?;
        }
    }
    Ok(s)
}

fn topology_digest(entity: &Entity) -> String {
    let mut h = 0xcbf29ce484222325u64;
    for b in entity.canonical.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}
fn constant_curve(curve: &Curve) -> bool {
    curve.control_points.iter().all(|point| point == &curve.control_points[0])
}

fn verify_correspondence(
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

struct Builder<'a> {
    e: &'a BTreeMap<usize, Entity>,
    scale: f64,
    merge_tolerance_mm: f64,
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    loops: Vec<Loop>,
    faces: Vec<Face>,
    shells: Vec<Shell>,
    bodies: Vec<Body>,
    vertex_de: Vec<usize>,
    edge_de: Vec<usize>,
    loop_de: Vec<usize>,
    face_de: Vec<usize>,
    shell_de: Vec<usize>,
    body_de: Vec<usize>,
}
impl<'a> Builder<'a> {
    fn curve(&self, de: usize, dim: usize) -> Result<Curve> {
        CurveDispatcher {
            entities: self.e,
            dim,
            scale: if dim == 3 { self.scale } else { 1. },
            tolerance_mm: self.merge_tolerance_mm,
            work: 0,
        }
        .dispatch(de)
    }

    fn vertex_list(&mut self, de: usize) -> Result<Vec<usize>> {
        let v = &entity(self.e, de, 502)?.values;
        let n = u(v, 1, "vertex count")?;
        if n > MAX_TOPOLOGY || v.len() != 2 + 3 * n {
            return Err(refuse("Malformed IGES vertex list"));
        }
        let mut out = Vec::new();
        for i in 0..n {
            let index = self.vertices.len();
            self.vertices.push(Vertex {
                point: [
                    f(v, 2 + 3 * i, "vertex X")? * self.scale,
                    f(v, 3 + 3 * i, "vertex Y")? * self.scale,
                    f(v, 4 + 3 * i, "vertex Z")? * self.scale,
                ],
            });
            self.vertex_de.push(de);
            out.push(index);
        }
        Ok(out)
    }

    fn trimmed_vertex(&mut self, owner: usize, point: Vec<f64>) -> Result<usize> {
        if point.len() != 3 || point.iter().any(|value| !value.is_finite()) {
            return Err(trimmed_error(
                "BREP_IGES_V2_TRIMMED_DANGLING",
                "Invalid trimmed boundary vertex",
            ));
        }
        if let Some(index) = self.vertices.iter().position(|vertex| {
            (0..3)
                .map(|axis| (vertex.point[axis] - point[axis]).abs())
                .fold(0., f64::max)
                <= 1e-12
        }) {
            return Ok(index);
        }
        if self.vertices.len() >= MAX_TOPOLOGY {
            return Err(trimmed_error(
                "BREP_IGES_V2_TRIMMED_BUDGET",
                "Trimmed boundary vertex budget exceeded",
            ));
        }
        let index = self.vertices.len();
        self.vertices.push(Vertex {
            point: [point[0], point[1], point[2]],
        });
        self.vertex_de.push(owner);
        Ok(index)
    }

    fn directed_endpoints(&self, coedge: &Coedge) -> Result<([Vec<f64>; 2], [Vec<f64>; 2])> {
        let edge = &self.edges[coedge.edge];
        let domain = edge.curve.domain();
        let pdomain = coedge.pcurve.domain();
        let (a, b) = if coedge.reversed {
            (domain[1], domain[0])
        } else {
            (domain[0], domain[1])
        };
        Ok((
            [edge.curve.evaluate(a)?.point, edge.curve.evaluate(b)?.point],
            [
                coedge.pcurve.evaluate(pdomain[0])?.point,
                coedge.pcurve.evaluate(pdomain[1])?.point,
            ],
        ))
    }

    fn flip_trimmed_coedge(coedge: &mut Coedge) -> Result<()> {
        coedge.reversed = !coedge.reversed;
        coedge.pcurve = coedge.pcurve.reverse()?;
        Ok(())
    }

    fn set_directed_edge_endpoint(
        &mut self,
        coedge: &Coedge,
        end: bool,
        point: &[f64],
    ) -> Result<()> {
        let edge = &mut self.edges[coedge.edge];
        let vertex = match (coedge.reversed, end) {
            (false, false) | (true, true) => edge.vertices[0],
            _ => edge.vertices[1],
        };
        let control = match (coedge.reversed, end) {
            (false, false) | (true, true) => edge.curve.control_points.first_mut(),
            _ => edge.curve.control_points.last_mut(),
        }
        .ok_or_else(|| trimmed_unsupported(0, "empty boundary curve"))?;
        *control = point.to_vec();
        self.vertices[vertex].point = [point[0], point[1], point[2]];
        edge.curve.validate()?;
        Ok(())
    }

    fn close_trimmed_joint(
        &mut self,
        boundary_de: usize,
        previous: usize,
        current: usize,
        coedges: &mut [Coedge],
        owned_edges: &BTreeSet<usize>,
        surface: &Surface,
    ) -> Result<()> {
        let (previous_xyz, previous_uv) = self.directed_endpoints(&coedges[previous])?;
        let (current_xyz, current_uv) = self.directed_endpoints(&coedges[current])?;
        let curve_gap = previous_xyz[1]
            .iter()
            .zip(&current_xyz[0])
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt();
        let a = surface
            .evaluate(previous_uv[1][0], previous_uv[1][1])?
            .point;
        let b = surface.evaluate(current_uv[0][0], current_uv[0][1])?.point;
        let parameter_gap = (0..3)
            .map(|axis| (a[axis] - b[axis]) * (a[axis] - b[axis]))
            .sum::<f64>()
            .sqrt();
        let gap = curve_gap.max(parameter_gap);
        if !gap.is_finite() || gap > self.merge_tolerance_mm {
            return Err(trimmed_error(
                "BREP_IGES_V2_TRIMMED_GAP",
                format!(
                    "IGES boundary DE {boundary_de} is open between segments {previous} and {current}: gap_mm={gap:.17e}, tolerance_mm={:.17e}",
                    self.merge_tolerance_mm
                ),
            ));
        }
        if gap <= 1e-12 {
            return Ok(());
        }
        if !owned_edges.contains(&coedges[previous].edge)
            || !owned_edges.contains(&coedges[current].edge)
        {
            return Err(trimmed_unsupported(
                boundary_de,
                "gap repair on a shared 504 edge would modify an adjacent face",
            ));
        }
        let xyz = previous_xyz[1]
            .iter()
            .zip(&current_xyz[0])
            .map(|(a, b)| (a + b) * 0.5)
            .collect::<Vec<_>>();
        let uv = previous_uv[1]
            .iter()
            .zip(&current_uv[0])
            .map(|(a, b)| (a + b) * 0.5)
            .collect::<Vec<_>>();
        let previous_edge = coedges[previous].edge;
        let current_edge = coedges[current].edge;
        let previous_coedge = coedges[previous].clone();
        let current_coedge = coedges[current].clone();
        self.set_directed_edge_endpoint(&previous_coedge, true, &xyz)?;
        self.set_directed_edge_endpoint(&current_coedge, false, &xyz)?;
        if previous_edge == current_edge {
            let edge = &self.edges[previous_edge];
            edge.curve.validate()?;
        }
        coedges[previous]
            .pcurve
            .control_points
            .last_mut()
            .ok_or_else(|| trimmed_unsupported(boundary_de, "empty boundary pcurve"))?
            .clone_from(&uv);
        coedges[current]
            .pcurve
            .control_points
            .first_mut()
            .ok_or_else(|| trimmed_unsupported(boundary_de, "empty boundary pcurve"))?
            .clone_from(&uv);
        coedges[previous].pcurve.validate()?;
        coedges[current].pcurve.validate()?;
        Ok(())
    }

    fn orient_trimmed_loop(
        boundary_de: usize,
        coedges: &mut Vec<Coedge>,
        outer: bool,
    ) -> Result<()> {
        let mut points = Vec::new();
        for coedge in coedges.iter() {
            points.extend(coedge.pcurve.control_points.iter().cloned());
        }
        let mut area = 0.;
        for pair in points.windows(2) {
            area += pair[0][0] * pair[1][1] - pair[1][0] * pair[0][1];
        }
        if let (Some(first), Some(last)) = (points.first(), points.last()) {
            area += last[0] * first[1] - first[0] * last[1];
        }
        if !area.is_finite() || area.abs() <= 1e-18 {
            return Err(trimmed_unsupported(
                boundary_de,
                "boundary has zero or non-finite UV signed area",
            ));
        }
        if (outer && area < 0.) || (!outer && area > 0.) {
            coedges.reverse();
            for coedge in coedges {
                Self::flip_trimmed_coedge(coedge)?;
            }
        }
        Ok(())
    }

    fn trimmed_loop(
        &mut self,
        de: usize,
        expected_surface: usize,
        curve_edges: &BTreeMap<usize, usize>,
        owned_edges: &mut BTreeSet<usize>,
        work: &mut usize,
    ) -> Result<usize> {
        let entity = self.e.get(&de).ok_or_else(|| {
            trimmed_error(
                "BREP_IGES_V2_TRIMMED_DANGLING",
                format!("Missing IGES boundary DE {de}"),
            )
        })?;
        let (surface, segments) = trimmed_boundary_segments(de, entity)?;
        if surface != expected_surface {
            return Err(trimmed_error(
                "BREP_IGES_V2_TRIMMED_DANGLING",
                format!(
                    "IGES boundary DE {de} surface DE {surface} differs from trimmed surface DE {expected_surface}"
                ),
            ));
        }
        let surface = surface128(self.e, expected_surface, self.scale)?;
        let mut coedges = Vec::with_capacity(segments.len());
        for segment in segments {
            if entity.directory.ty == 142 && segment.0 == 0 && segment.1 != 0 {
                let uv = self.curve(segment.1, 2)?;
                let remaining = MAX_GRAPH_WORK.saturating_sub(*work);
                let lift = nurbs_core::editing::exact_projective::lift_pcurve_to_surface(&surface, &uv, (remaining as u64).min(1_000_000))?;
                graph_work(work, lift.work_used as usize)?;
                let pieces = lift.pieces.ok_or_else(|| trimmed_unsupported(de, "UV boundary image could not be certified"))?;
                for piece in pieces {
                    let [a,b] = piece.domain();
                    let pcurve = if [a,b] == uv.domain() { uv.clone() } else { uv.trim(a,b)? };
                    let (edge, reversed) = self.match_or_create_boundary_edge(de, piece, work)?;
                    coedges.push(Coedge::new(edge, reversed, pcurve));
                }
            } else if entity.directory.ty == 142 && segment.0 != 0 && segment.1 == 0 {
                let curve = self.curve(segment.0, 3)?;
                let uv = affine_boundary::recover(de, &surface, &curve, work)?;
                let (edge, reversed) = if let Some(index) = curve_edges.get(&segment.0) {
                    (*index, false)
                } else {
                    self.match_or_create_boundary_edge(de, curve, work)?
                };
                coedges.push(Coedge::new(edge, reversed, uv));
            } else {
                coedges.push(self.trimmed_boundary_coedge(de, segment, curve_edges, owned_edges)?);
            }
        }
        for index in 1..coedges.len() {
            self.chain_trimmed_segment(de, index, &mut coedges)?;
        }
        for index in 0..coedges.len() {
            let next = (index + 1) % coedges.len();
            self.close_trimmed_joint(de, index, next, &mut coedges, owned_edges, &surface)?;
        }
        if coedges.len() > MAX_TOPOLOGY {
            return Err(trimmed_error(
                "BREP_IGES_V2_TRIMMED_BUDGET",
                format!("IGES boundary DE {de} exceeds topology budget"),
            ));
        }
        let index = self.loops.len();
        self.loops.push(Loop { coedges });
        self.loop_de.push(de);
        Ok(index)
    }

    fn trimmed_boundary_coedge(
        &mut self,
        de: usize,
        (curve_de, pcurve_de, sense): BoundarySegment,
        curve_edges: &BTreeMap<usize, usize>,
        owned_edges: &mut BTreeSet<usize>,
    ) -> Result<Coedge> {
        if curve_de == 0 || pcurve_de == 0 {
            return Err(trimmed_unsupported(
                de,
                "boundary requires both 3D curve and pcurve; projection is not inferred",
            ));
        }
        if !matches!(sense, 1 | 2) {
            return Err(trimmed_unsupported(de, "boundary sense must be 1 or 2"));
        }
        let curve = self.curve(curve_de, 3)?;
        let pcurve = self.curve(pcurve_de, 2)?;
        let edge = if let Some(edge) = curve_edges.get(&curve_de) {
            *edge
        } else {
            let domain = curve.domain();
            let a = self.trimmed_vertex(de, curve.evaluate(domain[0])?.point)?;
            let b = self.trimmed_vertex(de, curve.evaluate(domain[1])?.point)?;
            let edge = self.edges.len();
            self.edges.push(Edge {
                degenerate: a == b && constant_curve(&curve),
                vertices: [a, b],
                curve,
            });
            self.edge_de.push(de);
            owned_edges.insert(edge);
            edge
        };
        Ok(Coedge::new(edge, sense == 2, pcurve))
    }

    /// Orients segment `index` so that it starts where segment `index - 1`
    /// ends, flipping the first segment once when only its start matches.
    fn chain_trimmed_segment(&self, de: usize, index: usize, coedges: &mut [Coedge]) -> Result<()> {
        let tolerance = self.merge_tolerance_mm;
        let (mut previous_xyz, _) =
            self.directed_endpoints(boundary_segment(coedges, de, index - 1)?)?;
        let (current_xyz, _) = self.directed_endpoints(boundary_segment(coedges, de, index)?)?;
        let mut end_start = point_distance(&previous_xyz[1], &current_xyz[0]);
        let mut end_end = point_distance(&previous_xyz[1], &current_xyz[1]);
        if index == 1 && end_start > tolerance && end_end > tolerance {
            let start_start = point_distance(&previous_xyz[0], &current_xyz[0]);
            let start_end = point_distance(&previous_xyz[0], &current_xyz[1]);
            if start_start <= tolerance || start_end <= tolerance {
                Self::flip_trimmed_coedge(boundary_segment_mut(coedges, de, 0)?)?;
                (previous_xyz, _) = self.directed_endpoints(boundary_segment(coedges, de, 0)?)?;
                end_start = point_distance(&previous_xyz[1], &current_xyz[0]);
                end_end = point_distance(&previous_xyz[1], &current_xyz[1]);
            }
        }
        if end_start <= tolerance && end_end <= tolerance {
            return Err(trimmed_error(
                "BREP_IGES_V2_TRIMMED_AMBIGUOUS",
                format!("IGES boundary DE {de} has ambiguous orientation at segment {index}"),
            ));
        }
        if end_start > tolerance {
            if end_end > tolerance {
                let gap = end_start.min(end_end);
                return Err(trimmed_error(
                    "BREP_IGES_V2_TRIMMED_GAP",
                    format!(
                        "IGES boundary DE {de} is open at segment {index}: gap_mm={gap:.17e}, tolerance_mm={tolerance:.17e}"
                    ),
                ));
            }
            Self::flip_trimmed_coedge(boundary_segment_mut(coedges, de, index)?)?;
        }
        Ok(())
    }

    fn build(mut self) -> Result<(Model, TopologyMapping)> {
        self.reject_boundary_141()?;
        let vertex_lists = self.collect_vertex_lists()?;
        let (edge_lists, curve_edges) = self.build_edge_lists(&vertex_lists)?;
        let loop_map = self.build_edge_loops(&edge_lists)?;
        let mut face_map = self.build_loop_faces(&loop_map)?;
        self.build_trimmed_faces(&curve_edges, &mut face_map)?;
        let shell_map = self.build_shells(&face_map)?;
        self.build_bodies(&shell_map)?;
        self.finish()
    }

    fn reject_boundary_141(&self) -> Result<()> {
        if let Some((&de, _)) = self.e.iter().find(|(_, e)| e.directory.ty == 141) {
            return Err(trimmed_unsupported(
                de,
                "IGES 141 boundary entities are not admitted; use 143 or 142 with both curve representations",
            ));
        }
        Ok(())
    }

    fn entities_of_type(&self, ty: usize) -> impl Iterator<Item = (usize, &'a Entity)> + use<'a> {
        let entities: &'a BTreeMap<usize, Entity> = self.e;
        entities
            .iter()
            .filter(move |(_, e)| e.directory.ty == ty)
            .map(|(&de, e)| (de, e))
    }

    fn collect_vertex_lists(&mut self) -> Result<BTreeMap<usize, Vec<usize>>> {
        let mut vertex_lists = BTreeMap::new();
        for (de, _) in self.entities_of_type(502) {
            vertex_lists.insert(de, self.vertex_list(de)?);
        }
        Ok(vertex_lists)
    }

    fn build_edge_lists(
        &mut self,
        vertex_lists: &BTreeMap<usize, Vec<usize>>,
    ) -> Result<(EdgeListIndex, BTreeMap<usize, usize>)> {
        let mut edge_lists = BTreeMap::<(usize, usize), usize>::new();
        let mut curve_edges = BTreeMap::<usize, usize>::new();
        for (de, e) in self.entities_of_type(504) {
            let v = &e.values;
            let n = u(v, 1, "edge count")?;
            if v.len() != 2 + 5 * n {
                return Err(refuse("Malformed IGES edge list"));
            }
            for i in 0..n {
                let at = 2 + 5 * i;
                let curve_de = u(v, at, "edge curve")?;
                let curve = self.curve(curve_de, 3)?;
                let vl0 = u(v, at + 1, "start vertex list")?;
                let vi0 = u(v, at + 2, "start vertex index")?;
                let vl1 = u(v, at + 3, "end vertex list")?;
                let vi1 = u(v, at + 4, "end vertex index")?;
                let a = list_vertex(vertex_lists, vl0, vi0)
                    .ok_or_else(|| refuse("Invalid IGES start vertex index"))?;
                let b = list_vertex(vertex_lists, vl1, vi1)
                    .ok_or_else(|| refuse("Invalid IGES end vertex index"))?;
                let index = self.edges.len();
                self.edges.push(Edge {
                    degenerate: a == b && constant_curve(&curve),
                    vertices: [a, b],
                    curve,
                });
                self.edge_de.push(de);
                edge_lists.insert((de, i + 1), index);
                curve_edges.insert(curve_de, index);
            }
        }
        Ok((edge_lists, curve_edges))
    }

    fn build_edge_loops(
        &mut self,
        edge_lists: &BTreeMap<(usize, usize), usize>,
    ) -> Result<BTreeMap<usize, usize>> {
        let mut loop_map = BTreeMap::new();
        for (de, e) in self.entities_of_type(508) {
            let v = &e.values;
            let n = u(v, 1, "loop member count")?;
            let mut at = 2;
            let mut coedges = Vec::new();
            for _ in 0..n {
                coedges.push(self.edge_loop_member(v, at, edge_lists)?);
                at += LOOP_MEMBER_WIDTH;
            }
            if at != v.len() {
                return Err(refuse("Trailing IGES loop parameters"));
            }
            let index = self.loops.len();
            self.loops.push(Loop { coedges });
            self.loop_de.push(de);
            loop_map.insert(de, index);
        }
        Ok(loop_map)
    }

    fn edge_loop_member(
        &self,
        v: &[String],
        at: usize,
        edge_lists: &BTreeMap<(usize, usize), usize>,
    ) -> Result<Coedge> {
        if u(v, at, "loop member type")? != 0 {
            return Err(refuse("Vertex-only IGES loop member is unsupported"));
        }
        let el = u(v, at + 1, "loop edge list")?;
        let ei = u(v, at + 2, "loop edge index")?;
        let reversed = u(v, at + 3, "loop orientation")? == 0;
        let pcount = u(v, at + 4, "loop pcurve count")?;
        if pcount != 1 {
            return Err(refuse("Each IGES loop edge requires exactly one pcurve"));
        }
        let iso = u(v, at + 5, "loop pcurve isoparametric")?;
        if iso > 1 {
            return Err(refuse("Invalid IGES isoparametric flag"));
        }
        let pcurve = self.curve(u(v, at + 6, "loop pcurve")?, 2)?;
        let edge = *edge_lists
            .get(&(el, ei))
            .ok_or_else(|| refuse("Invalid IGES edge-list reference"))?;
        Ok(Coedge::new(edge, reversed, pcurve))
    }

    fn verify_face_loops(&self, loops: &[usize], surface: &Surface) -> Result<()> {
        for loop_index in loops {
            let face_loop = self
                .loops
                .get(*loop_index)
                .ok_or_else(|| refuse("Missing IGES face loop"))?;
            for coedge in &face_loop.coedges {
                let edge = self
                    .edges
                    .get(coedge.edge)
                    .ok_or_else(|| refuse("Invalid IGES edge-list reference"))?;
                verify_correspondence(edge, &coedge.pcurve, surface, coedge.reversed)?;
            }
        }
        Ok(())
    }

    fn push_face(
        &mut self,
        de: usize,
        surface: Surface,
        loops: &[usize],
        face_map: &mut BTreeMap<usize, usize>,
    ) -> Result<()> {
        let (&outer, holes) = loops
            .split_first()
            .ok_or_else(|| refuse("Malformed IGES face"))?;
        let index = self.faces.len();
        self.faces.push(Face {
            surface,
            outer,
            holes: holes.to_vec(),
        });
        self.face_de.push(de);
        face_map.insert(de, index);
        Ok(())
    }

    fn build_loop_faces(
        &mut self,
        loop_map: &BTreeMap<usize, usize>,
    ) -> Result<BTreeMap<usize, usize>> {
        let mut face_map = BTreeMap::new();
        for (de, e) in self.entities_of_type(510) {
            let v = &e.values;
            let surface = surface128(self.e, u(v, 1, "face surface")?, self.scale)?;
            let n = u(v, 2, "face loop count")?;
            if n == 0 || v.len() != 4 + n {
                return Err(refuse("Malformed IGES face"));
            }
            let loops = (0..n)
                .map(|i| {
                    loop_map
                        .get(&u(v, 4 + i, "face loop")?)
                        .copied()
                        .ok_or_else(|| refuse("Missing IGES face loop"))
                })
                .collect::<Result<Vec<_>>>()?;
            self.verify_face_loops(&loops, &surface)?;
            self.push_face(de, surface, &loops, &mut face_map)?;
        }
        Ok(face_map)
    }

    fn build_trimmed_faces(
        &mut self,
        curve_edges: &BTreeMap<usize, usize>,
        face_map: &mut BTreeMap<usize, usize>,
    ) -> Result<()> {
        let referenced_boundaries = referenced_trimmed_boundaries(self.e);
        let mut owned_edges = BTreeSet::new();
        let mut boundary_work = 0;
        let entities: &'a BTreeMap<usize, Entity> = self.e;
        for (&de, e) in entities {
            if !matches!(e.directory.ty, 143 | 144)
                || (e.directory.ty == 143 && referenced_boundaries.contains(&de))
            {
                continue;
            }
            let (surface_de, boundaries) = trimmed_face_boundaries(de, e)?;
            let mut loops = Vec::with_capacity(boundaries.len());
            let surface = surface128(self.e, surface_de, self.scale)?;
            if e.directory.ty == 144 && u(&e.values, 2, "trimmed surface outer flag")? == 0 {
                loops.push(self.natural_outer_loop(de, &surface, &mut boundary_work)?);
            }
            for boundary in boundaries {
                let index =
                    self.trimmed_loop(boundary, surface_de, curve_edges, &mut owned_edges, &mut boundary_work)?;
                let coedges = &mut self
                    .loops
                    .get_mut(index)
                    .ok_or_else(|| refuse("Missing IGES face loop"))?
                    .coedges;
                Self::orient_trimmed_loop(boundary, coedges, loops.is_empty())?;
                loops.push(index);
            }
            self.verify_face_loops(&loops, &surface).map_err(|error| {
                trimmed_error(
                    "BREP_IGES_V2_TRIMMED_DANGLING",
                    format!("IGES trimmed/bounded DE {de}: {}", error.message),
                )
            })?;
            self.push_face(de, surface, &loops, face_map)?;
        }
        Ok(())
    }

    fn build_shells(
        &mut self,
        face_map: &BTreeMap<usize, usize>,
    ) -> Result<BTreeMap<usize, usize>> {
        let mut shell_map = BTreeMap::new();
        for (de, e) in self.entities_of_type(514) {
            let v = &e.values;
            let n = u(v, 1, "shell face count")?;
            if n == 0 || v.len() != 2 + 2 * n {
                return Err(refuse("Malformed IGES shell"));
            }
            let faces = (0..n)
                .map(|i| {
                    Ok(FaceUse {
                        face: *face_map
                            .get(&u(v, 2 + 2 * i, "shell face")?)
                            .ok_or_else(|| refuse("Missing IGES shell face"))?,
                        reversed: u(v, 3 + 2 * i, "shell orientation")? == 0,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let index = self.shells.len();
            self.shells.push(Shell {
                faces,
                // IGES 514 form 1 is an open shell; form 2 (and legacy form 0) is closed.
                closed: e.directory.form != 1,
            });
            self.shell_de.push(de);
            shell_map.insert(de, index);
        }
        Ok(shell_map)
    }

    fn build_bodies(&mut self, shell_map: &BTreeMap<usize, usize>) -> Result<()> {
        for (de, e) in self.entities_of_type(186) {
            let v = &e.values;
            let outer = *shell_map
                .get(&u(v, 1, "solid outer shell")?)
                .ok_or_else(|| refuse("Missing IGES outer shell"))?;
            let n = u(v, 3, "solid void count")?;
            if v.len() != 4 + 2 * n {
                return Err(refuse("Malformed IGES manifold solid B-rep"));
            }
            let inners = (0..n)
                .map(|i| {
                    shell_map
                        .get(&u(v, 4 + 2 * i, "void shell")?)
                        .copied()
                        .ok_or_else(|| refuse("Missing IGES void shell"))
                })
                .collect::<Result<Vec<_>>>()?;
            self.bodies.push(Body {
                outer_shell: outer,
                inner_shells: inners,
            });
            self.body_de.push(de);
        }
        Ok(())
    }

    fn finish(self) -> Result<(Model, TopologyMapping)> {
        if self.bodies.is_empty() && self.shells.iter().all(|shell| shell.closed) {
            return Err(refuse(
                "IGES contains neither a manifold solid B-rep (186) nor an open shell (514 form 1)",
            ));
        }
        let count = self.vertices.len()
            + self.edges.len()
            + self.loops.len()
            + self.faces.len()
            + self.shells.len()
            + self.bodies.len();
        if count > MAX_TOPOLOGY {
            return Err(refuse("IGES topology budget exceeds 32768 entities"));
        }
        let mut model = Model(
            brep_topology::Model {
                vertices: self.vertices,
                edges: self.edges,
                loops: self.loops,
                faces: self.faces,
                shells: self.shells,
                bodies: self.bodies,
                tolerance_mm: IMPORT_TOLERANCE_MM,
            },
            TopologyIds::default(),
        );
        model.rebuild_topology_ids();
        model.validate()?;
        let mut map = Vec::new();
        for (kind, des) in [
            (TopoKind::Vertex, &self.vertex_de),
            (TopoKind::Edge, &self.edge_de),
            (TopoKind::Loop, &self.loop_de),
            (TopoKind::Face, &self.face_de),
            (TopoKind::Shell, &self.shell_de),
            (TopoKind::Body, &self.body_de),
        ] {
            map.extend(des.iter().enumerate().map(|(i, de)| (kind, i, *de)));
        }
        Ok((model, map))
    }
}

/// One IGES boundary segment: model curve DE, parameter curve DE and sense.
type BoundarySegment = (usize, usize, usize);

/// Reads the surface DE and the segments of an IGES 143 or 142 boundary.
fn trimmed_boundary_segments(de: usize, entity: &Entity) -> Result<(usize, Vec<BoundarySegment>)> {
    let v = &entity.values;
    match entity.directory.ty {
        143 => {
            let kind = u(v, 1, "bounded surface boundary type")?;
            let preference = u(v, 2, "bounded surface preference")?;
            if !matches!(kind, 0 | 2) || matches!(preference, 1) {
                return Err(trimmed_unsupported(
                    de,
                    "3D-only boundary without reliable parameterization",
                ));
            }
            if !matches!(preference, 0 | 2 | 3) {
                return Err(trimmed_unsupported(de, "unknown representation preference"));
            }
            let n = u(v, 4, "bounded surface segment count")?;
            if n == 0 || n > MAX_COMPOSITE_SEGMENTS || v.len() != 5 + 3 * n {
                return Err(trimmed_error(
                    "BREP_IGES_V2_TRIMMED_BUDGET",
                    format!("Invalid or excessive IGES boundary DE {de} segment count {n}"),
                ));
            }
            Ok((
                u(v, 3, "bounded surface geometry")?,
                (0..n)
                    .map(|i| {
                        Ok((
                            u(v, 5 + 3 * i, "boundary model curve")?,
                            u(v, 6 + 3 * i, "boundary pcurve")?,
                            u(v, 7 + 3 * i, "boundary sense")?,
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
            ))
        }
        142 => {
            if v.len() != 6 {
                return Err(trimmed_error(
                    "BREP_IGES_V2_TRIMMED_DANGLING",
                    format!("Malformed IGES curve-on-surface DE {de}"),
                ));
            }
            let preference = u(v, 5, "curve-on-surface preference")?;
            if preference > 3 {
                return Err(trimmed_unsupported(
                    de,
                    "unknown curve-on-surface representation preference",
                ));
            }
            Ok((
                u(v, 2, "curve-on-surface geometry")?,
                vec![(
                    u(v, 4, "boundary model curve")?,
                    u(v, 3, "boundary pcurve")?,
                    1,
                )],
            ))
        }
        _ => Err(trimmed_unsupported(
            de,
            format!(
                "boundary type {} is unsupported; expected 143 or 142",
                entity.directory.ty
            ),
        )),
    }
}

fn point_distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

fn missing_boundary_segment(de: usize, index: usize) -> Error {
    trimmed_error(
        "BREP_IGES_V2_TRIMMED_DANGLING",
        format!("IGES boundary DE {de} has no segment {index}"),
    )
}

fn boundary_segment(coedges: &[Coedge], de: usize, index: usize) -> Result<&Coedge> {
    coedges
        .get(index)
        .ok_or_else(|| missing_boundary_segment(de, index))
}

fn boundary_segment_mut(coedges: &mut [Coedge], de: usize, index: usize) -> Result<&mut Coedge> {
    coedges
        .get_mut(index)
        .ok_or_else(|| missing_boundary_segment(de, index))
}

/// Maps an IGES 504 edge list DE and its 1-based member index to an edge.
type EdgeListIndex = BTreeMap<(usize, usize), usize>;

const LOOP_MEMBER_WIDTH: usize = 7;
const IMPORT_TOLERANCE_MM: f64 = 1e-7;

fn list_vertex(
    vertex_lists: &BTreeMap<usize, Vec<usize>>,
    list: usize,
    index: usize,
) -> Option<usize> {
    vertex_lists.get(&list)?.get(index.checked_sub(1)?).copied()
}

fn referenced_trimmed_boundaries(entities: &BTreeMap<usize, Entity>) -> BTreeSet<usize> {
    entities
        .values()
        .filter(|entity| entity.directory.ty == 144)
        .flat_map(|entity| {
            let v = &entity.values;
            let n = u(v, 3, "trimmed surface inner boundary count").unwrap_or(0);
            (4..5 + n).filter_map(move |at| u(v, at, "trimmed boundary").ok())
        })
        .collect()
}

fn trimmed_face_boundaries(de: usize, e: &Entity) -> Result<(usize, Vec<usize>)> {
    let v = &e.values;
    if e.directory.ty != 144 {
        return Ok((u(v, 3, "bounded surface geometry")?, vec![de]));
    }
    if v.len() < 5 {
        return Err(trimmed_error(
            "BREP_IGES_V2_TRIMMED_DANGLING",
            format!("Malformed IGES trimmed surface DE {de}"),
        ));
    }
    let flag = u(v, 2, "trimmed surface outer flag")?;
    if flag > 1 {
        return Err(trimmed_unsupported(de, "invalid outer boundary flag"));
    }
    let n = u(v, 3, "trimmed surface inner boundary count")?;
    if n > MAX_TOPOLOGY || v.len() != 5 + n {
        return Err(trimmed_error(
            "BREP_IGES_V2_TRIMMED_BUDGET",
            format!("Invalid or excessive IGES trimmed DE {de} boundary count {n}"),
        ));
    }
    let outer = u(v, 4, "trimmed surface outer boundary")?;
    if (outer == 0) != (flag == 0) {
        return Err(trimmed_error(
            "BREP_IGES_V2_TRIMMED_DANGLING",
            format!("Inconsistent outer boundary in IGES trimmed surface DE {de}"),
        ));
    }
    Ok((
        u(v, 1, "trimmed surface geometry")?,
        ((if flag == 0 { 5 } else { 4 })..5 + n)
            .map(|at| u(v, at, "trimmed boundary"))
            .collect::<Result<Vec<_>>>()?,
    ))
}

fn restore_identity(
    entities: &BTreeMap<usize, Entity>,
    map: &[(TopoKind, usize, usize)],
    model: &mut Model,
) -> Result<crate::step_interchange::StepIdentityReport> {
    let mut metadata = BTreeMap::new();
    for e in entities
        .values()
        .filter(|e| e.directory.ty == 406 && e.directory.form == 15)
    {
        if e.values.len() != 4 {
            continue;
        }
        let Some(payload) = e.values[2].strip_prefix("OSCAD_TOPO/IGES2|") else {
            continue;
        };
        let p = payload.split('|').collect::<Vec<_>>();
        if p.len() != 4 {
            return Err(refuse("Malformed IGES identity property"));
        }
        let kind = TopoKind::parse(p[0]).ok_or_else(|| refuse("Unknown IGES identity kind"))?;
        let id = TopoId::parse(p[1]).map_err(refuse)?;
        let index = p[3]
            .parse::<usize>()
            .map_err(|_| refuse("Invalid IGES identity index"))?;
        let target = u(&e.values, 3, "identity target")?;
        let target_entity = entities
            .get(&target)
            .ok_or_else(|| refuse("Missing IGES identity target"))?;
        if id.kind() != kind || p[2] != topology_digest(target_entity) {
            return Err(refuse("Transplanted IGES identity metadata"));
        }
        if metadata.insert((kind, target, index), id).is_some() {
            return Err(refuse("Duplicate IGES identity property"));
        }
    }
    let count = map.len();
    if metadata.is_empty() {
        return Ok(crate::step_interchange::StepIdentityReport {
            preserved: false,
            source: "external-iges",
            preserved_count: 0,
            created_count: count,
            lost_count: count,
        });
    }
    if metadata.len() != count {
        return Err(refuse("Partial IGES identity metadata"));
    }
    let mut seen = BTreeSet::new();
    let mut remap = BTreeMap::new();
    let mut ordinal = BTreeMap::<(TopoKind, usize), usize>::new();
    for &(kind, index, de) in map {
        let occurrence = ordinal.entry((kind, de)).or_insert(0);
        let id = *metadata
            .get(&(kind, de, *occurrence))
            .ok_or_else(|| refuse("IGES identity map is not topology-bijective"))?;
        *occurrence += 1;
        if !seen.insert(id) {
            return Err(refuse("Duplicate IGES TopoId"));
        }
        let old = match kind {
            TopoKind::Vertex => model.1.vertices[index],
            TopoKind::Edge => model.1.edges[index],
            TopoKind::Loop => model.1.loops[index],
            TopoKind::Face => model.1.faces[index],
            TopoKind::Shell => model.1.shells[index],
            TopoKind::Body => model.1.bodies[index],
            TopoKind::ControlPoint => unreachable!(),
        };
        remap.insert(old, id);
        match kind {
            TopoKind::Vertex => model.1.vertices[index] = id,
            TopoKind::Edge => model.1.edges[index] = id,
            TopoKind::Loop => model.1.loops[index] = id,
            TopoKind::Face => model.1.faces[index] = id,
            TopoKind::Shell => model.1.shells[index] = id,
            TopoKind::Body => model.1.bodies[index] = id,
            TopoKind::ControlPoint => unreachable!(),
        }
    }
    model.1.change_set.nodes = model
        .1
        .change_set
        .nodes
        .iter()
        .map(|(id, kind)| (remap.get(id).copied().unwrap_or(*id), *kind))
        .collect();
    for change in &mut model.1.change_set.changes {
        for id in change.parents.iter_mut().chain(&mut change.children) {
            if let Some(next) = remap.get(id) {
                *id = *next
            }
        }
    }
    for relation in &mut model.1.lineage {
        for id in relation.parents.iter_mut().chain(&mut relation.children) {
            if let Some(next) = remap.get(id) {
                *id = *next
            }
        }
    }
    Ok(crate::step_interchange::StepIdentityReport {
        preserved: true,
        source: "internal-metadata",
        preserved_count: count,
        created_count: 0,
        lost_count: 0,
    })
}

/// Conservative maximum axis-aligned extent of vertices and 3D control hulls.
/// Two-dimensional pcurve control points are deliberately not included.
fn geometric_extent_mm(model: &Model) -> f64 {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    let mut include = |point: &[f64]| {
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    };
    for vertex in &model.vertices {
        include(&vertex.point);
    }
    for edge in &model.edges {
        for point in &edge.curve.control_points {
            include(point);
        }
    }
    for face in &model.faces {
        for point in face.surface.control_points.iter().flatten() {
            include(point);
        }
    }
    (0..3).map(|axis| max[axis] - min[axis]).fold(0., f64::max)
}

/// Broad diagnostic band for mechanical parts: 1 micrometre through 10 metres.
/// It never selects or changes units; it only gates an inch/millimeter warning.
fn reasonable_mechanical_extent(extent_mm: f64) -> bool {
    (1e-3..=1e4).contains(&extent_mm)
}

fn unit_mismatch_warning(
    model: &Model,
    declared: IgesLengthUnit,
    applied: IgesLengthUnit,
) -> Option<String> {
    let alternative = match declared {
        IgesLengthUnit::Inch => IgesLengthUnit::Millimeter,
        IgesLengthUnit::Millimeter => IgesLengthUnit::Inch,
        _ => return None,
    };
    let raw_extent = geometric_extent_mm(model) / applied.scale_to_mm();
    let declared_extent = raw_extent * declared.scale_to_mm();
    let alternative_extent = raw_extent * alternative.scale_to_mm();
    if reasonable_mechanical_extent(declared_extent)
        || !reasonable_mechanical_extent(alternative_extent)
    {
        return None;
    }
    Some(format!(
        "Possible IGES unit mismatch: declared {:?} (scale {} mm/unit) gives control-hull/vertex extent {} mm, outside the mechanical-part band [0.001, 10000] mm; alternative {:?} (scale {} mm/unit) gives extent {} mm inside the band. Diagnostic only; units are not changed automatically.",
        declared,
        declared.scale_to_mm(),
        declared_extent,
        alternative,
        alternative.scale_to_mm(),
        alternative_extent,
    ))
}

/// Read a spatial curve selected by its IGES directory entry.
pub fn import_iges_curve(text: &str, directory_entry: usize) -> Result<Curve> {
    let (entities, unit, resolution) = parse(text)?;
    let scale = unit.scale_to_mm();
    let tolerance_mm = resolution * scale;
    if !tolerance_mm.is_finite() || tolerance_mm <= 0. {
        return Err(refuse("Invalid scaled IGES curve tolerance"));
    }
    CurveDispatcher { entities: &entities, dim: 3, scale, tolerance_mm, work: 0 }
        .dispatch(directory_entry)
}

/// Read a rational surface selected by its IGES directory entry.
pub fn import_iges_surface(text: &str, directory_entry: usize) -> Result<Surface> {
    let (entities, unit, _) = parse(text)?;
    surface128(&entities, directory_entry, unit.scale_to_mm())
}

pub fn import_iges_v2(text: &str) -> Result<(Model, FeatureCertificate, IgesV2Report)> {
    import_iges_v2_with_options(text, IgesV2ImportOptions::default())
}

pub fn import_iges_v2_with_options(
    text: &str,
    options: IgesV2ImportOptions,
) -> Result<(Model, FeatureCertificate, IgesV2Report)> {
    let (mut entities, declared_unit, resolution) = parse(text)?;
    let entity_count = entities.len();
    let discarded = filter_discarded_entities(&mut entities)?;
    let applied_unit = options.unit_override.unwrap_or(declared_unit);
    let scale = applied_unit.scale_to_mm();
    let merge_tolerance_mm = resolution * scale;
    if !merge_tolerance_mm.is_finite() || merge_tolerance_mm <= 0. {
        return Err(refuse("Invalid scaled IGES composite merge tolerance"));
    }
    const GEOMETRIC: &[usize] = &[
        100, 102, 110, 124, 126, 128, 141, 142, 143, 144, 186, 502, 504, 508, 510, 514,
    ];
    const METADATA: &[usize] = &[314, 402, 406];
    let mut ignored = BTreeSet::new();
    for e in entities.values() {
        if !GEOMETRIC.contains(&e.directory.ty) && !METADATA.contains(&e.directory.ty) {
            return Err(refuse(format!(
                "IGES entity {} is outside the finite direct subset",
                e.directory.ty
            )));
        }
        if METADATA.contains(&e.directory.ty) {
            ignored.insert(format!("entity-{}", e.directory.ty));
        }
        if e.directory.color != 0 {
            ignored.insert("directory-color".into());
        }
        if !e.directory.label.is_empty() {
            ignored.insert("directory-label".into());
        }
        if e.directory.transform != 0 {
            if !matches!(e.directory.ty, 100 | 102 | 110 | 124 | 126 | 128) {
                return Err(refuse("IGES directory placement is supported only on curves, spline surfaces and placement matrices"));
            }
            placement::read(&entities, e.directory.transform)?;
        }
    }
    let builder = Builder {
        e: &entities,
        scale,
        merge_tolerance_mm,
        vertices: vec![],
        edges: vec![],
        loops: vec![],
        faces: vec![],
        shells: vec![],
        bodies: vec![],
        vertex_de: vec![],
        edge_de: vec![],
        loop_de: vec![],
        face_de: vec![],
        shell_de: vec![],
        body_de: vec![],
    };
    let (mut model, map) = builder.build()?;
    let identity = restore_identity(&entities, &map, &mut model)?;
    let topology_count = map.len();
    let warning = unit_mismatch_warning(&model, declared_unit, applied_unit);
    Ok((
        model,
        FeatureCertificate {
            capability: IGES_INTERCHANGE_V2_CAPABILITY,
            complete: true,
            notes: vec![
                "strict_80_column_sgdp_t",
                "direct_126_128_502_504_508_510_514_186",
                "shared_topology_and_pcurves",
                "multi_body_and_cavity",
                "no_aabb_constructor_or_mesh",
            ],
        },
        IgesV2Report {
            identity,
            ignored_metadata: ignored.into_iter().collect(),
            entity_count,
            topology_count,
            units: IgesV2UnitReport {
                declared_unit,
                applied_unit,
                declared_flag: declared_unit.flag(),
                declared_scale_to_mm: declared_unit.scale_to_mm(),
                applied_scale_to_mm: applied_unit.scale_to_mm(),
                overridden: applied_unit != declared_unit,
                warning,
            },
            discarded,
        },
    ))
}

#[derive(Clone)]
struct OutEntity {
    ty: usize,
    form: usize,
    label: String,
    color: i32,
    values: Vec<String>,
}
struct Writer {
    entities: Vec<OutEntity>,
}
impl Writer {
    fn new() -> Self {
        Self { entities: vec![] }
    }
    fn emit(&mut self, ty: usize, values: Vec<String>) -> usize {
        self.entities.push(OutEntity {
            ty,
            form: 0,
            label: String::new(),
            color: 0,
            values,
        });
        2 * self.entities.len() - 1
    }
}
fn number(x: f64) -> String {
    format!("{x:.17e}")
}
fn curve_values(c: &Curve, dim: usize) -> Result<Vec<String>> {
    let (start, end) = CurveDispatcher::endpoints(c)?;
    let n = c.control_points.len();
    let mut v = vec![
        "126".into(),
        (n - 1).to_string(),
        c.degree.to_string(),
        "0".into(),
        ((c.periodic || start == end) as usize).to_string(),
        (c.weights.iter().all(|weight| *weight == c.weights[0]) as usize).to_string(),
        (c.periodic as usize).to_string(),
    ];
    v.extend(c.knots.iter().map(|x| number(*x)));
    v.extend(c.weights.iter().map(|x| number(*x)));
    for p in &c.control_points {
        v.push(number(p[0]));
        v.push(number(p[1]));
        v.push(number(if dim == 3 { p[2] } else { 0. }));
    }
    let d = c.domain();
    v.extend([
        number(d[0]),
        number(d[1]),
        "0".into(),
        "0".into(),
        "1".into(),
    ]);
    Ok(v)
}
fn surface_values(s: &Surface) -> Result<Vec<String>> {
    let nu = s.control_points.len();
    let nv = s.control_points[0].len();
    let closed_u = s.periodic_u || s.iso(Axis::U, s.knots_u[s.degree_u])? == s.iso(Axis::U, s.knots_u[nu])?;
    let closed_v = s.periodic_v || s.iso(Axis::V, s.knots_v[s.degree_v])? == s.iso(Axis::V, s.knots_v[nv])?;
    let polynomial = s.weights.iter().flatten().all(|w| *w == s.weights[0][0]);
    let mut v = vec![
        "128".into(),
        (nu - 1).to_string(),
        (nv - 1).to_string(),
        s.degree_u.to_string(),
        s.degree_v.to_string(),
        (closed_u as usize).to_string(),
        (closed_v as usize).to_string(),
        (polynomial as usize).to_string(),
        (s.periodic_u as usize).to_string(),
        (s.periodic_v as usize).to_string(),
    ];
    v.extend(s.knots_u.iter().map(|x| number(*x)));
    v.extend(s.knots_v.iter().map(|x| number(*x)));
    for j in 0..nv {
        v.extend((0..nu).map(|i| number(s.weights[i][j])));
    }
    for j in 0..nv {
        for row in &s.control_points {
            v.extend(row[j].iter().map(|x| number(*x)));
        }
    }
    let u = [
        s.knots_u[s.degree_u],
        s.knots_u[s.knots_u.len() - s.degree_u - 1],
    ];
    let q = [
        s.knots_v[s.degree_v],
        s.knots_v[s.knots_v.len() - s.degree_v - 1],
    ];
    v.extend([number(u[0]), number(u[1]), number(q[0]), number(q[1])]);
    Ok(v)
}
fn canonical(values: &[String]) -> String {
    values
        .iter()
        .map(|v| v.trim())
        .collect::<Vec<_>>()
        .join(",")
}
fn digest_values(values: &[String]) -> String {
    let mut h = 0xcbf29ce484222325u64;
    for b in canonical(values).bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn pad_record(payload: &str, section: char, sequence: usize, width: usize) -> Result<String> {
    if payload.len() > width {
        return Err(refuse("IGES physical payload overflow"));
    }
    Ok(format!(
        "{payload:<width$}{section}{sequence:>7}",
        width = width
    ))
}
fn hollerith(s: &str) -> String {
    format!("{}H{s}", s.len())
}
// Numeric parameters must fit in one physical record. Hollerith metadata may
// span records and retains every payload byte, including spaces.
fn parameter_records(ty: usize, values: &[String]) -> Vec<String> {
    if ty == 406 {
        return format!("{};", values.join(",")).as_bytes().chunks(64)
            .map(|part| String::from_utf8_lossy(part).into_owned()).collect();
    }
    let mut rows = Vec::new();
    let mut row = String::new();
    for (i, value) in values.iter().enumerate() {
        if row.len() + value.len() + 1 > 64 {
            rows.push(std::mem::take(&mut row));
        }
        row.push_str(value);
        row.push(if i + 1 == values.len() { ';' } else { ',' });
    }
    rows.push(row);
    rows
}

fn render(mut w: Writer) -> Result<String> {
    let mut p_lines = Vec::new();
    let mut p_start = Vec::new();
    for (index, e) in w.entities.iter().enumerate() {
        let de = 2 * index + 1;
        p_start.push(p_lines.len() + 1);
        if e.values.iter().any(|value| !value.is_ascii()) {
            return Err(refuse("Non-ASCII IGES parameter"));
        }
        let rows = parameter_records(e.ty, &e.values);
        if rows.len() > MAX_ENTITY_PARAMETER_ROWS {
            return Err(refuse("IGES entity parameter span exceeds supported limit"));
        }
        for payload in rows {
            p_lines.push(
                pad_record(&payload, 'P', p_lines.len() + 1, 64)?[..64].to_string()
                    + &format!("{de:>8}P{:>7}", p_lines.len() + 1),
            );
        }
    }
    let mut lines = vec![pad_record(
        "OpenSCAD Viewer direct finite B-rep",
        'S',
        1,
        72,
    )?];
    let global = format!(
        "1H,,1H;,{} ,{} ,{} ,{} ,32,38,6,308,15,{},1.0,2,2HMM,1,0.001,{},0.001,{},{},{},11,0,{};",
        hollerith("OpenSCAD Viewer"),
        hollerith("open-scad-viewer"),
        hollerith("OpenSCAD Viewer"),
        hollerith("open-scad-viewer"),
        hollerith("IGES2.3"),
        hollerith("20260917.000000"),
        number(1e-7),
        hollerith(""),
        hollerith(""),
        hollerith("20260917.000000")
    );
    for (i, chunk) in global.as_bytes().chunks(72).enumerate() {
        lines.push(pad_record(
            std::str::from_utf8(chunk).unwrap(),
            'G',
            i + 1,
            72,
        )?);
    }
    p_start.push(p_lines.len() + 1);
    for (index, e) in w.entities.iter_mut().enumerate() {
        let de = 2 * index + 1;
        let a = format!(
            "{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}",
            e.ty, p_start[index], 0, 0, 0, 0, 0, 0, "00000000"
        );
        let b = format!(
            "{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:<8}{:>8}",
            e.ty, 0, e.color, p_start[index + 1] - p_start[index], e.form, 0, 0, e.label, 0
        );
        lines.push(pad_record(&a, 'D', de, 72)?);
        lines.push(pad_record(&b, 'D', de + 1, 72)?);
    }
    for p in &p_lines {
        lines.push(p.clone());
    }
    let terminate = format!(
        "S{:>7}G{:>7}D{:>7}P{:>7}",
        1,
        lines.iter().filter(|l| l.as_bytes()[72] == b'G').count(),
        2 * w.entities.len(),
        p_lines.len()
    );
    lines.push(pad_record(&terminate, 'T', 1, 72)?);
    if lines.len() > MAX_RECORDS {
        return Err(refuse("IGES output exceeds record limit"));
    }
    let text = lines.join("\n") + "\n";
    if text.len() > MAX_BYTES {
        return Err(refuse("IGES output exceeds 16 MiB"));
    }
    Ok(text)
}

pub fn export_iges_v2(model: &Model) -> Result<(String, FeatureCertificate, IgesV2Report)> {
    model.validate()?;
    if model.bodies.is_empty() || model.shells.iter().any(|s| !s.closed) {
        return Err(refuse("IGES /2 exports closed manifold bodies only"));
    }
    let mut w = Writer::new();
    let surfaces = model
        .faces
        .iter()
        .map(|face| Ok(w.emit(128, surface_values(&face.surface)?)))
        .collect::<Result<Vec<_>>>()?;
    let curves = model
        .edges
        .iter()
        .map(|edge| Ok(w.emit(126, curve_values(&edge.curve, 3)?)))
        .collect::<Result<Vec<_>>>()?;
    let mut vertex_values = vec!["502".into(), model.vertices.len().to_string()];
    for vertex in &model.vertices {
        vertex_values.extend(vertex.point.iter().map(|x| number(*x)))
    }
    let vertex_list = w.emit(502, vertex_values);
    let mut edge_values = vec!["504".into(), model.edges.len().to_string()];
    for (i, e) in model.edges.iter().enumerate() {
        edge_values.extend([
            curves[i].to_string(),
            vertex_list.to_string(),
            (e.vertices[0] + 1).to_string(),
            vertex_list.to_string(),
            (e.vertices[1] + 1).to_string(),
        ])
    }
    let edge_list = w.emit(504, edge_values);
    let mut loop_des = Vec::new();
    for l in &model.loops {
        let mut values = vec!["508".into(), l.coedges.len().to_string()];
        for c in &l.coedges {
            let pc = w.emit(126, curve_values(&c.pcurve, 2)?);
            values.extend([
                "0".into(),
                edge_list.to_string(),
                (c.edge + 1).to_string(),
                ((!c.reversed) as usize).to_string(),
                "1".into(),
                "0".into(),
                pc.to_string(),
            ]);
        }
        loop_des.push(w.emit(508, values));
    }
    let mut face_des = Vec::new();
    for (i, face) in model.faces.iter().enumerate() {
        let loops = std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .collect::<Vec<_>>();
        let mut values = vec![
            "510".into(),
            surfaces[i].to_string(),
            loops.len().to_string(),
            "1".into(),
        ];
        values.extend(loops.iter().map(|i| loop_des[*i].to_string()));
        face_des.push(w.emit(510, values));
    }
    let mut shell_des = Vec::new();
    for shell in &model.shells {
        let mut values = vec!["514".into(), shell.faces.len().to_string()];
        for f in &shell.faces {
            values.extend([
                face_des[f.face].to_string(),
                ((!f.reversed) as usize).to_string(),
            ])
        }
        shell_des.push(w.emit(514, values));
    }
    let mut body_des = Vec::new();
    for body in &model.bodies {
        let mut values = vec![
            "186".into(),
            shell_des[body.outer_shell].to_string(),
            "1".into(),
            body.inner_shells.len().to_string(),
        ];
        for s in &body.inner_shells {
            values.extend([shell_des[*s].to_string(), "0".into()])
        }
        body_des.push(w.emit(186, values));
    }
    let groups = [
        (
            TopoKind::Vertex,
            model.1.vertices.as_slice(),
            vec![vertex_list; model.vertices.len()],
        ),
        (
            TopoKind::Edge,
            model.1.edges.as_slice(),
            vec![edge_list; model.edges.len()],
        ),
        (TopoKind::Loop, model.1.loops.as_slice(), loop_des.clone()),
        (TopoKind::Face, model.1.faces.as_slice(), face_des.clone()),
        (
            TopoKind::Shell,
            model.1.shells.as_slice(),
            shell_des.clone(),
        ),
        (TopoKind::Body, model.1.bodies.as_slice(), body_des.clone()),
    ];
    for (kind, ids, targets) in groups {
        let mut ordinal = BTreeMap::<usize, usize>::new();
        for (&id, target) in ids.iter().zip(targets) {
            let occurrence = ordinal.entry(target).or_insert(0);
            let values = &w.entities[(target - 1) / 2].values;
            let payload = format!(
                "OSCAD_TOPO/IGES2|{}|{}|{}|{}",
                kind.as_str(),
                id,
                digest_values(values),
                *occurrence
            );
            *occurrence += 1;
            let de = w.emit(
                406,
                vec![
                    "406".into(),
                    "15".into(),
                    hollerith(&payload),
                    target.to_string(),
                ],
            );
            w.entities[(de - 1) / 2].form = 15;
        }
    }
    let text = render(w)?;
    let entity_count = parse(&text)?.0.len();
    let topology_count = model.vertices.len()
        + model.edges.len()
        + model.loops.len()
        + model.faces.len()
        + model.shells.len()
        + model.bodies.len();
    Ok((
        text,
        FeatureCertificate {
            capability: IGES_INTERCHANGE_V2_CAPABILITY,
            complete: true,
            notes: vec![
                "direct_model_export",
                "rational_126_128",
                "manifold_186_topology",
                "graph_bound_topoid_properties",
                "no_fallback",
            ],
        },
        IgesV2Report {
            identity: crate::step_interchange::StepIdentityReport {
                preserved: true,
                source: "internal-metadata",
                preserved_count: topology_count,
                created_count: 0,
                lost_count: 0,
            },
            ignored_metadata: vec![],
            entity_count,
            topology_count,
            units: IgesV2UnitReport::export_millimeters(),
            discarded: IgesV2DiscardReport::default(),
        },
    ))
}

#[cfg(test)]
#[path = "iges_interchange_v2/tests/mod.rs"]
mod tests;
