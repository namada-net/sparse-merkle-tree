#[macro_use]
extern crate criterion;

mod string_key;

use criterion::{BenchmarkId, Criterion};
use rand::{thread_rng, Rng};
use nam_sparse_merkle_tree::{
    sha256::Sha256Hasher, default_store::DefaultStore,
    tree::SparseMerkleTree, H256, Hash
};
use string_key::{IBC_KEY_LIMIT, StringKey, random_stringkey};


const TARGET_LEAVES_COUNT: usize = 20;

type ShaSmt = SparseMerkleTree<Sha256Hasher, Hash, H256, DefaultStore<Hash, H256, 32>, 32>;
type StringSmt = SparseMerkleTree<Sha256Hasher, StringKey, H256, DefaultStore<StringKey, H256, IBC_KEY_LIMIT>, IBC_KEY_LIMIT>;

fn random_h256(rng: &mut impl Rng) -> H256 {
    let mut buf = [0u8; 32];
    rng.fill(&mut buf);
    buf.into()
}

fn random_shasmt(update_count: usize, rng: &mut impl Rng) -> (ShaSmt, Vec<Hash>) {
    let mut smt = ShaSmt::default();
    let mut keys = Vec::with_capacity(update_count);
    for _ in 0..update_count {
        let key = random_h256(rng);
        let value = random_h256(rng);
        smt.update(key.into(), value).unwrap();
        keys.push(key.into());
    }
    (smt, keys)
}

fn random_stringsmt(update_count: usize, rng: &mut impl Rng) -> (StringSmt, Vec<StringKey>) {
    let mut smt = StringSmt::default();
    let mut keys = Vec::with_capacity(update_count);
    for _ in 0..update_count {
        let key = random_stringkey(rng);
        let value = random_h256(rng);
        smt.update(key, value).unwrap();
        keys.push(key);
    }
    (smt, keys)
}

fn bench_hashes(c: &mut Criterion) {
    let mut group = c.benchmark_group("ShaSmt update");
    for size in [100, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut rng = thread_rng();
                    random_shasmt(size, &mut rng)
                });
            }
        );
    }
    group.finish();

    let mut group = c.benchmark_group("ShaSmt get");
    for size in [5_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                let mut rng = thread_rng();
                let (smt, _keys) = random_shasmt(size, &mut rng);
                b.iter(|| {
                    let key = random_h256(&mut rng).into();
                    smt.get(&key).unwrap();
                });
            }
        );
    }
    group.finish();

    c.bench_function("ShaSmt generate merkle proof", |b| {
        let mut rng = thread_rng();
        let (smt, mut keys) = random_shasmt(10_000, &mut rng);
        keys.dedup();
        let keys: Vec<_> = keys.into_iter().take(TARGET_LEAVES_COUNT).collect();
        b.iter(|| {
            smt.merkle_proof(keys.clone()).unwrap();
        });
    });

    c.bench_function("ShaSmt verify merkle proof", |b| {
        let mut rng = thread_rng();
        let (smt, mut keys) = random_shasmt(10_000, &mut rng);
        keys.dedup();
        let leaves: Vec<_> = keys
            .iter()
            .take(TARGET_LEAVES_COUNT)
            .map(|k| (*k, smt.get(k).unwrap()))
            .collect();
        let proof = smt
            .merkle_proof(keys.into_iter().take(TARGET_LEAVES_COUNT).collect())
            .unwrap();
        let root = smt.root();
        b.iter(|| {
            let valid = proof.clone().verify::<Sha256Hasher, Hash, H256, 32>(root, leaves.clone());
            assert!(valid.expect("verify result"));
        });
    });

    c.bench_function("ShaSmt validate tree", |b| {
        let mut rng = thread_rng();
        let (smt, _) = random_shasmt(10_000, &mut rng);
        b.iter(||{ assert!(smt.validate()) });
    });
}

fn bench_strings(c: &mut Criterion) {
    let mut group = c.benchmark_group("StringSmt update");
    for size in [100, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut rng = thread_rng();
                    random_stringsmt(size, &mut rng)
                });
            }
        );
    }
    group.finish();

    let mut group = c.benchmark_group("StringSmt get");
    for size in [5_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                let mut rng = thread_rng();
                let (smt, _keys) = random_stringsmt(size, &mut rng);
                b.iter(|| {
                    let key = random_stringkey(&mut rng).into();
                    smt.get(&key).unwrap();
                });
            }
        );
    }
    group.finish();

    c.bench_function("StringSmt generate merkle proof", |b| {
        let mut rng = thread_rng();
        let (smt, mut keys) = random_stringsmt(10_000, &mut rng);
        keys.dedup();
        let keys: Vec<_> = keys.into_iter().take(TARGET_LEAVES_COUNT).collect();
        b.iter(|| {
            smt.merkle_proof(keys.clone()).unwrap();
        });
    });

    c.bench_function("StringSmt verify merkle proof", |b| {
        let mut rng = thread_rng();
        let (smt, mut keys) = random_stringsmt(10_000, &mut rng);
        keys.dedup();
        let leaves: Vec<_> = keys
            .iter()
            .take(TARGET_LEAVES_COUNT)
            .map(|k| (*k, smt.get(k).unwrap()))
            .collect();
        let proof = smt
            .merkle_proof(keys.into_iter().take(TARGET_LEAVES_COUNT).collect())
            .unwrap();
        let root = smt.root();
        b.iter(|| {
            let valid = proof.clone().verify::<Sha256Hasher, StringKey, H256, IBC_KEY_LIMIT>(root, leaves.clone());
            assert!(valid.expect("verify result"));
        });
    });

    c.bench_function("StringSmt validate tree", |b| {
        let mut rng = thread_rng();
        let (smt, _) = random_stringsmt(10_000, &mut rng);
        b.iter(||{ assert!(smt.validate()) });
    });

}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_strings
);
criterion_main!(benches);
