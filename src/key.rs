use crate::H256;
#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
use core::convert::TryFrom;
#[cfg(feature = "borsh")]
use core::convert::TryInto;
use core::ops::{Deref, DerefMut};
use std::fmt::Debug;
#[cfg(feature = "borsh")]
use std::io::Write;

/// Represents bytes that have been right padded with zeros to be
/// an `N`-length byte array.
#[derive(Eq, PartialEq, Debug, Hash, Clone, Copy, PartialOrd, Ord)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub struct PaddedKey<const N: usize> {
    padded: Key<N>,
    #[cfg(not(feature = "utf8-keys"))]
    length: usize,
}

impl<const N: usize> Deref for PaddedKey<N> {
    type Target = Key<N>;

    fn deref(&self) -> &Self::Target {
        &self.padded
    }
}

impl<const N: usize> DerefMut for PaddedKey<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.padded
    }
}

impl<const N: usize> PaddedKey<N> {
    #[cfg(feature = "utf8-keys")]
    pub fn as_slice(&self) -> &[u8] {
        let length = self.padded
            .0
            .iter()
            .enumerate()
            .find(|(_, x) | **x == 0xFF as u8)
            .map(|val | val.0)
            .unwrap_or_else(|| self.padded.0.len());
        &self.padded.0[..length]
    }

    #[cfg(not(feature = "utf8-keys"))]
    pub fn as_slice(&self) -> &[u8] {
        &self.padded.0[..self.length]
    }
}

/// The actual key value used in the tree
#[derive(Eq, PartialEq, Debug, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct Key<const N: usize>([u8; N]);

#[cfg(feature = "borsh")]
impl<const N: usize> BorshSerialize for Key<N> {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let bytes = self.0.to_vec();
        BorshSerialize::serialize(&bytes, writer)
    }
}

#[cfg(feature = "borsh")]
impl<const N: usize> BorshDeserialize for Key<N> {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        use std::io::ErrorKind;
        let bytes: Vec<u8> = BorshDeserialize::deserialize(buf)?;
        let bytes: [u8; N] = bytes.try_into().map_err(|_|
            std::io::Error::new(ErrorKind::InvalidData, "Input byte vector is too large")
        )?;
        Ok(Key(bytes))
    }
}

const BYTE_SIZE: usize = 8;

impl<const N: usize> Key<N> {
    pub const fn zero() -> Self {
        Key([0u8; N])
    }

    pub const fn max_index() -> usize {
        N - 1
    }

    pub fn is_zero(&self) -> bool {
        self == &Self::zero()
    }

    #[inline]
    pub fn get_bit(&self, i: usize) -> bool {
        if i / BYTE_SIZE > Self::max_index() {
            println!("Hey");
        }
        let byte_pos = Self::max_index() - i / BYTE_SIZE;
        let bit_pos = i % BYTE_SIZE;
        let bit = self.0[byte_pos] >> bit_pos & 1;
        bit != 0
    }

    #[inline]
    pub fn set_bit(&mut self, i: usize) {
        let byte_pos = Self::max_index() - i / BYTE_SIZE;
        let bit_pos = i % BYTE_SIZE;
        self.0[byte_pos as usize] |= 1 << bit_pos as u8;
    }

    #[inline]
    pub fn clear_bit(&mut self, i: usize) {
        let byte_pos = Self::max_index() - i / BYTE_SIZE;
        let bit_pos = i % BYTE_SIZE;
        self.0[byte_pos as usize] &= !((1 << bit_pos) as u8);
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0[..]
    }

    /// Treat Key as a path in a tree
    /// fork height is the number of common bits(from higher to lower)
    /// of two Key
    pub fn fork_height(&self, key: &Key<N>) -> usize {
        let max = (BYTE_SIZE * N) as usize;
        for h in (0..max).rev() {
            if self.get_bit(h) != key.get_bit(h) {
                return h;
            }
        }
        0
    }

