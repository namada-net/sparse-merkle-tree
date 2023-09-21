#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "borsh")]
use core::convert::TryInto;
use std::fmt::Debug;
use std::io::Read;
#[cfg(feature = "borsh")]
use std::io::Write;

/// The actual key value used in the tree
#[derive(Eq, PartialEq, Debug, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct InternalKey<const N: usize>([u8; N]);

#[cfg(feature = "borsh")]
impl<const N: usize> BorshSerialize for InternalKey<N> {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let bytes = self.0.to_vec();
        BorshSerialize::serialize(&bytes, writer)
    }
}

#[cfg(feature = "borsh")]
impl<const N: usize> BorshDeserialize for InternalKey<N> {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        use std::io::ErrorKind;
        let bytes: Vec<u8> = BorshDeserialize::deserialize_reader(reader)?;
        let bytes: [u8; N] = bytes.try_into().map_err(|_| {
            std::io::Error::new(ErrorKind::InvalidData, "Input byte vector is too large")
        })?;
        Ok(InternalKey(bytes))
    }
}

const BYTE_SIZE: usize = 8;

impl<const N: usize> InternalKey<N> {
    pub fn new(array: [u8; N]) -> Self {
        Self(array)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0[..]
    }

    pub const fn zero() -> Self {
        InternalKey([0u8; N])
    }

    pub const fn max_index() -> usize {
        N - 1
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

    /// Treat InternalKey as a path in a tree
    /// fork height is the number of common bits(from higher to lower)
    /// of two InternalKey
    pub fn fork_height(&self, key: &InternalKey<N>) -> usize {
        let max = (BYTE_SIZE * N) as usize;
        for h in (0..max).rev() {
            if self.get_bit(h) != key.get_bit(h) {
                return h;
            }
        }
        0
    }

    /// Treat InternalKey as a path in a tree
    /// return parent_path of self
    pub fn parent_path(&self, height: usize) -> Self {
        height
            .checked_add(1)
            .map(|i| self.copy_bits(i..))
            .unwrap_or_else(InternalKey::zero)
    }

    /// Copy bits and return a new InternalKey
    pub fn copy_bits(&self, range: impl core::ops::RangeBounds<usize>) -> Self {
        let array_size = N;
        let max = 8 * N;
        use core::ops::Bound;

        let mut target = InternalKey::zero();
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

impl<const N: usize> From<[u8; N]> for InternalKey<N> {
    fn from(v: [u8; N]) -> Self {
        Self::new(v)
    }
}

impl<const N: usize> From<InternalKey<N>> for [u8; N] {
    fn from(v: InternalKey<N>) -> Self {
        v.0
    }
}
