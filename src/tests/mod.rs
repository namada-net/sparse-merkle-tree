mod padded_key;

use super::*;
use crate::{
    blake2b::Blake2bHasher, default_store::DefaultStore, error::Error, sha256::Sha256Hasher,
    MerkleProof, SparseMerkleTree,
};
use core::convert::{TryFrom, TryInto};
use padded_key::PaddedKey;
use proptest::prelude::*;
use rand::prelude::{Rng, SliceRandom};

type Smt<const N: usize> =
    SparseMerkleTree<Blake2bHasher, PaddedKey<N>, H256, DefaultStore<PaddedKey<N>, H256, N>, N>;
type ShaSmt<const N: usize> =
    SparseMerkleTree<Sha256Hasher, PaddedKey<N>, H256, DefaultStore<PaddedKey<N>, H256, N>, N>;

#[test]
fn test_default_root() {
    let mut tree = Smt::<32>::new(H256::zero(), DefaultStore::default());
    assert_eq!(tree.store().branches_map().len(), 0);
    assert_eq!(tree.store().leaves_map().len(), 0);
    assert_eq!(tree.root(), &H256::zero());

    // insert a key-value
    tree.update(H256::zero().into(), [42u8; 32].into())
        .expect("update");
    assert_ne!(tree.root(), &H256::zero());
    assert_ne!(tree.store().branches_map().len(), 0);
    assert_ne!(tree.store().leaves_map().len(), 0);
    let zero: PaddedKey<32> = H256::zero().into();
    assert_eq!(tree.get(&zero).expect("get"), [42u8; 32].into());
    // update zero is to delete the key
    tree.update(H256::zero().into(), H256::zero())
        .expect("update");
    assert_eq!(tree.root(), &H256::zero());
    assert_eq!(tree.get(&zero).expect("get"), H256::zero());
}

#[test]
fn test_default_tree() {
    let tree = Smt::default();
    let zero: PaddedKey<32> = H256::zero().into();
    assert_eq!(tree.get(&zero).expect("get"), H256::zero());
    let proof = tree
        .merkle_proof(vec![H256::zero().into()])
        .expect("merkle proof");
    let root = proof
        .compute_root::<Blake2bHasher, PaddedKey<32>, H256, 32>(vec![(H256::zero().into(), H256::zero())])
        .expect("root");
    assert_eq!(&root, tree.root());
    let proof = tree
        .merkle_proof(vec![H256::zero().into()])
        .expect("merkle proof");
    let root2 = proof
        .compute_root::<Blake2bHasher, PaddedKey<32>, H256, 32>(vec![(
            H256::zero().into(),
            [42u8; 32].into(),
        )])
        .expect("root");
    assert_ne!(&root2, tree.root());
}

#[test]
fn test_default_merkle_proof() {
    let proof = MerkleProof::new(Default::default(), Default::default());
    let result = proof.compute_root::<Blake2bHasher, PaddedKey<50>, H256, 50>(vec![(
        [42u8; 50].into(),
        [42u8; 32].into(),
    )]);
    assert_eq!(
        result.unwrap_err(),
        Error::IncorrectNumberOfLeaves {
            expected: 0,
            actual: 1
        }
    );
    // makes room for leaves
    let proof = MerkleProof::new(vec![Vec::new()], Default::default());
    let root = proof
        .compute_root::<Blake2bHasher, PaddedKey<50>, H256, 50>(vec![(
            [42u8; 50].into(),
            [42u8; 32].into(),
        )])
        .expect("compute root");
    assert_ne!(root, H256::zero());
}

