use half::f16;
use serde::{Deserialize, Serialize};

use crate::{
    bitpack::{pack_indices, unpack_indices},
    codebook::FibCodebookV1,
    digest::{bytes_digest, json_digest},
    lloyd::nearest_index,
    metrics,
    profile::{FibQuantProfileV1, NormFormat},
    receipt::FibQuantCompressionReceiptV1,
    rotation::StoredRotation,
    FibQuantError, Result,
};

pub const CODE_SCHEMA: &str = "fib_code_v1";

/// Encoded fixed-rate FibQuant artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FibCodeV1 {
    /// Stable schema marker.
    pub schema_version: String,
    /// Profile digest.
    pub profile_digest: String,
    /// Codebook digest.
    pub codebook_digest: String,
    /// Rotation digest.
    pub rotation_digest: String,
    /// Ambient dimension.
    pub ambient_dim: u32,
    /// Block dimension.
    pub block_dim: u32,
    /// Norm payload format.
    pub norm_format: NormFormat,
    /// Norm bytes.
    pub norm_payload: Vec<u8>,
    /// Bits per fixed-rate index.
    pub wire_index_bits: u8,
    /// Number of indices.
    pub block_count: u32,
    /// Packed fixed-rate indices.
    pub indices: Vec<u8>,
}

/// FibQuant encoder/decoder bound to one profile and codebook.
#[derive(Debug, Clone)]
pub struct FibQuantizer {
    profile: FibQuantProfileV1,
    codebook: FibCodebookV1,
    rotation: StoredRotation,
}

impl FibQuantizer {
    /// Build a quantizer by constructing the profile codebook.
    pub fn new(profile: FibQuantProfileV1) -> Result<Self> {
        let codebook = FibCodebookV1::build(profile)?;
        Self::from_codebook(codebook)
    }

    /// Build a quantizer from a validated codebook.
    pub fn from_codebook(codebook: FibCodebookV1) -> Result<Self> {
        codebook.validate()?;
        let profile = codebook.profile.clone();
        let rotation = StoredRotation::new(profile.ambient_dim as usize, profile.rotation_seed)?;
        Ok(Self {
            profile,
            codebook,
            rotation,
        })
    }

    /// Access the profile.
    pub fn profile(&self) -> &FibQuantProfileV1 {
        &self.profile
    }

    /// Access the codebook.
    pub fn codebook(&self) -> &FibCodebookV1 {
        &self.codebook
    }

    /// Encode a vector into a fixed-rate artifact.
    pub fn encode(&self, x: &[f32]) -> Result<FibCodeV1> {
        let d = self.profile.ambient_dim as usize;
        let k = self.profile.block_dim as usize;
        if x.len() != d {
            return Err(FibQuantError::CorruptPayload(format!(
                "input dimension {}, expected {d}",
                x.len()
            )));
        }
        check_finite(x)?;
        let norm = l2_norm(x);
        if norm == 0.0 {
            return Err(FibQuantError::ZeroNorm);
        }
        let normalized: Vec<f64> = x.iter().map(|value| f64::from(*value) / norm).collect();
        let rotated = self.rotation.apply(&normalized)?;
        let codewords_f64: Vec<f64> = self
            .codebook
            .codewords
            .iter()
            .map(|value| f64::from(*value))
            .collect();
        let block_count = self.profile.block_count() as usize;
        let mut indices = Vec::with_capacity(block_count);
        for block in rotated.chunks_exact(k) {
            indices.push(nearest_index(block, &codewords_f64, k).0 as u32);
        }
        Ok(FibCodeV1 {
            schema_version: CODE_SCHEMA.into(),
            profile_digest: self.profile.digest()?,
            codebook_digest: self.codebook.codebook_digest.clone(),
            rotation_digest: self.rotation.digest()?,
            ambient_dim: self.profile.ambient_dim,
            block_dim: self.profile.block_dim,
            norm_format: self.profile.norm_format.clone(),
            norm_payload: encode_norm(norm, &self.profile.norm_format)?,
            wire_index_bits: self.profile.wire_index_bits,
            block_count: self.profile.block_count(),
            indices: pack_indices(&indices, self.profile.wire_index_bits)?,
        })
    }

