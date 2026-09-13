//! Bounded data-only encoding. Decoded records must pass source-checked replay.
use crate::*;
pub const MAX_RECIPE_GRAPH_BYTES: usize = 256 * 1024;
const MAGIC: &[u8] = b"BRG3\x01";
fn invalid() -> InputError {
    InputError::InvalidInput("Invalid 3D recipe encoding")
}
fn text(out: &mut Vec<u8>, value: &str) -> Result<(), InputError> {
    if value.len() > 4096 {
        return Err(invalid());
    }
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}
fn index(out: &mut Vec<u8>, value: usize) -> Result<(), InputError> {
    out.extend_from_slice(&u32::try_from(value).map_err(|_| invalid())?.to_le_bytes());
    Ok(())
}
/// BRG3 version 1; little-endian integers and raw IEEE-754 bits. No cached result.
pub fn encode_recipe_graph3(graph: &RecipeGraph3Record) -> Result<Vec<u8>, InputError> {
    if graph.schema_version != 1
        || graph.nodes.is_empty()
        || graph.nodes.len() > MAX_CONSTRUCTION_NODES
        || graph.root != graph.nodes.len() - 1
    {
        return Err(invalid());
    }
    let mut out = MAGIC.to_vec();
    text(&mut out, &graph.implementation)?;
    text(&mut out, &graph.source_id)?;
    out.extend_from_slice(&graph.source_revision.to_le_bytes());
    let s = &graph.tolerance;
    for x in [
        s.linear_abs,
        s.linear_rel,
        s.on_tol,
        s.clear_tol,
        s.angular,
        s.param_floor,
        s.max_entity_error,
    ] {
        out.extend_from_slice(&x.to_bits().to_le_bytes());
    }
    out.extend_from_slice(&s.ulp_guard.to_le_bytes());
    text(&mut out, &s.policy)?;
    index(&mut out, graph.nodes.len())?;
    index(&mut out, graph.root)?;
    for (n, node) in graph.nodes.iter().enumerate() {
        out.push(match node.kind {
            ConstructionKind3::LinePlane => 0,
            ConstructionKind3::CoplanarEndpoint(OverlapEndpoint3::Lower) => 1,
            ConstructionKind3::CoplanarEndpoint(OverlapEndpoint3::Upper) => 2,
        });
        for input in &node.inputs {
            match input {
                RecipeInput3Record::Node(i) => {
                    if *i >= n {
                        return Err(invalid());
                    }
                    out.push(0);
                    index(&mut out, *i)?;
                }
                RecipeInput3Record::Authored { indices, values } => {
                    out.push(1);
                    for i in indices {
                        index(&mut out, *i)?;
                    }
                    for v in values {
                        match v {
                            AuthoredScalar::Binary64Bits(bits) => {
                                out.push(0);
                                out.extend_from_slice(&bits.to_le_bytes());
                            }
                            AuthoredScalar::RationalConstant {
                                numerator,
                                denominator,
                            } => {
                                out.push(1);
                                out.extend_from_slice(&numerator.to_le_bytes());
                                out.extend_from_slice(&denominator.to_le_bytes());
                            }
                        }
                    }
                }
            }
        }
    }
    if out.len() > MAX_RECIPE_GRAPH_BYTES {
        return Err(invalid());
    }
    Ok(out)
}
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl Reader<'_> {
    fn take(&mut self, count: usize) -> Result<&[u8], InputError> {
        let end = self.offset.checked_add(count).ok_or_else(invalid)?;
        let result = self.bytes.get(self.offset..end).ok_or_else(invalid)?;
        self.offset = end;
        Ok(result)
    }
    fn byte(&mut self) -> Result<u8, InputError> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, InputError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| invalid())?,
        ))
    }
    fn u64(&mut self) -> Result<u64, InputError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().map_err(|_| invalid())?,
        ))
    }
    fn text(&mut self) -> Result<String, InputError> {
        let len = self.u32()? as usize;
        if len > 4096 {
            return Err(invalid());
        }
        String::from_utf8(self.take(len)?.to_vec()).map_err(|_| invalid())
    }
}
/// Decode data records only. Call replay_point3d with an authoritative admitted
/// source before using any decoded record as a constructed geometric point.
pub fn decode_recipe_graph3(bytes: &[u8]) -> Result<RecipeGraph3Record, InputError> {
    if bytes.len() > MAX_RECIPE_GRAPH_BYTES {
        return Err(invalid());
    }
    let mut r = Reader { bytes, offset: 0 };
    if r.take(MAGIC.len())? != MAGIC {
        return Err(invalid());
    }
    let implementation = r.text()?;
    let source_id = r.text()?;
    let source_revision = r.u64()?;
    let mut fields = [0.; 7];
    for f in &mut fields {
        *f = f64::from_bits(r.u64()?);
    }
    let ulp_guard = r.u32()?;
    let policy = r.text()?;
    let tolerance = ToleranceSpec {
        linear_abs: fields[0],
        linear_rel: fields[1],
        on_tol: fields[2],
        clear_tol: fields[3],
        angular: fields[4],
        param_floor: fields[5],
        max_entity_error: fields[6],
        ulp_guard,
        policy,
    };
    let count = r.u32()? as usize;
    let root = r.u32()? as usize;
    if count == 0 || count > MAX_CONSTRUCTION_NODES || root != count - 1 {
        return Err(invalid());
    }
    let mut nodes = Vec::with_capacity(count);
    for n in 0..count {
        let kind = match r.byte()? {
            0 => ConstructionKind3::LinePlane,
            1 => ConstructionKind3::CoplanarEndpoint(OverlapEndpoint3::Lower),
            2 => ConstructionKind3::CoplanarEndpoint(OverlapEndpoint3::Upper),
            _ => return Err(invalid()),
        };
        let mut inputs = Vec::with_capacity(5);
        for _ in 0..5 {
            inputs.push(match r.byte()? {
                0 => {
                    let i = r.u32()? as usize;
                    if i >= n {
                        return Err(invalid());
                    }
                    RecipeInput3Record::Node(i)
                }
                1 => {
                    let indices = [r.u32()? as usize, r.u32()? as usize, r.u32()? as usize];
                    let mut values = Vec::with_capacity(3);
                    for _ in 0..3 {
                        values.push(match r.byte()? {
                            0 => AuthoredScalar::Binary64Bits(r.u64()?),
                            1 => AuthoredScalar::RationalConstant {
                                numerator: r.u64()? as i64,
                                denominator: r.u64()?,
                            },
                            _ => return Err(invalid()),
                        });
                    }
                    RecipeInput3Record::Authored {
                        indices,
                        values: values.try_into().map_err(|_| invalid())?,
                    }
                }
                _ => return Err(invalid()),
            });
        }
        nodes.push(RecipeNode3Record {
            kind,
            inputs: inputs.try_into().map_err(|_| invalid())?,
        });
    }
    if r.offset != bytes.len() {
        return Err(invalid());
    }
    Ok(RecipeGraph3Record {
        schema_version: 1,
        implementation,
        source_id,
        source_revision,
        tolerance,
        nodes,
        root,
    })
}

