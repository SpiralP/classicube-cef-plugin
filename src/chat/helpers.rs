pub fn is_outgoing_whisper(message: &str) -> bool {
    message.len() >= 6
        && message.get(0..1).map(|a| a == "&").unwrap_or(false)
        && message.get(2..6).map(|a| a == "[<] ").unwrap_or(false)
}

pub fn is_incoming_whisper(message: &str) -> bool {
    message.len() >= 6
        && message.get(0..1).map(|a| a == "&").unwrap_or(false)
        && message.get(2..6).map(|a| a == "[>] ").unwrap_or(false)
}

pub fn is_cef_request_whisper(message: &str) -> bool {
    (is_outgoing_whisper(message) || is_incoming_whisper(message)) && message.contains("?CEF?")
}

pub fn is_cef_reply_whisper(message: &str) -> bool {
    (is_outgoing_whisper(message) || is_incoming_whisper(message)) && message.contains("!CEF!")
}

pub fn is_map_theme_message(message: &str) -> Option<&str> {
    let message = remove_color_left(message);

    if message.to_ascii_lowercase().starts_with("map theme:") {
        Some(remove_color_left(message["map theme:".len()..].trim()).trim())
    } else if message.to_ascii_lowercase().starts_with("map theme song:") {
        Some(remove_color_left(message["map theme song:".len()..].trim()).trim())
    } else {
        None
    }
}

pub fn is_global_cef_message(message: &str) -> Option<&str> {
    let message = remove_color_left(message);

    if message.starts_with("cef ") {
        message.get(4..)
    } else {
        None
    }
}

#[test]
fn test_is_global_cef_message() {
    assert_eq!(is_global_cef_message("&fcef is good"), Some("is good"));
    assert_eq!(is_global_cef_message("cef is good"), Some("is good"));
    assert_eq!(is_global_cef_message("cef "), Some(""));
    assert_eq!(is_global_cef_message("&fcef "), Some(""));
    assert_eq!(is_global_cef_message("cef"), None);
    assert_eq!(is_global_cef_message(""), None);
    assert_eq!(is_global_cef_message("&f"), None);
    assert_eq!(is_global_cef_message("&fcef"), None);

    assert_eq!(is_global_cef_message("&fceff is BAD"), None);
}

pub fn is_continuation_message(mut message: &str) -> Option<&str> {
    if message.starts_with("> ") {
        message = message.get(2..)?;
        Some(remove_color_left(message))
    } else {
        None
    }
}

pub fn is_clients_start_message(message: &str) -> bool {
    message.len() >= 14
        && message.get(0..1).map(|a| a == "&").unwrap_or(false)
        && message
            .get(2..)
            .map(|a| a == "Players using:")
            .unwrap_or(false)
}

pub fn is_clients_message(message: &str) -> Option<&str> {
    // &7  ClassiCube 1.1.6 + cef0.9.4 + Ponies v2.1: &f¿ Mew, ┌ Glim
    // > royalgazer, Princess, BOI, ╪ savage, Dino, ░ NotDerek,
    // &7  ClassiCube 1.1.6 + cef0.9.4 +cs3.4.5 + More Models v1.2.4 +
    // > &7Poni: &fSpiralP
    // > &7+ Pon: &fSpiralP
    // &7  ClassiCraft 1.1.3: &fFaeEmpress
    if message.len() >= 5
        && message.get(0..1).map(|a| a == "&").unwrap_or(false)
        && message.get(2..4).map(|a| a == "  ").unwrap_or(false)
    {
        Some(message.get(4..)?)
    } else {
        None
    }
}

pub fn remove_color_left(mut text: &str) -> &str {
    while text.len() >= 2 && text.get(0..1).map(|c| c == "&").unwrap_or(false) {
        text = &text[2..];
    }

    text
}
