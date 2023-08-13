use bitvec::vec::BitVec;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::mem;

#[derive(Clone, Copy, Debug)]
struct HfInternal {
    children: [usize; 2],
}

#[derive(Clone, Copy, Debug)]
struct HfLeaf {
    sym: u32,
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
pub struct Hf {
    heap: BinaryHeap<HeapData>,
    arena: Vec<HfNode>,
}

impl Hf {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            arena: Vec::new(),
        }
    }

    /// Pushes a new symbol with the specified frequency.
    /// The returned symbol id is guaranteed to be sequential, meaning it always
    /// increases with one for each new symbol.
    pub fn push(&mut self, freq: u32) -> u32 {
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

    pub fn estimate_memory(syms: usize) -> usize {
	// Heap size will never be larger
	let max_heapsz = syms;
	let mut heapsz = max_heapsz;

	let mut arenasz = syms;

	loop {
	    if heapsz < 2 {
		break;
	    }

	    heapsz -= 1; // -2 + 1
	    arenasz += 1;
	}

	arenasz * mem::size_of::<HfNode>()
	    + max_heapsz * mem::size_of::<HeapData>()
	    + HfTree::estimate_memory(arenasz, syms)
    }

    pub fn solve(mut self) -> HfTree {
        let lfcnt = self.arena.len();

        let root = loop {
            let left = match self.heap.pop() {
                Some(left) => left,
                None => return HfTree::new(0, self.arena, lfcnt),
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

        HfTree::new(root.arena, self.arena, lfcnt)
    }
}

pub struct HfTree {
    root: usize,
    arena: Vec<HfNode>,
    /// Maps symbols to huffman code
    symtc: Vec<BitVec>,
}

impl HfTree {
    pub fn estimate_memory(nodecnt: usize, syms: usize) -> usize {
	let mut stacklen = 1;
	let mut internal_cnt = nodecnt - syms;

	while stacklen != 0 {
	    stacklen -= 1;

	    if internal_cnt > 0 {
		internal_cnt -= 1;
		stacklen += 2;
	    }
	}

	// size of symtc
	syms * mem::size_of::<BitVec>() * (syms - 1)
	    // size of intermediary stack
	    + stacklen * mem::size_of::<(usize, BitVec)>()
    }
    
    pub fn new(root: usize, arena: Vec<HfNode>, lfcnt: usize) -> Self {
        debug_assert!(lfcnt <= arena.len());

        if lfcnt == 0 {
            return Self {
                root,
                arena,
                symtc: Vec::new(),
            };
        }

        let mut stack = Vec::new();
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
        let mut symtc: Vec<BitVec> = {
            let mut symtc = Vec::with_capacity(lfcnt);
            unsafe {
                symtc.set_len(lfcnt);
                symtc
            }
        };

        #[cfg(not(feature = "unsafe"))]
        let mut symtc = {
            let mut symtc = Vec::with_capacity(lfcnt);
            symtc.extend(std::iter::repeat_with(BitVec::new).take(lfcnt));
            symtc
        };

        while let Some((ndx, code)) = stack.pop() {
            let nd = &arena[ndx];
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

    pub fn encoder<I: IntoIterator<Item = u32>>(&self, iter: I) -> HfEncoder<I::IntoIter> {
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

pub struct HfEncoder<'a, I: Iterator<Item = u32>> {
    tree: &'a HfTree,
    input: I,
}

impl<'a, I: Iterator<Item = u32>> Iterator for HfEncoder<'a, I> {
    type Item = BitVec;

    fn next(&mut self) -> Option<Self::Item> {
        let sym = self.input.next()?;
        Some(self.tree.symtc[sym as usize].clone())
    }
}

pub struct HfDecoder<'a, I: Iterator<Item = bool>> {
    tree: &'a HfTree,
    input: I,
}

impl<'a, I: Iterator<Item = bool>> Iterator for HfDecoder<'a, I> {
    type Item = u32;

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
        let mut hf = Hf::new();
        let freqs = [10, 4, 1, 7, 18, 1, 5, 25];
        for freq in freqs.into_iter() {
            hf.push(freq);
        }

        let tree = hf.solve();

        let input = [0u32, 1, 2, 5];
        let encoded: Vec<_> = tree.encoder(input.iter().copied()).collect();
        let output: Vec<_> = tree.decoder(encoded.into_iter().flatten()).collect();

        assert_eq!(&input, output.as_slice());
    }
}
