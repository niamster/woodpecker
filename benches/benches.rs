#[macro_use]
extern crate bencher;

#[macro_use]
extern crate woodpecker;
use woodpecker as wp;

use bencher::Bencher;

fn bench_no_output_single_thread(b: &mut Bencher) {
    b.iter(|| {
        debug!("{}", "test");
    });
}

fn bench_output_drop_single_thread(b: &mut Bencher) {
    wp::logger::reset();
    handler!(Box::new(|_| {}));
    b.iter(|| {
        critical!("{}", "test");
    });
}

fn bench_stub(_: &mut Bencher) {
}

benchmark_group!(
    benches,
    bench_no_output_single_thread,
    bench_output_drop_single_thread,

    bench_stub
);
benchmark_main!(benches);
