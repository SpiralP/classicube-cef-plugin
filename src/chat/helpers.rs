use classicube_helpers::tab_list::remove_color;

pub fn is_outgoing_whisper(message: &str) -> bool {
    message.len() >= 6
        && (message.get(0..1).map(|a| a == "&").unwrap_or(false)
            && message.get(2..6).map(|a| a == "[<] ").unwrap_or(false))
}

pub fn is_incoming_whisper(message: &str) -> bool {
    message.len() >= 6
        && (message.get(0..1).map(|a| a == "&").unwrap_or(false)
            && message.get(2..6).map(|a| a == "[>] ").unwrap_or(false))
}

pub fn is_map_theme_message(message: &str) -> bool {
    let message = remove_color(message).to_lowercase();

    message.starts_with("map theme: ") || message.starts_with("map theme song: ")
}

pub fn is_global_cef_message(mut message: &str) -> Option<String> {
    if message.len() >= 2 && message.get(0..1).map(|a| a == "&").unwrap_or(false) {
        message = &message[2..];
    }

    if message.starts_with("cef ") {
        Some(message[4..].to_string())
    } else {
        None
    }
}

#[test]
fn test_is_global_cef_message() {
    assert_eq!(
        is_global_cef_message("&fcef is good"),
        Some("is good".to_string())
    );
    assert_eq!(
        is_global_cef_message("cef is good"),
        Some("is good".to_string())
    );
    assert_eq!(is_global_cef_message("cef "), Some("".to_string()));
    assert_eq!(is_global_cef_message(""), None);
    assert_eq!(is_global_cef_message("&f"), None);
    assert_eq!(is_global_cef_message("&fcef"), None);

    assert_eq!(is_global_cef_message("&fceff is BAD"), None);
}
