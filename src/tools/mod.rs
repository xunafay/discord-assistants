use async_openai::types::{AssistantTools, RunToolCallObject, SubmitToolOutputsRunRequest};
use serenity::client::Context;

use crate::tools::image::ImageTool;

use self::{
    assistant_create::AssistantCreateTool, assistant_list::AssistantListTool,
    transcribe::TranscribeTool, tts::TtsTool, datetime::DateTimeTool,
};

pub mod assistant_create;
pub mod assistant_list;
pub mod image;
pub mod transcribe;
pub mod tts;
pub mod datetime;

pub enum Tools {
    AssistantCreate,
    AssistantList,
    Image,
    Transcribe,
    Tts,
    DateTime
}

impl Tools {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "assistant_create" => Some(Tools::AssistantCreate),
            "assistant_list" => Some(Tools::AssistantList),
            "image" => Some(Tools::Image),
            "transcribe" => Some(Tools::Transcribe),
            "tts" => Some(Tools::Tts),
            "datetime" => Some(Tools::DateTime),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        match self {
            Tools::AssistantCreate => AssistantCreateTool::name(),
            Tools::AssistantList => AssistantListTool::name(),
            Tools::Image => ImageTool::name(),
            Tools::Transcribe => TranscribeTool::name(),
            Tools::Tts => TtsTool::name(),
            Tools::DateTime => DateTimeTool::name(),
        }
    }

    pub fn definition(&self) -> AssistantTools {
        match self {
            Tools::AssistantCreate => AssistantCreateTool::definition(),
            Tools::AssistantList => AssistantListTool::definition(),
            Tools::Image => ImageTool::definition(),
            Tools::Transcribe => TranscribeTool::definition(),
            Tools::Tts => TtsTool::definition(),
            Tools::DateTime => DateTimeTool::definition(),
        }
    }

    pub fn description(&self) -> String {
        match self {
            Tools::AssistantCreate => AssistantCreateTool::description(),
            Tools::AssistantList => AssistantListTool::description(),
            Tools::Image => ImageTool::description(),
            Tools::Transcribe => TranscribeTool::description(),
            Tools::Tts => TtsTool::description(),
            Tools::DateTime => DateTimeTool::description(),
        }
    
    }
}

pub fn available_tools() -> Vec<Tools> {
    vec![
        Tools::AssistantCreate,
        Tools::AssistantList,
        Tools::Image,
        Tools::Transcribe,
        Tools::Tts,
        Tools::DateTime
    ]
}

pub trait AlvariumTool {
    type Arguments: Send + Sync;

    fn name() -> String;
    fn description() -> String;
    fn definition() -> AssistantTools;
    async fn run(
        args: Self::Arguments,
        context: &Context,
        tool: &RunToolCallObject,
    ) -> SubmitToolOutputsRunRequest;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tools() {
        assert_eq!(available_tools().len(), std::mem::variant_count::<Tools>());
    }
}