/// Bounds the number of independently encoded vertex roots retained by one load.
pub const MAX_RECIPE_BATCH_POINTS: usize = 256;
pub const MAX_RECIPE_BATCH_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug)]
pub struct Point3BatchReplayReport {
    /// No partial points escape if any record fails admission or replay.
    pub outcome: Result<Vec<ConstructedPoint3>, Reason>,
    pub work_used: u64,
    pub context: ContextIdentity,
}

/// Decode and replay an ordered set of vertex records under one cumulative
/// context budget. The caller must admit the authoritative source beforehand
/// and validate topology after success. This does not certify a solid or merge
/// equal vertices. Byte decoding is charged before each bounded record; failure
/// consumes work already performed but publishes no partial batch.
pub fn replay_point3d_batch(
    ctx: &mut PredicateContext<'_>,
    records: &[&[u8]],
) -> Result<Point3BatchReplayReport, InputError> {
    let total = (records.len() <= MAX_RECIPE_BATCH_POINTS)
        .then(|| {
            records.iter().try_fold(0usize, |sum, bytes| {
                if bytes.len() > MAX_RECIPE_GRAPH_BYTES {
                    return None;
                }
                sum.checked_add(bytes.len())
            })
        })
        .flatten();
    let outcome = if records.len() > MAX_RECIPE_BATCH_POINTS
        || total.is_none_or(|n| n > MAX_RECIPE_BATCH_BYTES)
    {
        Err(Reason::ResourceLimit)
    } else {
        let mut points = Vec::with_capacity(records.len());
        let mut result = ctx.charge(0);
        for bytes in records {
            if result.is_err() {
                break;
            }
            result = ctx.charge(bytes.len() as u64);
            if result.is_err() {
                break;
            }
            let graph = decode_recipe_graph3(bytes)?;
            match replay_point3d(ctx, &graph)?.outcome {
                Ok(point) => points.push(point),
                Err(reason) => result = Err(reason),
            }
        }
        result.and_then(|()| ctx.charge(0)).map(|()| points)
    };
    Ok(Point3BatchReplayReport {
        outcome,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

#[derive(Debug)]
pub struct Point3BatchExportReport {
    /// Ordered binary recipes; no partial archive on failure.
    pub outcome: Result<Vec<Vec<u8>>, Reason>,
    pub work_used: u64,
    pub context: ContextIdentity,
}

/// Export vertex roots under one cumulative context budget and the same count
/// and encoded-size limits as batch replay. Coordinates are never rounded.
/// Source ownership is checked by each recipe export. Failure retains charged
/// work but returns no partially serialized vertex set.
pub fn export_point3d_batch(
    ctx: &mut PredicateContext<'_>,
    points: &[&ConstructedPoint3],
) -> Result<Point3BatchExportReport, InputError> {
    let outcome = if points.len() > MAX_RECIPE_BATCH_POINTS {
        Err(Reason::ResourceLimit)
    } else {
        let mut records = Vec::with_capacity(points.len());
        let mut total = 0usize;
        let mut result = ctx.charge(0);
        for point in points {
            if result.is_err() {
                break;
            }
            match export_point3d(ctx, point)?.outcome {
                Ok(graph) => {
                    let bytes = encode_recipe_graph3(&graph)?;
                    total += bytes.len();
                    if total > MAX_RECIPE_BATCH_BYTES {
                        result = Err(Reason::ResourceLimit);
                        break;
                    }
                    result = ctx.charge(bytes.len() as u64);
                    if result.is_ok() {
                        records.push(bytes);
                    }
                }
                Err(reason) => result = Err(reason),
            }
        }
        result.and_then(|()| ctx.charge(0)).map(|()| records)
    };
    Ok(Point3BatchExportReport {
        outcome,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
