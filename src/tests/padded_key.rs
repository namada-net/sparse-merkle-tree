//! A type to test custom implementations of mapping from
//! application specific keys to internal keys.

use crate::H256;
use crate::{Key, InternalKey};
#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
use core::convert::TryFrom;
use core::fmt::Debug;
use core::ops::{Deref, DerefMut};

/// Represents bytes that have been right padded with zeros to be
/// an `N`-length byte array.
///
/// This is handy default type for this library
#[derive(Eq, PartialEq, Debug, Hash, Clone, Copy)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub struct PaddedKey<const N: usize> {
    pub padded: InternalKey<N>,
    pub length: usize,
}

impl<const N: usize> Deref for PaddedKey<N> {
    type Target = InternalKey<N>;

    fn deref(&self) -> &Self::Target {
        &self.padded
    }
}

impl<const N: usize> DerefMut for PaddedKey<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.padded
    }
}

impl<const N: usize> Key<N> for PaddedKey<N> {
    type Error = crate::error::Error;
    fn to_vec(&self) -> Vec<u8> {
        let array: [u8; N] = self.padded.into();
        array[..self.length].to_vec()
    }

    fn as_slice(&self) -> &[u8] {
        &self.padded.as_slice()[..self.length]
    }

    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from(bytes.to_vec())
    }
}

impl<const N: usize> TryFrom<Vec<u8>> for PaddedKey<N> {
    type Error = crate::error::Error;
    fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
        if v.len() > N {
            Err(crate::error::Error::KeyTooLarge)
        } else {
            let mut padded = [0xFF_u8; N];
            padded[..v.len()].copy_from_slice(&v);
            Ok(PaddedKey {
                padded: InternalKey::<N>::new(padded),
                length: v.len(),
            })
        }
    }
}

impl<const N: usize> From<PaddedKey<N>> for [u8; N] {
    fn from(key: PaddedKey<N>) -> [u8; N] {
        key.padded.into()
    }
}

impl From<H256> for PaddedKey<32> {
    fn from(v: H256) -> Self {
        <[u8; 32]>::from(v).into()
    }
}

impl<const N: usize> From<[u8; N]> for PaddedKey<N> {
    fn from(v: [u8; N]) -> Self {
        PaddedKey {
            padded: InternalKey::<N>::new(v),
            length: N,
        }
    }
}
