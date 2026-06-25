use std::{
    cmp::{max, min},
    collections::{hash_map::Entry, HashMap},
    sync::{Arc, RwLock},
};

#[cfg(feature = "parallel")]
use rayon;

use crate::*;

// db[DEPTH_KEY] = depth
const DEPTH_KEY: DBKey = (u64::MAX - 1).to_be_bytes();

// db[NEXT_INDEX_KEY] = next_index;
const NEXT_INDEX_KEY: DBKey = u64::MAX.to_be_bytes();

// The Cantor pairing in `From<Key>` is computed in u64; it stays within u64 only while the largest
// node key `s = depth + (2^depth - 1)` keeps `s * (s + 1)` below u64::MAX (which is true for `depth <= 31`).
const MAX_DEPTH: usize = 31;

// Denotes keys (depth, index) in Merkle Tree. Can be converted to DBKey
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(usize, usize);
impl From<Key> for DBKey {
    fn from(key: Key) -> Self {
        let cantor_pairing = ((key.0 + key.1) * (key.0 + key.1 + 1) / 2 + key.1) as u64;
        cantor_pairing.to_be_bytes()
    }
}

impl Key {
    pub fn new(depth: usize, index: usize) -> Self {
        Key(depth, index)
    }
}

/// The Merkle Tree structure.
pub struct MerkleTree<D, H>
where
    D: Database,
    H: Hasher,
{
    pub db: D,
    depth: usize,
    next_index: usize,
    cache: Vec<H::Fr>,
    root: H::Fr,
}

/// The Merkle Proof structure.
#[derive(Clone, PartialEq, Eq)]
pub struct MerkleProof<H: Hasher>(pub Vec<(H::Fr, u8)>);

