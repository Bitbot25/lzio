use std::collections::VecDeque;

use bitvec::vec::BitVec;

const WIN_SIZE: usize = 32 * 1024;

pub struct Window {
    heap: Vec<u8>,
    begin: usize,
}

impl Window {
    pub fn data(&self) -> &[u8] {
	let slice = &self.heap[self.begin..];
	debug_assert!(slice.len() <= WIN_SIZE);
	slice
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Backref {
    offset: usize,
    length: usize,
}

/*

0 1 0 1 0 0
FIND
0 1 0

SLICE [0 1 0] CMP SOURCE [0] => MATCH 1
SLICE [0 1 0] CMP SOURCE [0 0] => MATCH 1
SLICE [0 1 0] CMP SOURCE [1 0 0] => MATCH 0
SLICE [0 1 0] CMP SOURCE [0 1 0 0] => MATCH 3 - BINGO! This is the maximum possible match

 */

fn strmatch(a: &[u8], b: &[u8]) -> usize {
    let mut n = 0;
    let mxlen = std::cmp::min(a.len(), b.len());
    while n < mxlen && a[n] == b[n] {
	n += 1;
    }
    n
}

// O(nm)
pub fn naive_substrmatch(search: &[u8], lookahead: &[u8]) -> Backref {
    let mut best = Backref { offset: 0, length: 0 };
    let mut offset = 0;
    let n = search.len();

    while offset < n {
	let slice = &search[n - offset..];
	let nmatch = strmatch(slice, lookahead);
	if nmatch > best.length {
	    best.offset = offset;
	    best.length = nmatch;
	}
	offset += 1;
    }
    best
}

fn lzmatch(search: &[u8], lookahead: &[u8]) -> Backref {
    naive_substrmatch(search, lookahead)
}

pub fn lz77(source: &[u8]) {
    let mut pivot = 0;
    let n = source.len();
    while pivot < n {
	let (window, lookahead) = source.split_at(pivot);
	//dbg!(lookahead[0]);
	let m: Backref = lzmatch(window, lookahead);
	let advance = if m.length != 0 {
	    // Insert a back reference
	    eprint!("[<- {}; {}]", m.offset, m.length);
	    m.length
	} else {
	    eprint!(",{}", lookahead[0]);
	    1
	};
	pivot += advance;
    }
    eprintln!();
}

#[cfg(test)]
mod tests {
    use crate::lz77;

    #[test]
    fn substrmatch() {
	let source = [0, 1, 1, 0, 0];

	assert_eq!(lz77::lzmatch(&source, &[0, 0]), lz77::Backref { offset: 2, length: 2 });
	assert_eq!(lz77::lzmatch(&source, &[1, 1, 0]), lz77::Backref { offset: 4, length: 3 });
    }
}
