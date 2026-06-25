<p align="center">
    <img src="./pmtree.png" width="150" alt="pmtree logo">
</p>

<p align="center">
    <img src="https://github.com/Rate-Limiting-Nullifier/pmtree/workflows/Build-Test-Fmt/badge.svg" width="140" alt="Build-Test-Fmt status">
</p>

<h1 align="center">pmtree</h1>

<h3 align="center">Persistent Merkle Tree (optimized & sparse & fixed-size) in Rust</h3>

## Overview

`pmtree` is a fixed-depth, sparse Merkle tree backed by a pluggable key/value store.
It is generic over two traits you provide:

- **`Database`** — the durable storage backend that persists the tree's nodes.
- **`Hasher`** — the hash function used to combine nodes.

Highlights:

- **Persistent & sparse** — only set nodes are stored; unset subtrees fall back to cached default hashes.
- **Atomic batch writes** — `batch_insert` / `batch_set` recompute in memory and commit through a single `put_batch`, so a crash mid-write cannot leave a partially updated tree.
- **Optional parallelism** — enable the `parallel` feature to recompute subtrees with [`rayon`].
- **Bounded depth** — supports depths up to 31 (the Cantor pairing of `(depth, index)` must fit in a `u64`).

## Install

```toml
[dependencies]
vacp2p_pmtree = { git = "https://github.com/vacp2p/pmtree" }
```

Enable parallel recomputation:

```toml
[dependencies]
vacp2p_pmtree = { git = "https://github.com/vacp2p/pmtree", features = ["parallel"] }
```

## Example

In-memory DB (`HashMap`) + Keccak hasher:

```rust
use std::collections::HashMap;

use tiny_keccak::{Hasher as _, Keccak};
use vacp2p_pmtree::*;

struct MemoryDB(HashMap<DBKey, Value>);
struct MyKeccak;

#[derive(Default)]
struct MemoryDBConfig;

impl Database for MemoryDB {
    type Config = MemoryDBConfig;

    fn new(_config: MemoryDBConfig) -> PmtreeResult<Self> {
        Ok(MemoryDB(HashMap::new()))
    }

    fn load(_config: MemoryDBConfig) -> PmtreeResult<Self> {
        Err(PmtreeError::Database("Cannot load database".to_string()))
    }

    fn get(&self, key: DBKey) -> PmtreeResult<Option<Value>> {
        Ok(self.0.get(&key).cloned())
    }

    fn put(&mut self, key: DBKey, value: Value) -> PmtreeResult<()> {
        self.0.insert(key, value);
        Ok(())
    }

    fn put_batch(&mut self, subtree: HashMap<DBKey, Value>) -> PmtreeResult<()> {
        self.0.extend(subtree);
        Ok(())
    }

    fn close(&mut self) -> PmtreeResult<()> {
        Ok(())
    }
}

impl Hasher for MyKeccak {
    type Fr = [u8; 32];

    fn serialize(value: Self::Fr) -> PmtreeResult<Value> {
        Ok(value.to_vec())
    }

    fn deserialize(bytes: &[u8]) -> PmtreeResult<Self::Fr> {
        Ok(bytes.try_into()?)
    }

    fn default_leaf() -> Self::Fr {
        [0; 32]
    }

    fn hash_pair(left: Self::Fr, right: Self::Fr) -> Self::Fr {
        let mut output = [0; 32];
        let mut hasher = Keccak::v256();
        hasher.update(&left);
        hasher.update(&right);
        hasher.finalize(&mut output);
        output
    }
}

fn main() -> PmtreeResult<()> {
    let mut mt = MerkleTree::<MemoryDB, MyKeccak>::new(2, MemoryDBConfig)?;

    assert_eq!(mt.capacity(), 4);
    assert_eq!(mt.depth(), 2);

    // Append leaves one at a time.
    mt.update_next([1; 32])?;

    // Or commit many at once, atomically.
    mt.batch_insert(None, &[[2; 32], [3; 32]])?;

    // Prove and verify a leaf.
    let proof = mt.proof(0)?;
    assert!(mt.verify(&mt.get(0)?, &proof));

    Ok(())
}
```