#[test]
fn test_merkle_root() {
    fn new_blake2b() -> blake2b_rs::Blake2b {
        blake2b_rs::Blake2bBuilder::new(32).personal(b"Smt").build()
    }

    let mut tree = Smt::<42>::default();
    for (i, word) in "The quick brown fox jumps over the lazy dog"
        .split_whitespace()
        .enumerate()
    {
        let key: PaddedKey<42> = {
            let mut buf = [0u8; 42];
            let mut hasher = new_blake2b();
            hasher.update(&(i as u32).to_le_bytes());
            hasher.finalize(&mut buf);
            buf.into()
        };
        let value: H256 = {
            let mut buf = [0u8; 32];
            let mut hasher = new_blake2b();
            hasher.update(word.as_bytes());
            hasher.finalize(&mut buf);
            buf.into()
        };
        tree.update(key, value).expect("update");
    }

    let expected_root: H256 = [
        53, 6, 166, 103, 176, 25, 32, 25, 11, 238, 105, 12, 97, 160, 103, 70, 170, 35, 89, 138, 68,
        83, 84, 45, 133, 246, 181, 201, 166, 57, 150, 17,
    ]
    .into();
    assert_eq!(tree.store().leaves_map().len(), 9);
    assert_eq!(tree.root(), &expected_root);
}

#[test]
fn test_zero_value_donot_change_root() {
    let mut tree = Smt::<33>::default();
    let key = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 1,
    ]
    .into();
    let value = H256::zero();
    tree.update(key, value).unwrap();
    assert_eq!(tree.root(), &H256::zero());
    assert_eq!(tree.store().leaves_map().len(), 0);
    assert_eq!(tree.store().branches_map().len(), 0);
}

#[test]
fn test_zero_value_donot_change_store() {
    let mut tree = Smt::<30>::default();
    let key = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]
    .into();
    let value = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1,
    ]
    .into();
    tree.update(key, value).unwrap();
    assert_ne!(tree.root(), &H256::zero());
    let root = *tree.root();
    let store = tree.store().clone();

    // insert a zero value leaf
    let key = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
    ]
    .into();
    let value = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]
    .into();
    tree.update(key, value).unwrap();
    assert_eq!(tree.root(), &root);
    assert_eq!(tree.store().leaves_map(), store.leaves_map());
    assert_eq!(tree.store().branches_map(), store.branches_map());
}

#[test]
fn test_delete_a_leaf() {
    let mut tree = Smt::<32>::default();
    let key = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]
    .into();
    let value = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1,
    ]
    .into();
    tree.update(key, value).unwrap();
    assert_ne!(tree.root(), &H256::zero());
    let root = *tree.root();
    let store = tree.store().clone();

    // insert a leaf
    let key = [
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]
    .into();
    let value = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1,
    ]
    .into();
    tree.update(key, value).unwrap();
    assert_ne!(tree.root(), &root);

    // delete a leaf
    tree.update(key, H256::zero()).unwrap();
    assert_eq!(tree.root(), &root);
    assert_eq!(tree.store().leaves_map(), store.leaves_map());
    assert_eq!(tree.store().branches_map(), store.branches_map());
}

fn test_construct(key: PaddedKey<10>, value: H256) {
    // insert same value to sibling key will construct a different root

    let mut tree = Smt::<10>::default();
    tree.update(key, value).expect("update");

    let mut sibling_key = key;
    if sibling_key.get_bit(0) {
        sibling_key.clear_bit(0);
    } else {
        sibling_key.set_bit(0);
    }
    let mut tree2 = Smt::default();
    tree2.update(sibling_key, value).expect("update");
    assert_ne!(tree.root(), tree2.root());
}

fn test_update(key: PaddedKey<31>, value: H256) {
    let mut tree = Smt::<31>::default();
    tree.update(key, value).expect("update");
    assert_eq!(tree.get(&key), Ok(value));
}

fn test_update_tree_store(key: PaddedKey<25>, value: H256, value2: H256) {
    const EXPECTED_BRANCHES_LEN: usize = 1;
    const EXPECTED_LEAVES_LEN: usize = 1;

    let mut tree = Smt::<25>::default();
    tree.update(key, value).expect("update");
    assert_eq!(tree.store().branches_map().len(), EXPECTED_BRANCHES_LEN);
    assert_eq!(tree.store().leaves_map().len(), EXPECTED_LEAVES_LEN);
    tree.update(key, value2).expect("update");
    assert_eq!(tree.store().branches_map().len(), EXPECTED_BRANCHES_LEN);
    assert_eq!(tree.store().leaves_map().len(), EXPECTED_LEAVES_LEN);
    assert_eq!(tree.get(&key), Ok(value2));
}

