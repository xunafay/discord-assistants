use async_openai::types::{AssistantTools, RunToolCallObject, SubmitToolOutputsRunRequest};
use serenity::client::Context;

use crate::tools::image::ImageTool;

use self::{
    assistant_create::AssistantCreateTool, assistant_list::AssistantListTool,
    transcribe::TranscribeTool, tts::TtsTool,
};

pub mod assistant_create;
pub mod assistant_list;
pub mod image;
pub mod transcribe;
pub mod tts;

pub enum Tools {
    AssistantCreate,
    AssistantList,
    Image,
    Transcribe,
    Tts,
}

impl Tools {
    pub fn name(&self) -> String {
        match self {
            Tools::AssistantCreate => AssistantCreateTool::name(),
            Tools::AssistantList => AssistantListTool::name(),
            Tools::Image => ImageTool::name(),
            Tools::Transcribe => TranscribeTool::name(),
            Tools::Tts => TtsTool::name(),
        }
    }

    pub fn definition(&self) -> AssistantTools {
        match self {
            Tools::AssistantCreate => AssistantCreateTool::definition(),
            Tools::AssistantList => AssistantListTool::definition(),
            Tools::Image => ImageTool::definition(),
            Tools::Transcribe => TranscribeTool::definition(),
            Tools::Tts => TtsTool::definition(),
        }
    }
}

pub fn tools() -> Vec<Tools> {
    vec![
        Tools::AssistantCreate,
        Tools::AssistantList,
        Tools::Image,
        Tools::Transcribe,
        Tools::Tts,
    ]
}

pub trait AlvariumTool {
    type Arguments: Send + Sync;

    fn name() -> String;
    fn definition() -> AssistantTools;
    async fn run(
        args: Self::Arguments,
        context: &Context,
        tool: &RunToolCallObject,
    ) -> SubmitToolOutputsRunRequest;
}
