use async_openai::types::{AssistantTools, RunToolCallObject, SubmitToolOutputsRunRequest};

use crate::openai::OpenAI;

pub mod image;
pub mod transcribe;
pub mod tts;

pub trait AlvariumTool {
    fn name(&self) -> String;
    fn definition() -> AssistantTools;
    async fn run(
        &mut self,
        args: String,
        openai: &OpenAI,
        tool: &RunToolCallObject,
    ) -> SubmitToolOutputsRunRequest;
}