use base64::engine::general_purpose::STANDARD as Base64Engine;
use base64::Engine;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Keypair, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    pub plan_id: String,
    pub step_id: String,
    pub ts: DateTime<Utc>,
    pub event_type: String,
    pub cost_usd: Option<f64>,
    pub tokens_in: Option<u64>,
    pub tokens_out: Option<u64>,
    pub citations: Option<Vec<String>>,
    pub signature: Option<String>,
    pub data: Option<serde_json::Value>,
}

impl Trace {
    pub fn new(event_type: String, step_id: String, description: String) -> Self {
        Self {
            plan_id: Uuid::new_v4().to_string(),
            step_id,
            ts: Utc::now(),
            event_type,
            cost_usd: None,
            tokens_in: None,
            tokens_out: None,
            citations: None,
            signature: None,
            data: Some(serde_json::json!({ "description": description })),
        }
    }

    pub fn with_plan_id(
        plan_id: String,
        event_type: String,
        step_id: String,
        description: String,
    ) -> Self {
        Self {
            plan_id,
            step_id,
            ts: Utc::now(),
            event_type,
            cost_usd: None,
            tokens_in: None,
            tokens_out: None,
            citations: None,
            signature: None,
            data: Some(serde_json::json!({ "description": description })),
        }
    }

    pub fn sign(&mut self, keypair: &Keypair) -> Result<(), TraceError> {
        let message = format!(
            "{}:{}:{}:{}",
            self.plan_id, self.step_id, self.ts, self.event_type
        );
        let signature: Signature = keypair.sign(message.as_bytes());
        self.signature = Some(Base64Engine.encode(signature.to_bytes()));
        Ok(())
    }

    pub fn verify_signature(
        &self,
        public_key: &ed25519_dalek::PublicKey,
    ) -> Result<bool, TraceError> {
        if let Some(ref sig_str) = self.signature {
            let signature_bytes = Base64Engine
                .decode(sig_str)
                .map_err(|e| TraceError::SignatureError(e.to_string()))?;

            if signature_bytes.len() != 64 {
                return Err(TraceError::SignatureError(
                    "Invalid signature length".to_string(),
                ));
            }

            let mut sig_bytes = [0u8; 64];
            sig_bytes.copy_from_slice(&signature_bytes[..64]);
            let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes)
                .map_err(|e| TraceError::SignatureError(e.to_string()))?;

            let message = format!(
                "{}:{}:{}:{}",
                self.plan_id, self.step_id, self.ts, self.event_type
            );

            public_key
                .verify(message.as_bytes(), &signature)
                .map_err(|e| TraceError::SignatureError(e.to_string()))
                .map(|_| true)
        } else {
            Err(TraceError::SignatureError(
                "No signature present".to_string(),
            ))
        }
    }
}

pub struct TraceSigner {
    keypair: Keypair,
    public_key: ed25519_dalek::PublicKey,
}

impl TraceSigner {
    pub fn new() -> Result<Self, TraceError> {
        let mut rng = OsRng::default();
        let keypair = Keypair::generate(&mut rng);
        let public_key = keypair.public;

        Ok(Self {
            keypair,
            public_key,
        })
    }

    pub fn sign_trace(&self, trace: &mut Trace) -> Result<(), TraceError> {
        trace.sign(&self.keypair)
    }

    pub fn get_public_key(&self) -> &ed25519_dalek::PublicKey {
        &self.public_key
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TraceError {
    #[error("Signature error: {0}")]
    SignatureError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_signature() {
        let signer = TraceSigner::new().unwrap();
        let mut trace = Trace::new(
            "test_event".to_string(),
            "step_1".to_string(),
            "test trace".to_string(),
        );

        // Sign the trace
        signer.sign_trace(&mut trace).unwrap();

        // Verify the signature
        assert!(trace.verify_signature(signer.get_public_key()).unwrap());
    }
}
