//! iges interchange v2 algorithm: parse_directory_status.
use super::*;
pub fn parse_directory_status(de: usize, text: &str) -> Result<DirectoryStatus> {
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
