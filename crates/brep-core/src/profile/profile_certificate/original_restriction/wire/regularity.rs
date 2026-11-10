//! Piecewise regular source curves with exact oriented internal-knot G1.
//! Authored corners BETWEEN source pieces are allowed.
use super::*;
#[derive(Clone, Debug)]
pub struct SourceRegularity {
    wire: Wire,
    spans: usize,
    internal_joins: Vec<smooth::Junction>,
}
fn derive(wire: &Wire) -> Result<(usize, Vec<smooth::Junction>)> {
    wire.reverify()?;
    let mut spans = 0;
    let mut joins = Vec::new();
    for (piece, net) in wire.pieces.iter().enumerate() {
        for span in net {
            smooth::regular(&span.net)?;
            spans += 1;
        }
        for i in 1..net.len() {
            joins.push(smooth::join(wire, [piece, i - 1], [piece, i], &Q::zero())?);
        }
    }
    Ok((spans, joins))
}
impl Wire {
    pub fn qualify_source_regularity(&self) -> Result<SourceRegularity> {
        let (spans, internal_joins) = derive(self)?;
        Ok(SourceRegularity {
            wire: self.clone(),
            spans,
            internal_joins,
        })
    }
}
impl SourceRegularity {
    pub fn wire(&self) -> &Wire {
        &self.wire
    }
    pub fn span_count(&self) -> usize {
        self.spans
    }
    pub fn internal_joins(&self) -> &[smooth::Junction] {
        &self.internal_joins
    }
    pub fn reverify(&self) -> Result<()> {
        let (spans, joins) = derive(&self.wire)?;
        if spans != self.spans || joins != self.internal_joins {
            return Err(refused("Native source regularity replay changed"));
        }
        Ok(())
    }
}
