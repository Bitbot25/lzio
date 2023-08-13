use criterion::{Throughput, black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use lzio::{huffman::{Hf, HfTree, HfNode}, BitBuf};
use rand::Rng;
use std::io;

pub fn generate_huffman(sz: usize) -> Hf {
    let mut hf = Hf::new();

    let mut rng = rand::thread_rng();
    for _ in 0..sz {
        hf.push(rng.gen());
    }
    hf
}

pub fn random_bools(sz: usize) -> Vec<bool> {
    let mut v = Vec::with_capacity(sz);

    let mut rng = rand::thread_rng();
    for _ in 0..sz {
	v.push(rng.gen());
    }
    v
}

pub fn random_bools_four(sz: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(sz);

    let mut rng = rand::thread_rng();
    for _ in 0..sz {
	let b0 = rng.gen() as u8;
	let b1 = rng.gen() as u8;
	let b2 = rng.gen() as u8;
	let b3 = rng.gen() as u8;
	v.push((rng.gen(), rng.gen(), rng.gen(), rng.gen()));
    }
    v    
}

const KB: u64 = 1024;
const MB: u64 = 1024 * KB;

pub fn huffman_benchmark(c: &mut Criterion) {
    let mut solve_group = c.benchmark_group("huffman solve");

    // Going above 10*1024 amount of distinct symbols is not currently recommended.
    // The computation consumes ~2.2GiB of memory when 10*1024 symbols are used.
    for sz in [128, KB, 8 * KB] {
	solve_group.throughput(Throughput::Bytes(sz));

	let hf = generate_huffman(sz.try_into().unwrap());
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

struct NoOpWriter;

impl io::Write for NoOpWriter {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
	Ok(input.len())
    }

    fn flush(&mut self) -> io::Result<()> {
	Ok(())
    }
}

pub fn bitbuf_benchmark(c: &mut Criterion) {
    let mut put_group = c.benchmark_group("bitbuf put");
    for sz in [128, KB, 8 * KB, 64 * KB, MB, 8 * MB] {
	put_group.throughput(Throughput::Bytes(sz));

	let bools = random_bools(sz.try_into().unwrap());
	put_group.bench_with_input(BenchmarkId::from_parameter(sz), &bools, |bench, bools| {
	    bench.iter(|| {
		let mut buf = BitBuf::new(black_box(NoOpWriter));
		for b in bools {
		    buf.put(*b).expect("should never fail");
		}
		buf
	    })
	});
    }
    put_group.finish();

    let mut putcn_group = c.benchmark_group("bitbuf putcn");
    for sz in [128, KB, 8 * KB, 64 * KB, MB, 8 * MB] {
	putcn_group.throughput(Throughput::Bytes(sz));

	// sz / 4
	let bools = random_bools((sz << 2).try_into().unwrap());
	putcn_group.bench_with_input(BenchmarkId::from_parameter(sz), &bools, |bench, bools| {
	});
    }

    putcn_group.finish();
}

criterion_group!(benches, bitbuf_benchmark, huffman_benchmark);
criterion_main!(benches);