fn test_merkle_proof(key: PaddedKey<32>, value: H256) {
    const EXPECTED_PROOF_SIZE: usize = 16;

    let mut tree = Smt::<32>::default();
    tree.update(key, value).expect("update");
    if !tree.is_empty() {
        let proof = tree.merkle_proof(vec![key]).expect("proof");
        let compiled_proof = proof
            .clone()
            .compile(vec![(key, value)])
            .expect("compile proof");
        assert!(proof.proof().len() < EXPECTED_PROOF_SIZE);
        assert!(proof
            .verify::<Blake2bHasher, PaddedKey<32>, H256, 32>(tree.root(), vec![(key, value)])
            .expect("verify"));
        assert!(compiled_proof
            .verify::<Blake2bHasher, PaddedKey<32>, H256, 32>(tree.root(), vec![(key, value)])
            .expect("compiled verify"));
    }
}

fn new_smt<const N: usize>(pairs: Vec<(PaddedKey<N>, H256)>) -> Smt<N> {
    let mut smt = Smt::<N>::default();
    for (key, value) in pairs {
        smt.update(key, value).unwrap();
    }
    smt
}

fn new_sha_smt<const N: usize>(pairs: Vec<(PaddedKey<N>, H256)>) -> ShaSmt<N> {
    let mut smt = ShaSmt::<N>::default();
    for (key, value) in pairs {
        smt.update(key, value).unwrap();
    }
    smt
}

#[test]
fn test_ics23_non_membership_proof() {
    use rand::Rng;
    let pairs: Vec<(PaddedKey<115>, H256)> = (0u8..20)
        .into_iter()
        .map(|i| {
            (
                PaddedKey::<115>::try_from(vec![i; 29]).expect("Test failed"),
                H256::from(rand::thread_rng().gen::<[u8; 32]>()),
            )
        })
        .collect();
    let smt = new_sha_smt::<115>(pairs);
    let spec = proof_ics23::get_spec(ics23::HashOp::Sha256);
    let root = smt.root().as_slice().to_vec();
    let non_existent_key =
        PaddedKey::<115>::try_from("Non existent key".as_bytes().to_vec()).expect("Test failed");
    assert_eq!(
        String::from_utf8(non_existent_key.to_vec()).expect("Test failed"),
        String::from("Non existent key")
    );
    let proof = smt
        .non_membership_proof(&non_existent_key)
        .expect("gen proof");
    assert!(ics23::verify_non_membership(
        &proof,
        &spec,
        &root,
        &non_existent_key.to_vec()
    ));
}

#[test]
fn test_ics23_membership_proof() {
    use rand::Rng;
    let pairs: Vec<(PaddedKey<115>, H256)> = (0u8..20)
        .into_iter()
        .map(|i| {
            (
                PaddedKey::<115>::try_from(vec![i; 29]).expect("Test failed"),
                H256::from(rand::thread_rng().gen::<[u8; 32]>()),
            )
        })
        .collect();
    let mut smt = new_sha_smt::<115>(pairs);
    let spec = proof_ics23::get_spec(ics23::HashOp::Sha256);
    let existent_key =
        PaddedKey::<115>::try_from("Existent key".as_bytes().to_vec()).expect("Test failed");
    smt.update(existent_key, H256::from([42u8; 32]))
        .expect("Test failed");
    let root = smt.root().as_slice().to_vec();
    assert_eq!(
        String::from_utf8(existent_key.to_vec()).expect("Test failed"),
        String::from("Existent key")
    );
    let proof = smt.membership_proof(&existent_key).expect("gen proof");
    assert!(ics23::verify_membership(
        &proof,
        &spec,
        &root,
        &existent_key.to_vec(),
        [42u8; 32].as_slice()
    ));
}

