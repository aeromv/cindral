// SPDX-License-Identifier: MIT OR Apache-2.0

//! `Chat Message` (0x02, clientbound, play): chat and system messages.
//!
//! Layout (`wiki.vg`, protocol 47): chat JSON String (32767-char cap, `readChatComponent`)
//! followed by a raw u8 position byte (1.8 reads it as an i8 filter: 0 chat,
//! 1 system, 2 action bar).

use std::io::{Read, Write};

use crate::error::ProtocolError;
use crate::packets::codec::read_u8;
use crate::packets::EncodablePacket;
use crate::types::{read_string, write_string, MAX_JSON_CHARS};

/// `Chat Message` (0x02, clientbound, play).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chat {
    /// Raw chat JSON (`{"text": ...}` or component tree).
    pub json: String,
    /// System-bit field raw u8 (1.8 uses an i8 message filter).
    pub position: u8,
}

impl Chat {
    /// Wire packet id.
    pub const ID: u8 = 0x02;

    /// Reads the body.
    ///
    /// # Errors
    /// [`ProtocolError`] on truncation or invalid UTF-8.
    pub fn decode(reader: &mut impl Read) -> Result<Self, ProtocolError> {
        Ok(Self {
            json: read_string(reader, MAX_JSON_CHARS)?,
            position: read_u8(reader)?,
        })
    }
}

impl EncodablePacket for Chat {
    fn packet_id(&self) -> u8 {
        Self::ID
    }

    fn encode_body(&self, writer: &mut impl Write) -> Result<(), ProtocolError> {
        write_string(writer, &self.json, MAX_JSON_CHARS)?;
        writer.write_all(&[self.position])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_round_trip() {
        let packet = Chat {
            json: "{\"text\":\"hi\"}".to_string(),
            position: 1,
        };
        let mut buffer = Vec::new();
        packet.encode_body(&mut buffer).unwrap();
        assert_eq!(Chat::decode(&mut &buffer[..]).unwrap(), packet);
    }

    /// A server's broadcast of 1 929 bytes (a coloured `tellraw` with click
    /// and hover events) ended the session under the old 256-char cap.
    #[test]
    fn a_long_chat_component_is_read() {
        let text = "{\"text\":\"x\",\"color\":\"aqua\"},".repeat(70);
        let packet = Chat {
            json: format!("[{}{{\"text\":\"\"}}]", text),
            position: 0,
        };
        assert!(packet.json.len() > 1900);
        let mut buffer = Vec::new();
        packet.encode_body(&mut buffer).unwrap();
        assert_eq!(Chat::decode(&mut &buffer[..]).unwrap(), packet);
    }
}
