// @task(P1-T4a) Define AiClient trait with chunk_diff() and assess_hypothesis() method signatures
// @task(P1-T4b) Define ChunkingResult and AssessmentResult types with Success/Error variants
mod claude;
mod client;
mod types;

#[allow(unused_imports)] // Used in PHASE-4 integration
pub use claude::ClaudeClient;
pub use client::{AiClient, MockAiClient};
#[allow(unused_imports)] // Used in PHASE-4 integration
pub use types::{AssessmentResult, ChunkingResult};
