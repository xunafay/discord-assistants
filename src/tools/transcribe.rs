use async_openai::types::{RunToolCallObject, SubmitToolOutputsRunRequest, ToolsOutputs};
use log::debug;

use crate::openai::{OpenAI, TranscribeToolArguments};

pub fn transcribe_tool(
    args: &String,
    openai: &OpenAI,
    tool: &RunToolCallObject,
) -> SubmitToolOutputsRunRequest {
    let args: TranscribeToolArguments =
        serde_json::from_str(args).expect("Failed to deserialize arguments");

    let transcript = openai.stt(&args.url).expect("Failed to transcribe");
    debug!("Transcript: {}", transcript);
    let output = SubmitToolOutputsRunRequest {
        tool_outputs: vec![ToolsOutputs {
            tool_call_id: Some(tool.id.clone()),
            output: Some(transcript),
        }],
    };
    output
}
