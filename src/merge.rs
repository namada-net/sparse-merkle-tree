use crate::h256::H256;
use crate::traits::{Hasher, Key};

/// Merge two hash
/// this function optimized for ZERO_HASH
/// if one of lhs or rhs is ZERO_HASH, this function just return another one
pub fn merge<H: Hasher + Default>(lhs: &H256, rhs: &H256) -> H256 {
    if lhs.is_zero() {
        return *rhs;
    } else if rhs.is_zero() {
        return *lhs;
    }
    let mut hasher = H::default();
    hasher.write_bytes(lhs.as_slice());
    hasher.write_bytes(rhs.as_slice());
    hasher.finish()
}

/// hash_leaf = hash(prefix | key | value)
/// zero value represent delete the key, this function return zero for zero value
pub fn hash_leaf<H, K>(key: &K, value: &H256) -> H256
where
    H: Hasher + Default,
    K: Key,
{
    if value.is_zero() {
        return H256::zero();
    }
    let mut hasher = H::default();
    hasher.write_bytes(H256::zero().as_slice());
    key.write_bytes(&mut hasher);
    hasher.write_bytes(value.as_slice());
    hasher.finish()
}
