#![allow(unstable_name_collisions)]
use criterion::*;
use cser::*;

fn bench(c: &mut Criterion) {
    for bits in 1..=9 {
        c.bench_function(&format!("{bits} bits"), |b| {
            b.iter(|| {
                const N: usize = 10_000;

                let mut writer = BitsWriter::new(Vec::with_capacity((bits * N).div_ceil(8)));
                for _ in 0..N {
                    writer.write(bits, 0xff);
                }

                let mut reader = BitsReader::new(writer.view_bytes());
                for _ in 0..N {
                    reader.read(bits);
                }
            })
        });
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
