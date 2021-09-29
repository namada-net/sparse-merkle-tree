use crate::{traits::Hasher, H256};
use core::convert::TryInto;
use sha2::{Digest, Sha256};

pub struct Sha256Hasher(Sha256);

impl Default for Sha256Hasher {
    fn default() -> Self {
        Self(Sha256::default())
    }
}

impl Hasher for Sha256Hasher {
    fn write_h256(&mut self, h: &H256) {
        self.0.update(h.as_slice());
    }

    fn finish(self) -> H256 {
        let hash = self.0.finalize();
        let bytes: [u8; 32] = hash
            .as_slice()
            .try_into()
            .expect("Sha256 output conversion to fixed array shouldn't fail");
        bytes.into()
    }
}
