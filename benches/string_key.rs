use std::ops::Deref;

use rand::Rng;
use random_string::generate;
use nam_sparse_merkle_tree::{InternalKey, Key};

pub const IBC_KEY_LIMIT: usize = 300;
pub const ICS_IDENTIFIER_CHARSET: &str = "1234567890abcdefghijklmnopqrstuvwxyz._+-#[]<>";

#[derive(Eq, PartialEq, Copy, Clone, Hash)]
pub struct StringKey {
    /// The original key string, in bytes
    pub original: [u8; IBC_KEY_LIMIT],
    /// The utf8 bytes representation of the key to be
    /// used internally in the merkle tree
    pub tree_key: InternalKey<IBC_KEY_LIMIT>,
    /// The length of the input (without the padding)
    pub length: usize,
}

impl Deref for StringKey {
    type Target = InternalKey<IBC_KEY_LIMIT>;

    fn deref(&self) -> &Self::Target {
        &self.tree_key
    }
}

impl Key<IBC_KEY_LIMIT> for StringKey {
    type Error = String;

    fn as_slice(&self) -> &[u8] {
        &self.original.as_slice()[..self.length]
    }

    fn try_from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut tree_key = [0u8; IBC_KEY_LIMIT];
        let mut original = [0u8; IBC_KEY_LIMIT];
        let mut length = 0;
        for (i, byte) in bytes.iter().enumerate() {
            if i >= IBC_KEY_LIMIT {
                return Err(
                    "Input IBC key is too large".into(),
                );
            }
            original[i] = *byte;
            tree_key[i] = byte.wrapping_add(1);
            length += 1;
        }
        Ok(Self {
            original,
            tree_key: tree_key.into(),
            length,
        })
    }
}

enum Identifier {
    Port,
    Client,
    Connection,
    Channel
}

/// Generate a random identifier complying with ICS
fn random_identifier(id: Identifier, rng: &mut impl Rng) -> String {
    let range = match id {
        Identifier::Port => 2..=128usize,
        Identifier::Client => 9..=64,
        Identifier::Connection => 10..=64,
        Identifier::Channel => 8..=64,
    };
    generate(rng.gen_range(range), ICS_IDENTIFIER_CHARSET)
}

/// Generate a random path in storage specified by ICS
fn random_ics_path(rng: &mut impl Rng) -> String {
    match rng.gen_range(0..13u8) {
        0 => format!("clients/{}/clientType", random_identifier(Identifier::Client, rng)),
        1 => format!("clients/{}/clientState", random_identifier(Identifier::Client, rng)),
        2 => format!(
            "clients/{}/consensusStates/{}",
            random_identifier(Identifier::Client, rng),
            rng.gen_range(1..=128u8)
        ),
        3 => format!("clients/{}/connections", random_identifier(Identifier::Client, rng)),
        4 => format!("connections/{}", random_identifier(Identifier::Connection, rng)),
        5 => format!("ports/{}", random_identifier(Identifier::Port, rng)),
        6 => format!(
            "channelEnds/ports/{}/channels/{}",
            random_identifier(Identifier::Port, rng),
            random_identifier(Identifier::Channel, rng),
        ),
        7 => format!(
            "nextSequenceSend/ports/{}/channels/{}",
            random_identifier(Identifier::Port, rng),
            random_identifier(Identifier::Channel, rng),
        ),
        8 => format!(
            "nextSequenceRecv/ports/{}/channels/{}",
            random_identifier(Identifier::Port, rng),
            random_identifier(Identifier::Channel, rng),
        ),
        9 => format!(
            "nextSequenceAck/ports/{}/channels/{}",
            random_identifier(Identifier::Port, rng),
            random_identifier(Identifier::Channel, rng),
        ),
        10 => format!(
            "commitments/ports/{}/channels/{}/sequences/{}",
            random_identifier(Identifier::Port, rng),
            random_identifier(Identifier::Channel, rng),
            rng.gen_range(0..=128u8),
        ),
        11 => format!(
            "receipts/ports/{}/channels/{}/sequences/{}",
            random_identifier(Identifier::Port, rng),
            random_identifier(Identifier::Channel, rng),
            rng.gen_range(0..=128u8),
        ),
        12 => format!(
            "acks/ports/{}/channels/{}/sequences/{}",
            random_identifier(Identifier::Port, rng),
            random_identifier(Identifier::Channel, rng),
            rng.gen_range(0..=128u8),
        ),
        _ => unreachable!(),
    }
}

pub fn random_stringkey(rng: &mut impl Rng) -> StringKey  {
    let bytes = random_ics_path(rng).into_bytes();
    StringKey::try_from_bytes(bytes.as_slice()).expect("Should not fail")
}