use crate::command::Command;
use crate::command::CommandType;

pub struct LoginCommand {
    username: String,
}

impl Command for LoginCommand {
    fn get_type(&self) -> CommandType {
        CommandType::Login
    }

    fn to_json(&self) -> String {
        format!("{{ \"username\": \"{}\" }}", self.username)
    }
}
