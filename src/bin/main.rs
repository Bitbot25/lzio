use bumpalo::Bump;
use lzio::huffman::*;

fn main() {
    let mut hf = Hf::new();
    let freqs = [10, 4, 1, 7, 18, 1, 5, 25];
    for freq in freqs.into_iter() {
        hf.push(freq);
    }

    for _ in 0..10_000_000 {
	std::hint::black_box(hf.clone().solve());
    }

    let tree = hf.solve();

    let input = [0, 1, 2, 5];
    let encoded: Vec<_> = tree.encoder(input.iter().copied()).collect();
    let output: Vec<_> = tree.decoder(encoded.into_iter().flatten()).collect();

    assert_eq!(&input, output.as_slice());
}
