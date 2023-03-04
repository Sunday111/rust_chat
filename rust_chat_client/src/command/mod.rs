pub trait Command : std::any::Any {
    fn get_type(&self) -> CommandType;
    fn to_json(&self) -> String;
}

pub enum CommandType {
    GatherLoginData,
    Login,
}

pub mod login_command;
pub mod gather_login_data;