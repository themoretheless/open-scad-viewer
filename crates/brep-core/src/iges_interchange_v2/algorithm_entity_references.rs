//! iges interchange v2 algorithm: entity_references.
use super::*;
pub fn entity_references(de: usize, entity: &Entity) -> Result<Vec<EntityReference>> {
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
        100 | 502 | 110 | 124 | 126 | 128 | 141 | 142 | 143 | 314 => {}
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
    Ok(refs)
}
