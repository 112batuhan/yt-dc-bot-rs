pub mod commands;
pub mod events;

pub fn msg_send_error_log(err: &serenity::Error) {
    tracing::error!("Failed to send a message: {err}")
}
