pub fn is_outgoing_whisper(message: &str) -> bool {
    message.len() >= 6
        && (&message.as_bytes()[0..1] == b"&" && &message.as_bytes()[2..6] == b"[<] ")
}

pub fn is_incoming_whisper(message: &str) -> bool {
    message.len() >= 6
        && (&message.as_bytes()[0..1] == b"&" && &message.as_bytes()[2..6] == b"[>] ")
}
