use criterion::{Throughput, black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use lzio::huffman::{Hf, HfTree, HfNode};
use rand::Rng;

pub fn generate_huffman(sz: usize) -> Hf {
    let mut hf = Hf::new();

    let mut rng = rand::thread_rng();
    for _ in 0..sz {
        hf.push(rng.gen());
    }
    hf
}

pub fn huffman_benchmark(c: &mut Criterion) {
    const KB: usize = 1024;
    const MB: usize = 1024 * KB;
    
    let mut solve_group = c.benchmark_group("huffman solve");
    for sz in [128, KB, 8 * KB, 64 * KB, MB] {
	solve_group.throughput(Throughput::Bytes(sz as u64));

	let hf = generate_huffman(sz);
	solve_group.bench_with_input(BenchmarkId::from_parameter(sz), &hf, |b, hf| {
	    b.iter(|| {
		let new_hf = hf.clone();
		let res = new_hf.solve();
		res
	    })
	});
	drop(hf);
    }

    solve_group.finish();
}

criterion_group!(benches, huffman_benchmark);
criterion_main!(benches);


