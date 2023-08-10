use std::alloc::{Global, Allocator};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::ops::{Deref, Index, IndexMut};
use std::slice::SliceIndex;

use bitvec::vec::BitVec;
use bumpalo::Bump;
use smallvec::{Array, SmallVec};

#[derive(Clone, Copy, Debug)]
struct HfInternal {
    children: [usize; 2],
}

#[derive(Clone, Copy, Debug)]
struct HfLeaf {
    sym: u8,
}

#[derive(Clone, Debug)]
enum HfNodeKind {
    Internal(HfInternal),
    Leaf(HfLeaf),
}

#[derive(Clone, Debug)]
pub struct HfNode {
    parent: usize,
    kind: HfNodeKind,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
struct HeapData {
    freq: Reverse<u32>,
    arena: usize,
}

#[derive(Clone)]
pub struct Hf<'bump> {
    heap: BinaryHeap<HeapData, &'bump Bump>,
    arena: Vec<HfNode, &'bump Bump>,
    alloc: &'bump Bump,
}

impl<'bump> Hf<'bump> {
    pub fn new_in(alloc: &'bump Bump) -> Self {
        Self {
            heap: BinaryHeap::new_in(alloc),
            arena: Vec::new_in(alloc),
	    alloc
        }
    }

    /// Pushes a new symbol with the specified frequency.
    /// The returned symbol id is guaranteed to be sequential, meaning it always
    /// increases with one for each new symbol.
    pub fn push(&mut self, freq: u32) -> u8 {
        let nsym = self.arena.len().try_into().unwrap();
        self.heap.push(HeapData {
            arena: self.arena.len(),
            freq: Reverse(freq),
        });
        self.arena.push(HfNode {
            parent: 0,
            kind: HfNodeKind::Leaf(HfLeaf { sym: nsym }),
        });
        nsym
    }

    pub fn solve(mut self) -> HfTree<'bump> {
        let lfcnt = self.arena.len();

        let root = loop {
            let left = match self.heap.pop() {
                Some(left) => left,
                None => return HfTree::new(self.alloc, 0, self.arena, lfcnt), //self.arena.to_vec(), lfcnt)
            };

            let right = match self.heap.pop() {
                Some(right) => right,
                None => break left,
            };

            let out = self.arena.len();

            self.arena[left.arena].parent = out;
            self.arena[right.arena].parent = out;

            self.heap.push(HeapData {
                arena: out,
                freq: Reverse(left.freq.0.saturating_add(right.freq.0)),
            });

            let children = [left.arena, right.arena];

            self.arena.push(HfNode {
                parent: 0,
                kind: HfNodeKind::Internal(HfInternal { children }),
            });
        };

        HfTree::new(self.alloc, root.arena, self.arena, lfcnt)
    }
}

#[cfg(feature = "unstable")]
pub struct HfTree<'alloc> {
    root: usize,
    arena: Vec<HfNode, &'alloc Bump>,
    /// Maps symbols to huffman code
    symtc: Vec<BitVec, &'alloc Bump>,
}

#[cfg(not(all(feature = "unstable", feature = "bump")))]
compile_error! { "Stable HfTree is a work-in-progress" }

impl<'alloc> HfTree<'alloc> {
    pub fn new(alloc: &'alloc Bump, root: usize, arena: Vec<HfNode, &'alloc Bump>, lfcnt: usize) -> Self {
        debug_assert!(lfcnt <= arena.len());

        if lfcnt == 0 {
            return Self {
                root,
                arena,
                symtc: Vec::new_in(alloc),
            };
        }

        //let mut stack = SmallVec::<[(usize, BitVec); 256]>::new();
	let mut stack = Vec::new_in(alloc);
        stack.push((root, BitVec::new()));

        /* ------------------------------------------------------------------------- */
        // We use a symbol-to-huffman map because we need the leaf nodes
        // when encoding. A solution to finding a specific leaf node with a symbol
        // would be a map from symbol -> leaf node. We would then walk the tree backwards
        // to the root (we know which path to take based on increasing frequencies).
        //
        // But, if we already have to cache the leaf nodes, we might aswell cache their
        // huffman code instead, which is what we're doing here.
        /* ------------------------------------------------------------------------- */
	
        #[cfg(feature = "unsafe")]
        let mut symtc: Vec<BitVec, _> = {
            let mut symtc = Vec::with_capacity_in(lfcnt, alloc);
            unsafe {
                symtc.set_len(lfcnt);
                symtc
            }
        };

        #[cfg(not(feature = "unsafe"))]
        let mut symtc = {
	    let mut symtc = Vec::new_in(alloc);
	    symtc.extend(std::iter::repeat_with(BitVec::new).take(lfcnt));
	    symtc
	};

        while let Some((ndx, code)) = stack.pop() {
            let nd = unsafekit::arrayget!(arena, ndx);
            match &nd.kind {
                HfNodeKind::Internal(int) => {
                    let mut lc = code.clone();
                    let mut rc = code;

                    // Use `false, true` instead of `true, false` so we don't have to negate the bool in decoder
                    lc.push(false);
                    rc.push(true);

                    stack.push((int.children[0], lc));
                    stack.push((int.children[1], rc));
                }
                HfNodeKind::Leaf(leaf) => {
                    #[cfg(feature = "unsafe")]
                    unsafe {
                        let bsp: *mut BitVec = symtc.as_mut_ptr();
                        std::ptr::write(bsp.add(leaf.sym as usize), code);
                    }

		    #[cfg(not(feature = "unsafe"))]
                    {
                        symtc[leaf.sym as usize] = code;
                    }
                }
            }
        }

        Self { root, arena, symtc }
    }

    pub fn encoder<I: IntoIterator<Item = u8>>(&self, iter: I) -> HfEncoder<I::IntoIter> {
        HfEncoder {
            tree: self,
            input: iter.into_iter(),
        }
    }

    pub fn decoder<I: IntoIterator<Item = bool>>(&self, iter: I) -> HfDecoder<I::IntoIter> {
        HfDecoder {
            tree: self,
            input: iter.into_iter(),
        }
    }
}

pub struct HfEncoder<'a, I: Iterator<Item = u8>> {
    tree: &'a HfTree<'a>,
    input: I,
}

impl<'a, I: Iterator<Item = u8>> Iterator for HfEncoder<'a, I> {
    type Item = BitVec;

    fn next(&mut self) -> Option<Self::Item> {
        let sym = self.input.next()?;
        Some(self.tree.symtc[sym as usize].clone())
    }
}

pub struct HfDecoder<'a, I: Iterator<Item = bool>> {
    tree: &'a HfTree<'a>,
    input: I,
}

impl<'a, I: Iterator<Item = bool>> Iterator for HfDecoder<'a, I> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let mut int = self.tree.arena.get(self.tree.root)?;

        loop {
            match &int.kind {
                HfNodeKind::Internal(ch) => {
                    let path = self.input.next()? as usize;
                    int = &self.tree.arena[ch.children[path]];
                }
                HfNodeKind::Leaf(leaf) => return Some(leaf.sym),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::huffman::*;

    #[test]
    fn test_huffman() {
	let bump = Bump::new();
        let mut hf = Hf::new_in(&bump);
        let freqs = [10, 4, 1, 7, 18, 1, 5, 25];
        for freq in freqs.into_iter() {
            hf.push(freq);
        }

        let tree = hf.solve();

        let input = [0, 1, 2, 5];
        let encoded: Vec<_> = tree.encoder(input.iter().copied()).collect();
        let output: Vec<_> = tree.decoder(encoded.into_iter().flatten()).collect();

        assert_eq!(&input, output.as_slice());
    }
}
