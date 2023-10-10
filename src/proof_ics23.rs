use ics23::{ExistenceProof, HashOp, InnerOp, InnerSpec, LeafOp, LengthOp, ProofSpec};

use crate::collections::VecDeque;
use crate::error::{Error, Result};
use crate::{Key, MerkleProof, H256, TREE_HEIGHT, traits::Value};

pub fn convert<K, V, const N: usize>(
    merkle_proof: MerkleProof,
    key: &K,
    value: &V,
    hash_op: HashOp,
) -> Result<ExistenceProof>
where
    K: Key<N>,
    V: Value,
{
    let (leaves_path, proof) = merkle_proof.take();
    let mut merge_heights: VecDeque<_> = leaves_path
        .get(0)
        .expect("The heights should exist")
        .clone()
        .into();
    let mut proof: VecDeque<_> = proof.into();
    let mut cur_key = **key;
    let mut height = 0;
    let mut path = Vec::new();
    while !proof.is_empty() {
        if height == TREE_HEIGHT {
            if !proof.is_empty() {
                return Err(Error::CorruptedProof);
            }
            break;
        }

        // check the height is valid
        let merge_height = merge_heights.front().map(|h| *h as usize).unwrap_or(height);
        if height != merge_height {
            // skip the heights
            height = merge_height;
            continue;
        }

        // get a proof
        let (sibling, sibling_height) = proof.pop_front().expect("no proof");
        if height < sibling_height as usize {
            // skip heights
            height = sibling_height as usize;
        }
        let inner_op = get_inner_op(hash_op, &sibling, cur_key.get_bit(height));
        path.push(inner_op);

        merge_heights.pop_front();
        cur_key = cur_key.parent_path(height);
        height += 1;
    }

    Ok(ExistenceProof {
        key: key.to_vec(),
        value: value.as_slice().to_vec(),
        leaf: Some(get_leaf_op(hash_op)),
        path,
    })
}

pub fn get_spec(hash_op: HashOp) -> ProofSpec {
    ProofSpec {
        leaf_spec: Some(get_leaf_op(hash_op)),
        inner_spec: Some(get_inner_spec(hash_op)),
        max_depth: TREE_HEIGHT as i32,
        min_depth: 0,
        prehash_key_before_comparison: false,
    }
}

fn get_leaf_op(hash_op: HashOp) -> LeafOp {
    LeafOp {
        hash: hash_op.into(),
        prehash_key: HashOp::NoHash.into(),
        prehash_value: HashOp::NoHash.into(),
        length: LengthOp::NoPrefix.into(),
        prefix: H256::zero().as_slice().to_vec(),
    }
}

fn get_inner_op(hash_op: HashOp, sibling: &H256, is_right_node: bool) -> InnerOp {
    let node = sibling.as_slice().to_vec();
    let (prefix, suffix) = if is_right_node {
        (node, vec![])
    } else {
        (vec![], node)
    };
    InnerOp {
        hash: hash_op.into(),
        prefix,
        suffix,
    }
}

fn get_inner_spec(hash_op: HashOp) -> InnerSpec {
    InnerSpec {
        child_order: vec![0, 1],
        child_size: 32,
        min_prefix_length: 0,
        max_prefix_length: 32,
        empty_child: vec![],
        hash: hash_op.into(),
    }
}