    /// Decode a fixed-rate artifact.
    pub fn decode(&self, code: &FibCodeV1) -> Result<Vec<f32>> {
        self.validate_code_header(code)?;
        let k = self.profile.block_dim as usize;
        let block_count = self.profile.block_count() as usize;
        let unpacked = unpack_indices(&code.indices, block_count, self.profile.wire_index_bits)?;
        let mut rotated = Vec::with_capacity(self.profile.ambient_dim as usize);
        for index in unpacked {
            if index >= self.profile.codebook_size {
                return Err(FibQuantError::IndexOutOfRange {
                    index,
                    codebook_size: self.profile.codebook_size,
                });
            }
            rotated.extend(self.codebook.codeword(index as usize)?);
        }
        let expected_rotated_len = block_count.checked_mul(k).ok_or_else(|| {
            FibQuantError::ResourceLimitExceeded("decoded rotated vector length overflow".into())
        })?;
        if rotated.len() != expected_rotated_len {
            return Err(FibQuantError::CorruptPayload(
                "decoded rotated vector length mismatch".into(),
            ));
        }
        let norm = decode_norm(&code.norm_payload, &code.norm_format)?;
        let reconstructed = self.rotation.apply_inverse(&rotated)?;
        let out: Vec<f32> = reconstructed
            .into_iter()
            .map(|value| (value * norm) as f32)
            .collect();
        check_finite(&out)?;
        Ok(out)
    }

    /// Encode and emit a receipt.
    pub fn encode_with_receipt(
        &self,
        x: &[f32],
    ) -> Result<(FibCodeV1, FibQuantCompressionReceiptV1)> {
        let code = self.encode(x)?;
        let source_vector_digest = source_vector_digest(x)?;
        let mut receipt = FibQuantCompressionReceiptV1::new(
            &self.profile,
            code.profile_digest.clone(),
            code.codebook_digest.clone(),
            code.rotation_digest.clone(),
            source_vector_digest,
            encoded_digest(&code)?,
        );
        let decoded = self.decode(&code)?;
        receipt.mse = Some(metrics::mse(x, &decoded)?);
        receipt.cosine_similarity = Some(metrics::cosine_similarity(x, &decoded)?);
        Ok((code, receipt))
    }

    /// Reconstruction MSE for one vector.
    pub fn reconstruction_mse(&self, x: &[f32]) -> Result<f64> {
        let code = self.encode(x)?;
        let decoded = self.decode(&code)?;
        metrics::mse(x, &decoded)
    }

    /// Reconstruction cosine similarity for one vector.
    pub fn cosine_similarity(&self, x: &[f32]) -> Result<f64> {
        let code = self.encode(x)?;
        let decoded = self.decode(&code)?;
        metrics::cosine_similarity(x, &decoded)
    }

    fn validate_code_header(&self, code: &FibCodeV1) -> Result<()> {
        if code.schema_version != CODE_SCHEMA {
            return Err(FibQuantError::CorruptPayload(format!(
                "code schema_version {}, expected {CODE_SCHEMA}",
                code.schema_version
            )));
        }
        let expected_profile = self.profile.digest()?;
        if code.profile_digest != expected_profile {
            return Err(FibQuantError::ProfileDigestMismatch {
                expected: expected_profile,
                actual: code.profile_digest.clone(),
            });
        }
        if code.codebook_digest != self.codebook.codebook_digest {
            return Err(FibQuantError::CodebookDigestMismatch {
                expected: self.codebook.codebook_digest.clone(),
                actual: code.codebook_digest.clone(),
            });
        }
        let expected_rotation = self.rotation.digest()?;
        if code.rotation_digest != expected_rotation
            || code.rotation_digest != self.codebook.rotation_digest
        {
            return Err(FibQuantError::RotationDigestMismatch {
                expected: expected_rotation,
                actual: code.rotation_digest.clone(),
            });
        }
        if code.ambient_dim != self.profile.ambient_dim
            || code.block_dim != self.profile.block_dim
            || code.block_count != self.profile.block_count()
            || code.wire_index_bits != self.profile.wire_index_bits
            || code.norm_format != self.profile.norm_format
        {
            return Err(FibQuantError::CorruptPayload(
                "encoded header does not match profile".into(),
            ));
        }
        Ok(())
    }
}

