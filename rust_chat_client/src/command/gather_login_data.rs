use crate::command::Command;
use crate::command::CommandType;

pub struct GatherLoginDataCommand {
}

impl Command for GatherLoginDataCommand {
    fn get_type(&self) -> CommandType {
        CommandType::GatherLoginData
    }

    fn to_json(&self) -> String {
        "{}".to_string()
    }
}