impl<D, H> MerkleTree<D, H>
where
    D: Database,
    H: Hasher,
{
    /// Creates a [`MerkleTree`] of the given `depth` with the default [`Database`] config.
    pub fn default(depth: usize) -> PmtreeResult<Self> {
        Self::new(depth, D::Config::default())
    }

    /// Creates a new [`MerkleTree`] and stores it in the [`Database`] given by `db_config`.
    pub fn new(depth: usize, db_config: D::Config) -> PmtreeResult<Self> {
        // Rejects depths that overflow Cantor pairing or exceed `1 << depth` capacity.
        if depth > MAX_DEPTH {
            return Err(PmtreeError::DepthTooLarge(depth));
        }

        // Create new db instance
        let mut db = D::new(db_config)?;

        // Insert depth val into db
        let depth_val = depth.to_be_bytes().to_vec();
        db.put(DEPTH_KEY, depth_val)?;

        // Insert next_index val into db
        let next_index = 0usize;
        let next_index_val = next_index.to_be_bytes().to_vec();
        db.put(NEXT_INDEX_KEY, next_index_val)?;

        // Cache nodes
        let mut cache = vec![H::default_leaf(); depth + 1];

        // Initialize one branch of the `Merkle Tree` from bottom to top
        cache[depth] = H::default_leaf();
        db.put(Key(depth, 0).into(), H::serialize(cache[depth])?)?;
        for i in (0..depth).rev() {
            cache[i] = H::hash_pair(cache[i + 1], cache[i + 1]);
            db.put(Key(i, 0).into(), H::serialize(cache[i])?)?;
        }

        let root = cache[0];

        Ok(Self {
            db,
            depth,
            next_index,
            cache,
            root,
        })
    }

    /// Loads an existing [`MerkleTree`] from the [`Database`] given by `db_config`.
    pub fn load(db_config: D::Config) -> PmtreeResult<Self> {
        // Load existing db instance
        let db = D::load(db_config)?;

        // Load depth & next_index, missing one means the tree is corrupted.
        let depth = match db.get(DEPTH_KEY)? {
            Some(depth) => usize::from_be_bytes(depth.as_slice().try_into()?),
            None => return Err(PmtreeError::Corrupted),
        };

        // Rejects depths that overflow Cantor pairing or exceed `1 << depth` capacity.
        if depth > MAX_DEPTH {
            return Err(PmtreeError::DepthTooLarge(depth));
        }

        let next_index = match db.get(NEXT_INDEX_KEY)? {
            Some(next_index) => usize::from_be_bytes(next_index.as_slice().try_into()?),
            None => return Err(PmtreeError::Corrupted),
        };

        // Rebuild the empty-subtree default hash for each level.
        let mut cache = vec![H::default_leaf(); depth + 1];
        for i in (0..depth).rev() {
            cache[i] = H::hash_pair(cache[i + 1], cache[i + 1]);
        }

        // Load root; an absent root means the (empty) default root `cache[0]`.
        let root = match db.get(Key(0, 0).into())? {
            Some(root) => H::deserialize(&root)?,
            None => cache[0],
        };

        Ok(Self {
            db,
            depth,
            next_index,
            cache,
            root,
        })
    }

    /// Closes the [`Database`] connection.
    pub fn close(&mut self) -> PmtreeResult<()> {
        self.db.close()
    }

    /// Returns the depth of the tree.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Returns the capacity of the tree, i.e. the maximum number of leaves.
    pub fn capacity(&self) -> usize {
        1 << self.depth
    }

    /// Returns the total number of leaves set (`next_index`).
    pub fn leaves_set(&self) -> usize {
        self.next_index
    }

    /// Returns the root of the tree.
    pub fn root(&self) -> H::Fr {
        self.root
    }

    /// Returns the leaf at `key`.
    pub fn get(&self, key: usize) -> PmtreeResult<H::Fr> {
        if key >= self.capacity() {
            return Err(PmtreeError::IndexOutOfBounds);
        }

        self.get_elem(Key(self.depth, key))
    }

    /// Returns the subtree root at `level` on the path to leaf `index`
    /// (`level == 0` is the tree root, `level == depth` is the leaf itself).
    pub fn subtree_root(&self, level: usize, index: usize) -> PmtreeResult<H::Fr> {
        if level > self.depth || index >= self.capacity() {
            return Err(PmtreeError::IndexOutOfBounds);
        }
        if level == 0 {
            Ok(self.root)
        } else if level == self.depth {
            self.get_elem(Key(self.depth, index))
        } else {
            self.get_elem(Key(level, index >> (self.depth - level)))
        }
    }

    /// Computes a [`MerkleProof`] for the leaf at `index`.
    pub fn proof(&self, index: usize) -> PmtreeResult<MerkleProof<H>> {
        if index >= self.capacity() {
            return Err(PmtreeError::IndexOutOfBounds);
        }

        let mut witness = Vec::with_capacity(self.depth);

        let mut i = index;
        let mut depth = self.depth;
        while depth != 0 {
            i ^= 1;
            witness.push((self.get_elem(Key(depth, i))?, (1 - (i & 1)) as u8));
            i >>= 1;
            depth -= 1;
        }

        Ok(MerkleProof(witness))
    }

    /// Verifies a [`MerkleProof`] against `leaf` and the current tree root.
    pub fn verify(&self, leaf: &H::Fr, witness: &MerkleProof<H>) -> bool {
        let expected_root = witness.compute_root_from(leaf);

        self.root() == expected_root
    }

    /// Sets the leaf at index `key`.
    pub fn set(&mut self, key: usize, leaf: H::Fr) -> PmtreeResult<()> {
        if key >= self.capacity() {
            return Err(PmtreeError::IndexOutOfBounds);
        }

        // A single set is a one-element batch, so it commits atomically through `batch_insert`.
        self.batch_insert(Some(key), &[leaf])
    }

    /// Inserts `leaf` at the next available index.
    pub fn update_next(&mut self, leaf: H::Fr) -> PmtreeResult<()> {
        self.set(self.next_index, leaf)?;

        Ok(())
    }

    /// Deletes the leaf at `key` by resetting it to [`Hasher::default_leaf`].
    pub fn delete(&mut self, key: usize) -> PmtreeResult<()> {
        if key >= self.next_index {
            return Err(PmtreeError::IndexOutOfBounds);
        }

        self.set(key, H::default_leaf())?;

        Ok(())
    }

    /// Sets `leaves` contiguously from `start` via [`MerkleTree::batch_insert`].
    pub fn set_range<I: IntoIterator<Item = H::Fr>>(
        &mut self,
        start: usize,
        leaves: I,
    ) -> PmtreeResult<()> {
        self.batch_insert(
            Some(start),
            leaves.into_iter().collect::<Vec<_>>().as_slice(),
        )
    }

    /// Batch insertion of contiguous leaves from `start`, updated in parallel and committed atomically.
    pub fn batch_insert(&mut self, start: Option<usize>, leaves: &[H::Fr]) -> PmtreeResult<()> {
        if leaves.is_empty() {
            return Ok(());
        }

        let start = start.unwrap_or(self.next_index);
        let end = start + leaves.len();

        if end > self.capacity() {
            return Err(PmtreeError::TreeIsFull);
        }

        let root_key = Key(0, 0);
        let mut subtree = HashMap::<Key, H::Fr>::new();
        subtree.insert(root_key, self.root);
        self.fill_nodes(root_key, start, end, &mut subtree, leaves, start)?;

        self.commit_subtree(subtree, max(self.next_index, end))
    }

    /// Sets a batch of leaves at arbitrary, possibly non-contiguous, indices committed atomically.
    pub fn batch_set(&mut self, pairs: &[(usize, H::Fr)]) -> PmtreeResult<()> {
        if pairs.is_empty() {
            return Ok(());
        }
        let mut max_end = 0;
        for &(index, _) in pairs {
            if index >= self.capacity() {
                return Err(PmtreeError::IndexOutOfBounds);
            }
            max_end = max(max_end, index + 1);
        }

        let root_key = Key(0, 0);
        let mut subtree = HashMap::<Key, H::Fr>::new();
        subtree.insert(root_key, self.root);

        // For each affected leaf, load every node on its path plus the off-path sibling, so
        // `batch_recalculate` can recompute the union of touched paths; then overwrite the leaf.
        for &(index, value) in pairs {
            let mut key = root_key;
            for level in 0..self.depth {
                let left = Key(level + 1, key.1 * 2);
                let right = Key(level + 1, key.1 * 2 + 1);
                if let Entry::Vacant(slot) = subtree.entry(left) {
                    slot.insert(self.get_elem(left)?);
                }
                if let Entry::Vacant(slot) = subtree.entry(right) {
                    slot.insert(self.get_elem(right)?);
                }
                let goes_right = (index >> (self.depth - level - 1)) & 1 == 1;
                key = if goes_right { right } else { left };
            }
            subtree.insert(Key(self.depth, index), value);
        }

        self.commit_subtree(subtree, max(self.next_index, max_end))
    }

    // Returns the node at `key`, falling back to the empty-subtree default for an unwritten node.
    fn get_elem(&self, key: Key) -> PmtreeResult<H::Fr> {
        let res = match self.db.get(key.into())? {
            Some(value) => H::deserialize(&value)?,
            None => self.cache[key.0],
        };

        Ok(res)
    }

    // Recomputes the subtree root in memory, then commits every changed node and next_index through a
    // single atomic put_batch (so a crash mid-write cannot leave the tree partially updated)
    fn commit_subtree(
        &mut self,
        subtree: HashMap<Key, H::Fr>,
        new_next_index: usize,
    ) -> PmtreeResult<()> {
        let root_key = Key(0, 0);
        let subtree = Arc::new(RwLock::new(subtree));
        let root_val = Self::batch_recalculate(root_key, Arc::clone(&subtree), self.depth)?;

        let mut batch = subtree
            .read()?
            .iter()
            .map(|(key, value)| Ok(((*key).into(), H::serialize(*value)?)))
            .collect::<PmtreeResult<HashMap<DBKey, Value>>>()?;

        if new_next_index != self.next_index {
            batch.insert(NEXT_INDEX_KEY, new_next_index.to_be_bytes().to_vec());
        }

        self.db.put_batch(batch)?;

        self.next_index = new_next_index;
        self.root = root_val;

        Ok(())
    }

    // Fills hashmap subtree
    fn fill_nodes(
        &self,
        key: Key,
        start: usize,
        end: usize,
        subtree: &mut HashMap<Key, H::Fr>,
        leaves: &[H::Fr],
        from: usize,
    ) -> PmtreeResult<()> {
        if key.0 == self.depth {
            if key.1 >= from {
                subtree.insert(key, leaves[key.1 - from]);
            }
            return Ok(());
        }

        let left = Key(key.0 + 1, key.1 * 2);
        let right = Key(key.0 + 1, key.1 * 2 + 1);

        let left_val = self.get_elem(left)?;
        let right_val = self.get_elem(right)?;

        subtree.insert(left, left_val);
        subtree.insert(right, right_val);

        let half = 1 << (self.depth - key.0 - 1);

        if start < half {
            self.fill_nodes(left, start, min(end, half), subtree, leaves, from)?;
        }

        if end > half {
            self.fill_nodes(right, 0, end - half, subtree, leaves, from)?;
        }

        Ok(())
    }

    // Recalculates tree in parallel (in-memory)
    fn batch_recalculate(
        key: Key,
        subtree: Arc<RwLock<HashMap<Key, H::Fr>>>,
        depth: usize,
    ) -> PmtreeResult<H::Fr> {
        let left_child = Key(key.0 + 1, key.1 * 2);
        let right_child = Key(key.0 + 1, key.1 * 2 + 1);

        let is_leaf = key.0 == depth || !subtree.read()?.contains_key(&left_child);
        if is_leaf {
            return subtree
                .read()?
                .get(&key)
                .copied()
                .ok_or(PmtreeError::Corrupted);
        }

        #[cfg(feature = "parallel")]
        let (left, right) = rayon::join(
            || Self::batch_recalculate(left_child, Arc::clone(&subtree), depth),
            || Self::batch_recalculate(right_child, Arc::clone(&subtree), depth),
        );

        #[cfg(not(feature = "parallel"))]
        let (left, right) = (
            Self::batch_recalculate(left_child, Arc::clone(&subtree), depth),
            Self::batch_recalculate(right_child, Arc::clone(&subtree), depth),
        );

        let result = H::hash_pair(left?, right?);

        subtree.write()?.insert(key, result);

        Ok(result)
    }
}

impl<H: Hasher> MerkleProof<H> {
    /// Computes the Merkle root by hashing `leaf` up through the [`MerkleProof`].
    pub fn compute_root_from(&self, leaf: &H::Fr) -> H::Fr {
        let mut acc = *leaf;
        for w in self.0.iter() {
            if w.1 == 0 {
                acc = H::hash_pair(acc, w.0);
            } else {
                acc = H::hash_pair(w.0, acc);
            }
        }

        acc
    }

    /// Computes the leaf index this [`MerkleProof`] corresponds to.
    pub fn leaf_index(&self) -> usize {
        self.get_path_index()
            .into_iter()
            .rev()
            .fold(0, |acc, digit| (acc << 1) + usize::from(digit))
    }

    /// Returns the path indices forming the [`MerkleProof`].
    pub fn get_path_index(&self) -> Vec<u8> {
        self.0.iter().map(|x| x.1).collect()
    }

    /// Returns the path elements forming the [`MerkleProof`].
    pub fn get_path_elements(&self) -> Vec<H::Fr> {
        self.0.iter().map(|x| x.0).collect()
    }

    /// Returns the length of the [`MerkleProof`].
    pub fn length(&self) -> usize {
        self.0.len()
    }
}
