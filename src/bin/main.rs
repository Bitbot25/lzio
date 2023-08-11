use lzio::huffman::*;
use lzio::lz77;
use rand::Rng;

fn main() {
    let mut hf = Hf::new();
    
    let freqs = [10, 4, 1, 7, 18, 1, 5, 25];
    for freq in freqs.into_iter() {
        hf.push(freq);
    }
    let tree = hf.solve();

    lz77::lz77(&[20, 0, 20, 0]);

    /*let input = [0, 1, 2, 5];
    let encoded: Vec<_> = tree.encoder(input.iter().copied()).collect();
    let output: Vec<_> = tree.decoder(encoded.into_iter().flatten()).collect();

    assert_eq!(&input, output.as_slice());*/
}