    /// Treat Key as a path in a tree
    /// return parent_path of self
    pub fn parent_path(&self, height: usize) -> Self {
        height
            .checked_add(1)
            .map(|i| self.copy_bits(i..))
            .unwrap_or_else(Key::zero)
    }

    /// Copy bits and return a new Key
    pub fn copy_bits(&self, range: impl core::ops::RangeBounds<usize>) -> Self {
        let array_size = N;
        let max = 8 * N;
        use core::ops::Bound;

        let mut target = Key::zero();
        let start = match range.start_bound() {
            Bound::Included(&i) => i as usize,
            Bound::Excluded(&i) => panic!("do not allows excluded start: {}", i),
            Bound::Unbounded => 0,
        };

        let mut end = match range.end_bound() {
            Bound::Included(&i) => i.saturating_add(1) as usize,
            Bound::Excluded(&i) => i as usize,
            Bound::Unbounded => max,
        };

        if start >= max {
            return target;
        } else if end > max {
            end = max;
        }

        if end < start {
            panic!("end can't less than start: start {} end {}", start, end);
        }

        let end_byte = {
            let remain = if start % BYTE_SIZE != 0 { 1 } else { 0 };
            array_size - start / BYTE_SIZE - remain
        };
        let start_byte = array_size - end / BYTE_SIZE;
        // copy bytes
        if start_byte < self.0.len() && start_byte <= end_byte {
            target.0[start_byte..end_byte].copy_from_slice(&self.0[start_byte..end_byte]);
        }

        // copy remain bits
        for i in (start..core::cmp::min((array_size - end_byte) * BYTE_SIZE, end))
            .chain(core::cmp::max((array_size - start_byte) * BYTE_SIZE, start)..end)
        {
            if self.get_bit(i) {
                target.set_bit(i)
            }
        }
        target
    }
}

impl<const N: usize> TryFrom<Vec<u8>> for PaddedKey<N> {
    type Error = String;
    fn try_from(v: Vec<u8>) -> Result<Self, String> {
        if v.len() > N {
            Err("Byte vector is too large to be a key".into())
        } else {
            let mut padded = [0xFF as u8; N];
            padded[..v.len()].copy_from_slice(&v);
            #[cfg(feature = "utf8-keys")]
            {
                Ok(PaddedKey {
                    padded: Key::<N>(padded),
                })
            }
            #[cfg(not(feature = "utf8-keys"))]
            {
                Ok(PaddedKey {
                    padded: Key::<N>(padded),
                    length: v.len()
                })
            }
        }
    }
}

impl<const N: usize> From<PaddedKey<N>> for [u8; N] {
    fn from(key: PaddedKey<N>) -> [u8; N] {
        key.padded.0
    }
}

impl From<H256> for PaddedKey<32> {
    fn from(v: H256) -> Self {
        <[u8; 32]>::from(v).into()
    }
}

impl<const N: usize> From<[u8; N]> for PaddedKey<N> {
    fn from(v: [u8; N]) -> Self {
        #[cfg(feature = "utf8-keys")]
        {
            PaddedKey {
                padded: Key::<N>(v),
            }
        }
        #[cfg(not(feature = "utf8-keys"))]
        {
            PaddedKey {
                padded: Key::<N>(v),
                length: N,
            }
        }
    }
}

#[cfg(all(test, feature="utf8-keys"))]
mod test_keys {
    use super::*;

    #[test]
    fn test_padded_key_from_utf8() {
        let ibc_key = "clients/tendermint-0/clientState".as_bytes().to_vec();
        let key = PaddedKey::<120>::try_from(ibc_key.clone()).expect("Test failed");
        let value = String::from_utf8(key.as_slice().to_vec()).expect("Test failed");
        assert_eq!(value, String::from("clients/tendermint-0/clientState"));
        let key = PaddedKey::<32>::try_from(ibc_key).expect("Test failed");
        let value = String::from_utf8(key.as_slice().to_vec()).expect("Test failed");
        assert_eq!(value, String::from("clients/tendermint-0/clientState"));
    }
}