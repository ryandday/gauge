mod claude;
mod client;
mod types;

pub use claude::ClaudeClient;
pub use client::AiClient;
#[allow(unused_imports)] // Used in tests
pub use client::MockAiClient;
pub use types::{AssessmentResult, ChunkingResult};
