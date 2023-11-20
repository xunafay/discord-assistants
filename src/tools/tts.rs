use async_openai::types::{RunToolCallObject, SubmitToolOutputsRunRequest, Voice, SpeechModel, ToolsOutputs};
use log::debug;

use crate::openai::{OpenAI, TtsToolArguments};

pub async fn tts_tool(
    args: &String,
    openai: &OpenAI,
    tool: &RunToolCallObject,
) -> SubmitToolOutputsRunRequest {
    let args: TtsToolArguments =
        serde_json::from_str(args).expect("Failed to deserialize arguments");

    let voice = match args.voice {
        Some(text) => match text.as_str() {
            "alloy" => Voice::Alloy,
            "echo" => Voice::Echo,
            "fable" => Voice::Fable,
            "nova" => Voice::Nova,
            "onyx" => Voice::Onyx,
            "shimmer" => Voice::Shimmer,
            _ => Voice::Nova,
        },
        None => Voice::Nova,
    };

    let quality = SpeechModel::Tts1;

    debug!("calling tts with options: content: {:?}, voice: {:?}, quality: {:?}", args.content, voice, quality);
    let result = openai
        .tts(&args.content, voice, quality)
        .await
        .expect("Failed to generate voice");

    let file_name = rand::random::<u64>().to_string();
    let file_location = format!("./voice/{}.mp3", file_name);

    result
        .save(&file_location)
        .await
        .expect("Failed to save voice");

    let output = SubmitToolOutputsRunRequest {
        tool_outputs: vec![ToolsOutputs {
            tool_call_id: Some(tool.id.clone()),
            output: Some(format!("file://{}/", file_location)),
        }],
    };
    output
}