/// Stable digest over the encoded artifact fields.
pub fn encoded_digest(code: &FibCodeV1) -> Result<String> {
    json_digest(CODE_SCHEMA, code)
}

fn source_vector_digest(x: &[f32]) -> Result<String> {
    check_finite(x)?;
    let mut bytes = Vec::with_capacity(32 + std::mem::size_of_val(x));
    bytes.extend_from_slice(b"fib_quant_source_vector_v1");
    bytes.push(0);
    bytes.extend_from_slice(&(x.len() as u64).to_le_bytes());
    for value in x {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    Ok(bytes_digest(&bytes))
}

fn encode_norm(norm: f64, format: &NormFormat) -> Result<Vec<u8>> {
    if !norm.is_finite() || norm <= 0.0 {
        return Err(FibQuantError::CorruptPayload(
            "norm must be finite and positive".into(),
        ));
    }
    match format {
        NormFormat::Fp16Paper => {
            let narrowed = f16::from_f32(norm as f32);
            if !narrowed.is_finite() || narrowed <= f16::ZERO {
                return Err(FibQuantError::CorruptPayload(
                    "norm cannot be represented as finite positive fp16".into(),
                ));
            }
            Ok(narrowed.to_le_bytes().to_vec())
        }
        NormFormat::F32Reference => {
            let narrowed = norm as f32;
            if !narrowed.is_finite() || narrowed <= 0.0 {
                return Err(FibQuantError::CorruptPayload(
                    "norm cannot be represented as finite positive f32".into(),
                ));
            }
            Ok(narrowed.to_le_bytes().to_vec())
        }
    }
}

fn decode_norm(bytes: &[u8], format: &NormFormat) -> Result<f64> {
    match format {
        NormFormat::Fp16Paper => {
            let bytes: [u8; 2] = bytes
                .try_into()
                .map_err(|_| FibQuantError::CorruptPayload("fp16 norm length".into()))?;
            let value = f16::from_le_bytes(bytes).to_f32() as f64;
            if value.is_finite() && value > 0.0 {
                Ok(value)
            } else {
                Err(FibQuantError::CorruptPayload("invalid fp16 norm".into()))
            }
        }
        NormFormat::F32Reference => {
            let bytes: [u8; 4] = bytes
                .try_into()
                .map_err(|_| FibQuantError::CorruptPayload("f32 norm length".into()))?;
            let value = f32::from_le_bytes(bytes) as f64;
            if value.is_finite() && value > 0.0 {
                Ok(value)
            } else {
                Err(FibQuantError::CorruptPayload("invalid f32 norm".into()))
            }
        }
    }
}

fn l2_norm(x: &[f32]) -> f64 {
    x.iter()
        .map(|value| {
            let value = f64::from(*value);
            value * value
        })
        .sum::<f64>()
        .sqrt()
}

fn check_finite(x: &[f32]) -> Result<()> {
    if let Some((idx, _)) = x.iter().enumerate().find(|(_, value)| !value.is_finite()) {
        return Err(FibQuantError::NonFiniteInput(idx));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_norm_overflow_rejects_before_payload_emit() {
        let err = encode_norm(f64::MAX, &NormFormat::F32Reference).unwrap_err();
        assert!(matches!(err, FibQuantError::CorruptPayload(message) if message.contains("f32")));
    }

    #[test]
    fn f32_norm_underflow_rejects_before_payload_emit() {
        let err = encode_norm(
            f64::from(f32::from_bits(1)) / 2.0,
            &NormFormat::F32Reference,
        )
        .unwrap_err();
        assert!(matches!(err, FibQuantError::CorruptPayload(message) if message.contains("f32")));
    }
}
