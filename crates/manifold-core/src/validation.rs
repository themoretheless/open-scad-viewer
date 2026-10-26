use std::fmt;

/// Structural input errors rejected before topology or metric analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexedMeshError {
    PositionsLength { len: usize },
    IndicesLength { len: usize },
    NonFiniteCoordinate { index: usize },
    IndexOutOfRange { index: usize, vertex_count: usize },
}

impl fmt::Display for IndexedMeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PositionsLength { len } => {
                write!(f, "Position buffer length {len} is not divisible by 3")
            }
            Self::IndicesLength { len } => {
                write!(f, "Index buffer length {len} is not divisible by 3")
            }
            Self::NonFiniteCoordinate { index } => {
                write!(f, "Position coordinate {index} is not finite")
            }
            Self::IndexOutOfRange {
                index,
                vertex_count,
            } => {
                write!(
                    f,
                    "Vertex index {index} exceeds vertex count {vertex_count}"
                )
            }
        }
    }
}

impl std::error::Error for IndexedMeshError {}

/// Validate the complete indexed-triangle transport before any topology pass.
pub fn validate_indexed_mesh(positions: &[f64], indices: &[usize]) -> Result<(), IndexedMeshError> {
    if positions.len() % 3 != 0 {
        return Err(IndexedMeshError::PositionsLength {
            len: positions.len(),
        });
    }
    if indices.len() % 3 != 0 {
        return Err(IndexedMeshError::IndicesLength { len: indices.len() });
    }
    if let Some(index) = positions.iter().position(|value| !value.is_finite()) {
        return Err(IndexedMeshError::NonFiniteCoordinate { index });
    }
    let vertex_count = positions.len() / 3;
    if let Some(&index) = indices.iter().find(|&&index| index >= vertex_count) {
        return Err(IndexedMeshError::IndexOutOfRange {
            index,
            vertex_count,
        });
    }
    Ok(())
}
