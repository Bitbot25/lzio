use bumpalo::Bump;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use smallvec::SmallVec;
use lzio::huffman::{Hf, HfTree, HfNode};

pub fn huffman_benchmark(c: &mut Criterion) {
    c.bench_function("huffman solve small", |b| {
	// let mut hf = Hf::<Vec<HfNode>>::new();
	let bump = Bump::new();
	let mut hf: Hf = Hf::new_in(&bump);
	let freqs = black_box([10, 4, 1, 7, 18, 1, 5, 25]);
	for freq in freqs {
	    hf.push(freq);
	}
	b.iter(|| hf.clone().solve());
    });
}

criterion_group!(benches, huffman_benchmark);
criterion_main!(benches);


