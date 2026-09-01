//! ONNX sidecar stamp (`docs/specs/phase4-in-wasm-az.md`).
//!
//! `boardRows` / `boardCols` are the **pad size** (5×5), not the live game.

use dab_core::{AZ_CHANNELS, AZ_FEATURES_VERSION, AZ_HUD_COLS, AZ_HUD_ROWS, AZ_PLANE, AZ_POLICY};
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub const SCHEMA: &str = "dab-az-model/1";

/// Compile-time stamp the sidecar must match. Tests use `"dev"` unless
/// `DAB_AZ_SOURCE_STAMP` is set (CI / `wasm-pack`).
pub fn compiled_source_stamp() -> &'static str {
    option_env!("DAB_AZ_SOURCE_STAMP").unwrap_or("dev")
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Sidecar {
    pub schema: String,
    pub name: String,
    #[serde(rename = "boardRows")]
    pub board_rows: u8,
    #[serde(rename = "boardCols")]
    pub board_cols: u8,
    pub channels: usize,
    pub plane: usize,
    #[serde(rename = "policyLength")]
    pub policy_length: usize,
    #[serde(rename = "featuresVersion")]
    pub features_version: u32,
    #[serde(rename = "sourceStamp")]
    pub source_stamp: String,
    #[serde(rename = "onnxSha256")]
    pub onnx_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StampError {
    Json,
    Schema,
    Name,
    PadSize,
    Shape,
    FeaturesVersion,
    SourceStamp,
    Sha256,
}

impl StampError {
    pub fn as_str(self) -> &'static str {
        match self {
            StampError::Json => "invalid sidecar JSON",
            StampError::Schema => "unknown sidecar schema",
            StampError::Name => "sidecar name missing or malformed",
            StampError::PadSize => "sidecar boardRows/boardCols must be the 5×5 pad",
            StampError::Shape => "sidecar channels/plane/policyLength mismatch",
            StampError::FeaturesVersion => "sidecar featuresVersion mismatch",
            StampError::SourceStamp => "sidecar sourceStamp mismatch",
            StampError::Sha256 => "sidecar onnxSha256 mismatch",
        }
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn name_ok(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-')
}

/// Validate sidecar fields against the frozen tensor contract and the ONNX bytes.
pub fn validate_sidecar(
    sidecar: &str,
    onnx: &[u8],
    expected_source_stamp: &str,
) -> Result<Sidecar, StampError> {
    let parsed: Sidecar = serde_json::from_str(sidecar).map_err(|_| StampError::Json)?;
    if parsed.schema != SCHEMA {
        return Err(StampError::Schema);
    }
    if !name_ok(&parsed.name) {
        return Err(StampError::Name);
    }
    if parsed.board_rows != AZ_HUD_ROWS || parsed.board_cols != AZ_HUD_COLS {
        return Err(StampError::PadSize);
    }
    if parsed.channels != AZ_CHANNELS
        || parsed.plane != AZ_PLANE
        || parsed.policy_length != AZ_POLICY
    {
        return Err(StampError::Shape);
    }
    if parsed.features_version != AZ_FEATURES_VERSION {
        return Err(StampError::FeaturesVersion);
    }
    if parsed.source_stamp != expected_source_stamp {
        return Err(StampError::SourceStamp);
    }
    let digest = sha256_hex(onnx);
    if parsed.onnx_sha256 != digest {
        return Err(StampError::Sha256);
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_json(sha: &str, stamp: &str) -> String {
        format!(
            r#"{{
              "schema": "dab-az-model/1",
              "name": "az-5x5-v1",
              "boardRows": 5,
              "boardCols": 5,
              "channels": 7,
              "plane": 11,
              "policyLength": 60,
              "valueRange": [-1, 1],
              "featuresVersion": 1,
              "sourceStamp": "{stamp}",
              "onnxSha256": "{sha}"
            }}"#
        )
    }

    #[test]
    fn accept_matching_sidecar() {
        let onnx = b"not-a-real-onnx";
        let sha = sha256_hex(onnx);
        let stamp = compiled_source_stamp();
        let parsed = validate_sidecar(&valid_json(&sha, stamp), onnx, stamp).unwrap();
        assert_eq!(parsed.name, "az-5x5-v1");
        assert_eq!(parsed.board_rows, 5);
    }

    #[test]
    fn reject_bad_json() {
        assert_eq!(validate_sidecar("{", b"x", "dev"), Err(StampError::Json));
    }

    #[test]
    fn reject_schema() {
        let onnx = b"x";
        let json = valid_json(&sha256_hex(onnx), "dev").replace("dab-az-model/1", "other/1");
        assert_eq!(
            validate_sidecar(&json, onnx, "dev"),
            Err(StampError::Schema)
        );
    }

    #[test]
    fn reject_name() {
        let onnx = b"x";
        let json = valid_json(&sha256_hex(onnx), "dev").replace("az-5x5-v1", "AZ NET");
        assert_eq!(validate_sidecar(&json, onnx, "dev"), Err(StampError::Name));
    }

    #[test]
    fn reject_pad_size() {
        let onnx = b"x";
        let json =
            valid_json(&sha256_hex(onnx), "dev").replace("\"boardRows\": 5", "\"boardRows\": 3");
        assert_eq!(
            validate_sidecar(&json, onnx, "dev"),
            Err(StampError::PadSize)
        );
    }

    #[test]
    fn reject_shape() {
        let onnx = b"x";
        let json = valid_json(&sha256_hex(onnx), "dev")
            .replace("\"policyLength\": 60", "\"policyLength\": 24");
        assert_eq!(validate_sidecar(&json, onnx, "dev"), Err(StampError::Shape));
    }

    #[test]
    fn reject_features_version() {
        let onnx = b"x";
        let json = valid_json(&sha256_hex(onnx), "dev")
            .replace("\"featuresVersion\": 1", "\"featuresVersion\": 2");
        assert_eq!(
            validate_sidecar(&json, onnx, "dev"),
            Err(StampError::FeaturesVersion)
        );
    }

    #[test]
    fn reject_source_stamp() {
        let onnx = b"x";
        let json = valid_json(&sha256_hex(onnx), "dev");
        assert_eq!(
            validate_sidecar(&json, onnx, "other"),
            Err(StampError::SourceStamp)
        );
    }

    #[test]
    fn reject_sha256() {
        let json = valid_json("deadbeef", "dev");
        assert_eq!(
            validate_sidecar(&json, b"x", "dev"),
            Err(StampError::Sha256)
        );
    }
}
