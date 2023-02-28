use std::fmt::Debug;

#[derive(Debug)]
pub struct ChatError(pub String);

pub type ChatResult<T> = std::result::Result<T, ChatError>;

pub trait ConvertibleToChatResult<T> {
    fn to_chat_result(self) -> ChatResult<T>;
}

impl<T, E> ConvertibleToChatResult<T> for std::result::Result<T, E>
    where E: Debug
{
    fn to_chat_result(self) -> ChatResult<T> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(ChatError(format!("{:#?}", error)))
        }
    }
}