fn leaves(
    min_leaves: usize,
    max_leaves: usize,
) -> impl Strategy<Value = (Vec<(PaddedKey<29>, H256)>, usize)> {
    prop::collection::vec(
        prop::array::uniform2(prop::array::uniform32(1u8..0xF8)),
        min_leaves..=max_leaves,
    )
    .prop_flat_map(|mut pairs| {
        pairs.dedup_by_key(|[k, _v]| *k);
        let len = pairs.len();
        (
            Just(
                pairs
                    .into_iter()
                    .map(|[k, v]| (k[..29].to_vec().try_into().expect("Test failed"), v.into()))
                    .collect(),
            ),
            core::cmp::min(1, len)..=len,
        )
    })
}

proptest! {
    #[test]
    fn test_h256(key: [u8; 32], key2: [u8; 32]) {
        let mut list1: Vec<H256> = vec![key.into() , key2.into()];
        let mut list2 = list1.clone();
        // sort H256
        list1.sort_unstable_by_key(|k| *k);
        // sort by high bits to lower bits
        list2.sort_unstable_by(|k1, k2| {
            for i in (0u8..=255).rev() {
                let b1 = if k1.get_bit(i) { 1 } else { 0 };
                let b2 = if k2.get_bit(i) { 1 } else { 0 };
                let o = b1.cmp(&b2);
                if o != std::cmp::Ordering::Equal {
                    return o;
                }
            }
            std::cmp::Ordering::Equal
        });
        assert_eq!(list1, list2);
    }

    #[test]
    fn test_h256_copy_bits(start in 0u8..254u8, size in 1u8..255u8) {
        let one: H256 = [255u8; 32].into();
        let target = one.copy_bits(start..(start.saturating_add(size)));
        for i in start..start.saturating_add(size) {
            assert_eq!(one.get_bit(i as u8), target.get_bit(i as u8));
        }
        for i in 0..start {
            assert!(!target.get_bit(i as u8));
        }
        if let Some(start_i) = start.checked_add(size).and_then(|i| i.checked_add(1)){
            for i in start_i..=255 {
                assert!(!target.get_bit(i as u8));
            }
        }
    }

    #[test]
    fn test_padded_key_copy_bits(start in 0usize..319usize, size in 1usize..319usize) {
        let one: PaddedKey<40> = [255u8; 40].into();
        let target = one.copy_bits(start..(start.saturating_add(size)));
        for i in start..start.saturating_add(size) {
            if i >= 320 {
                continue;
            }
            assert_eq!(one.get_bit(i), target.get_bit(i));
        }
        for i in 0..start {
            assert!(!target.get_bit(i));
        }
        if let Some(start_i) = start.checked_add(size).and_then(|i| i.checked_add(1)){
            for i in start_i..320 {
                assert!(!target.get_bit(i));
            }
        }
    }

    #[test]
    fn test_random_update(key: [u8; 31], value: [u8;32]) {
        test_update(key.into(), value.into());
    }

    #[test]
    fn test_random_update_tree_store(key: [u8; 25], value: [u8;32], value2: [u8;32]) {
        test_update_tree_store(key.into(), value.into(), value2.into());
    }

    #[test]
    fn test_random_construct(key: [u8;10], value: [u8;32]) {
        test_construct(key.into(), value.into());
    }

    #[test]
    fn test_random_merkle_proof(key: [u8; 32], value: [u8;32]) {
        test_merkle_proof(key.into(), value.into());
    }

    #[test]
    fn test_smt_single_leaf_small((pairs, _n) in leaves(1, 50)) {
        let smt = new_smt::<29>(pairs.clone());
        for (k, v) in pairs {
            let proof = smt.merkle_proof(vec![k]).expect("gen proof");
            let compiled_proof = proof.clone().compile(vec![(k, v)]).expect("compile proof");
            assert!(proof.verify::<Blake2bHasher, PaddedKey<29>, H256, 29>(smt.root(), vec![(k, v)]).expect("verify proof"));
            assert!(compiled_proof.verify::<Blake2bHasher, PaddedKey<29>, H256, 29>(smt.root(), vec![(k, v)]).expect("verify compiled proof"));
        }
    }

    #[test]
    fn test_smt_single_leaf_large((pairs, _n) in leaves(50, 100)) {
        let smt = new_smt::<29>(pairs.clone());
        for (k, v) in pairs {
            let proof = smt.merkle_proof(vec![k]).expect("gen proof");
            let compiled_proof = proof.clone().compile(vec![(k, v)]).expect("compile proof");
            assert!(proof.verify::<Blake2bHasher, PaddedKey<29>, H256, 29>(smt.root(), vec![(k, v)]).expect("verify proof"));
            assert!(compiled_proof.verify::<Blake2bHasher, PaddedKey<29>, H256, 29>(smt.root(), vec![(k, v)]).expect("verify compiled proof"));
        }
    }

    #[test]
    fn test_smt_multi_leaves_small((pairs, n) in leaves(1, 50)){
        let smt = new_smt::<29>(pairs.clone());
        let proof = smt.merkle_proof(pairs.iter().take(n).map(|(k, _v)| *k).collect()).expect("gen proof");
        let data: Vec<(PaddedKey<29>, H256)> = pairs.into_iter().take(n).collect();
        let compiled_proof = proof.clone().compile(data.clone()).expect("compile proof");
        assert!(proof.verify::<Blake2bHasher, PaddedKey<29>, H256, 29>(smt.root(), data.clone()).expect("verify proof"));
        assert!(compiled_proof.verify::<Blake2bHasher, PaddedKey<29>, H256, 29>(smt.root(), data).expect("verify compiled proof"));
    }

    #[test]
    fn test_smt_multi_leaves_large((pairs, _n) in leaves(50, 100)){
        let n = 20;
        let pairs: Vec<_> = pairs
            .into_iter()
            .map(|(key, v)| (
            PaddedKey::<120>::try_from(key.to_vec()).unwrap(),
            v))
            .collect();
        let smt = new_smt::<120>(pairs.clone());
        let proof = smt.merkle_proof(pairs.iter().take(n).map(|(k, _v)| *k).collect()).expect("gen proof");
        let data: Vec<(PaddedKey<120>, H256)> = pairs.into_iter().take(n).collect();
        let compiled_proof = proof.clone().compile(data.clone()).expect("compile proof");
        assert!(proof.verify::<Blake2bHasher, PaddedKey<120>, H256, 120>(smt.root(), data.clone()).expect("verify proof"));
        assert!(compiled_proof.verify::<Blake2bHasher, PaddedKey<120>, H256, 120>(smt.root(), data).expect("verify compiled proof"));
    }

    #[test]
    fn test_smt_non_exists_leaves((pairs, _n) in leaves(1, 20), (pairs2, _n2) in leaves(1, 5)){
        let smt = new_smt::<29>(pairs);
        let non_exists_keys: Vec<_> = pairs2.into_iter().map(|(k, _v)|k).collect();
        let proof = smt.merkle_proof(non_exists_keys.clone()).expect("gen proof");
        let data: Vec<(PaddedKey<29>, H256)> = non_exists_keys.into_iter().map(|k|(k, H256::zero())).collect();
        let compiled_proof = proof.clone().compile(data.clone()).expect("compile proof");
        assert!(proof.verify::<Blake2bHasher, PaddedKey<29>, H256, 29>(smt.root(), data.clone()).expect("verify proof"));
        assert!(compiled_proof.verify::<Blake2bHasher, PaddedKey<29>, H256, 29>(smt.root(), data).expect("verify compiled proof"));
    }

    #[test]
    fn test_smt_non_exists_leaves_mix((pairs, _n) in leaves(1, 20), (pairs2, _n2) in leaves(1, 5)){
        let smt = new_smt::<29>(pairs.clone());
        let exists_keys: Vec<_> = pairs.into_iter().map(|(k, _v)|k).collect();
        let non_exists_keys: Vec<_> = pairs2.into_iter().map(|(k, _v)|k).collect();
        let exists_keys_len = std::cmp::max(exists_keys.len() / 2, 1);
        let non_exists_keys_len = std::cmp::max(non_exists_keys.len() / 2, 1);
        let mut keys: Vec<_> = exists_keys.into_iter().take(exists_keys_len).chain(non_exists_keys.into_iter().take(non_exists_keys_len)).collect();
        keys.dedup();
        let proof = smt.merkle_proof(keys.clone()).expect("gen proof");
        let data: Vec<(PaddedKey<29>, H256)> = keys.into_iter().map(|k|(k, smt.get(&k).expect("get"))).collect();
        let compiled_proof = proof.clone().compile(data.clone()).expect("compile proof");
        assert!(proof.verify::<Blake2bHasher, PaddedKey<29>, H256, 29>(smt.root(), data.clone()).expect("verify proof"));
        assert!(compiled_proof.verify::<Blake2bHasher, PaddedKey<29>, H256, 29>(smt.root(), data).expect("verify compiled proof"));
    }

    #[test]
    fn test_update_smt_tree_store((pairs, n) in leaves(1, 20)) {
        let smt = new_smt::<29>(pairs.clone());
        for (k, v) in pairs.into_iter().take(n) {
            assert_eq!(smt.get(&k), Ok(v));
        }
    }

    #[test]
    fn test_smt_random_insert_order((pairs, _n) in leaves(5, 30)){
        let mut pairs: Vec<(PaddedKey<40>, H256)> = pairs
        .into_iter()
        .map(|(key, v)| (PaddedKey::<40>::try_from(<[u8; 29]>::from(key).to_vec()).expect("Test failed"), v))
        .collect();
        let smt = new_smt::<40>(pairs.clone());
        let root = *smt.root();
        let mut rng = rand::thread_rng();
        for _i in 0..10 {
            pairs.shuffle(&mut rng);
            let smt = new_smt::<40>(pairs.clone());
            let current_root = *smt.root();
            assert_eq!(root, current_root);
        }
    }

    #[test]
    fn test_smt_update_with_zero_values((pairs, _n) in leaves(5, 30)){
        let mut rng = rand::thread_rng();
        let len =  rng.gen_range(0..pairs.len());
        let mut smt = new_smt::<29>(pairs[..len].to_vec());
        let root = *smt.root();

        // insert zero values
        for (k, _v) in pairs[len..].iter() {
            smt.update(*k, H256::zero()).unwrap();
        }
        // check root
        let current_root = *smt.root();
        assert_eq!(root, current_root);
        // check inserted pairs
        for (k, v) in pairs[..len].iter() {
            let value = smt.get(k).unwrap();
            assert_eq!(v, &value);
        }
    }

    #[test]
    fn test_ics23_proof_single_leaf_small((pairs, _n) in leaves(1, 50)){
        let pairs: Vec<(PaddedKey<120>, H256)> = pairs
        .into_iter()
        .map(|(key, v)| (PaddedKey::<120>::try_from(<[u8; 29]>::from(key).to_vec()).expect("Test failed"), v))
        .collect();
        let smt = new_sha_smt::<120>(pairs.clone());
        let spec = proof_ics23::get_spec(ics23::HashOp::Sha256);
        let root = smt.root().as_slice().to_vec();
        for (k, v) in pairs {
            let proof = smt.membership_proof(&k).expect("gen proof");
            assert!(ics23::verify_membership(&proof, &spec, &root, &k.to_vec(), v.as_slice()));
        }
    }

    #[test]
    fn test_ics23_proof_non_exists_leaves((pairs, _n) in leaves(1, 20), (pairs2, _n2) in leaves(1, 5)) {
       let pairs: Vec<(PaddedKey<115>, H256)> = pairs
        .into_iter()
        .filter_map(|(key, v)| PaddedKey::<115>::try_from(<[u8; 29]>::from(key).to_vec()).ok().map(|k| (k, v)))
        .collect();
        let pairs2: Vec<(PaddedKey<115>, H256)> = pairs2
        .into_iter()
        .filter_map(|(key, v)| PaddedKey::<115>::try_from(<[u8; 29]>::from(key).to_vec()).ok().map(|k| (k, v)))
        .collect();
        let smt = new_sha_smt::<115>(pairs.clone());
        let spec = proof_ics23::get_spec(ics23::HashOp::Sha256);
        let root = smt.root().as_slice().to_vec();
        let exists_key: Vec<_> = pairs.into_iter().map(|(k, _v)|k).collect();
        let non_exists_keys: Vec<_> = pairs2.into_iter().map(|(k, _v)|k).filter(|k| !exists_key.contains(k)).collect();
        for k in non_exists_keys {
            let proof = smt.non_membership_proof(&k).expect("gen proof");
            assert!(ics23::verify_non_membership(&proof, &spec, &root, &k.to_vec()));
        }
    }
}

