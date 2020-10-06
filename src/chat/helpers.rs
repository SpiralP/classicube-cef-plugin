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

pub fn is_global_cef_message(mut message: &str) -> Option<&str> {
    if message.len() >= 2 && message.get(0..1).map(|a| a == "&").unwrap_or(false) {
        message = message.get(2..)?;
    }

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
    assert_eq!(is_global_cef_message(""), None);
    assert_eq!(is_global_cef_message("&f"), None);
    assert_eq!(is_global_cef_message("&fcef"), None);

    assert_eq!(is_global_cef_message("&fceff is BAD"), None);
}

pub fn is_continuation_message(mut message: &str) -> Option<&str> {
    if message.starts_with("> ") {
        message = message.get(2..)?;

        // skip "&f" if it exists
        if message.len() >= 2 && message.get(0..1).map(|a| a == "&").unwrap_or(false) {
            Some(message.get(2..)?)
        } else {
            Some(message)
        }
    } else {
        None
    }
}

pub fn is_clients_start_message(message: &str) -> bool {
    message.len() >= 14
        && (message.get(0..1).map(|a| a == "&").unwrap_or(false)
            && message
                .get(2..)
                .map(|a| a == "Players using:")
                .unwrap_or(false))
}

pub fn is_clients_message(message: &str) -> Option<&str> {
    // &7  ClassiCube 1.1.6 + cef0.9.4 + Ponies v2.1: &f¿ Mew, ┌ Glim
    // > royalgazer, Princess, BOI, ╪ savage, Dino, ░ NotDerek,
    // &7  ClassiCube 1.1.6 + cef0.9.4 +cs3.4.5 + More Models v1.2.4 +
    // > &7Poni: &fSpiralP
    // > &7+ Pon: &fSpiralP
    // &7  ClassiCraft 1.1.3: &fFaeEmpress
    if message.len() >= 5
        && (message.get(0..1).map(|a| a == "&").unwrap_or(false)
            && message.get(2..4).map(|a| a == "  ").unwrap_or(false))
    {
        Some(message.get(4..)?)
    } else {
        None
    }
}
