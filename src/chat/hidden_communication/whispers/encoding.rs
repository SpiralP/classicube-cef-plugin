use crate::{error::*, players::Player};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    Player(Player),
}

/// to base64
pub fn encode(message: &Message) -> Result<String> {
    let data = bincode::serialize(message)?;

    Ok(base64::encode(data))
}

/// from base64
pub fn decode<T: AsRef<[u8]>>(input: T) -> Result<Message> {
    let data = base64::decode(input)?;

    Ok(bincode::deserialize(&data)?)
}

#[test]
fn test_encode_decode() {
    use crate::players::*;

    let player =
        Player::Web(WebPlayer::from_url("https://www.google.com/".parse().unwrap()).unwrap());
    let message = Message::Player(player);

    let base64 = encode(&message).unwrap();

    println!("{:#?}", base64);

    let message = decode(base64).unwrap();
    println!("{:#?}", message);
}