#[test]
fn test_v0_2_broken_sample() {
    fn parse_h256(s: &str) -> H256 {
        let data = hex::decode(s).unwrap();
        let mut inner = [0u8; 32];
        inner.copy_from_slice(&data);
        H256::from(inner)
    }

    let keys = vec![
        "0000000000000000000000000000000000000000000000000000000000000000",
        "0000000000000000000000000000000000000000000000000000000000000002",
        "0000000000000000000000000000000000000000000000000000000000000003",
        "0000000000000000000000000000000000000000000000000000000000000004",
        "0000000000000000000000000000000000000000000000000000000000000005",
        "0000000000000000000000000000000000000000000000000000000000000006",
        "000000000000000000000000000000000000000000000000000000000000000e",
        "f652222313e28459528d920b65115c16c04f3efc82aaedc97be59f3f377c0d3f",
        "f652222313e28459528d920b65115c16c04f3efc82aaedc97be59f3f377c0d40",
        "5eff886ea0ce6ca488a3d6e336d6c0f75f46d19b42c06ce5ee98e42c96d256c7",
        "6d5257204ebe7d88fd91ae87941cb2dd9d8062b64ae5a2bd2d28ec40b9fbf6df",
    ]
    .into_iter()
    .map(|key| parse_h256(key).into());
    let values = vec![
        "000000000000000000000000c8328aabcd9b9e8e64fbc566c4385c3bdeb219d7",
        "000000000000000000000001c8328aabcd9b9e8e64fbc566c4385c3bdeb219d7",
        "0000384000001c2000000e1000000708000002580000012c000000780000003c",
        "000000000000000000093a80000546000002a300000151800000e10000007080",
        "000000000000000000000000000000000000000000000000000000000000000f",
        "0000000000000000000000000000000000000000000000000000000000000001",
        "00000000000000000000000000000000000000000000000000071afd498d0000",
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "0000000000000000000000000000000000000000000000000000000000000000",
        "0000000000000000000000000000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000000",
    ]
    .into_iter()
    .map(parse_h256);
    let mut pairs = keys.into_iter().zip(values.into_iter()).collect::<Vec<_>>();
    let smt = new_smt::<32>(pairs.clone());
    let base_root = *smt.root();

    // insert in random order
    let mut rng = rand::thread_rng();
    for _i in 0..10 {
        pairs.shuffle(&mut rng);
        let smt = new_smt(pairs.clone());
        let current_root = *smt.root();
        assert_eq!(base_root, current_root);
    }
}

#[test]
fn test_v0_3_broken_sample() {
    let k1 = [
        0u8, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];
    let v1 = [
        108u8, 153, 9, 238, 15, 28, 173, 182, 146, 77, 52, 203, 162, 151, 125, 76, 55, 176, 192,
        104, 170, 5, 193, 174, 137, 255, 169, 176, 132, 64, 199, 115,
    ];
    let k2 = [
        1u8, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];
    let v2 = [
        0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];
    let k3 = [
        1u8, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];
    let v3 = [
        0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];

    let mut smt = Smt::<32>::default();
    // inserted keys shouldn't interfere with each other
    assert_ne!(k1, k2);
    assert_ne!(k2, k3);
    assert_ne!(k1, k3);
    smt.update(k1.into(), v1.into()).unwrap();
    smt.update(k2.into(), v2.into()).unwrap();
    smt.update(k3.into(), v3.into()).unwrap();
    assert_eq!(smt.get(&k1.into()).unwrap(), v1.into());
}
