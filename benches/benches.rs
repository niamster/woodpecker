#[macro_use]
extern crate bencher;

#[macro_use]
extern crate woodpecker;

mod wpb {
    use bencher::Bencher;
    use woodpecker as wp;

    fn bench_no_output_single_thread(b: &mut Bencher) {
        wp::logger::reset();
        level!([wp::LogLevel::ERROR]);
        b.iter(|| {
            debug!("{}", "test");
        });
    }

    fn bench_no_output_sub_this_module_single_thread(b: &mut Bencher) {
        wp::logger::reset();
        level!([wp::LogLevel::ERROR]);
        level!(module_path!(), [wp::LogLevel::INFO]);
        b.iter(|| {
            debug!("{}", "test");
        });
    }

    fn bench_no_output_sub_this_file_single_thread(b: &mut Bencher) {
        wp::logger::reset();
        level!([wp::LogLevel::ERROR]);
        level!(format!("{}::{}", module_path!(), file!()), [wp::LogLevel::INFO]);
        b.iter(|| {
            debug!("{}", "test");
        });
    }

    fn bench_no_output_sub_other_single_thread(b: &mut Bencher) {
        wp::logger::reset();
        level!([wp::LogLevel::ERROR]);
        level!("foo", [wp::LogLevel::INFO]);
        b.iter(|| {
            debug!("{}", "test");
        });
    }

    fn bench_output_drop_single_thread(b: &mut Bencher) {
        wp::logger::reset();
        level!([wp::LogLevel::ERROR]);
        handler!(Box::new(|_| {}));
        b.iter(|| {
            critical!("{}", "test");
        });
    }

    fn bench_output_drop_sub_this_module_single_thread(b: &mut Bencher) {
        wp::logger::reset();
        level!([wp::LogLevel::ERROR]);
        handler!(Box::new(|_| {}));
        level!(module_path!(), [wp::LogLevel::INFO]);
        b.iter(|| {
            critical!("{}", "test");
        });
    }

    fn bench_output_drop_sub_this_file_single_thread(b: &mut Bencher) {
        wp::logger::reset();
        level!([wp::LogLevel::ERROR]);
        handler!(Box::new(|_| {}));
        level!(format!("{}::{}", module_path!(), file!()), [wp::LogLevel::INFO]);
        b.iter(|| {
            critical!("{}", "test");
        });
    }

    fn bench_output_drop_sub_other_single_thread(b: &mut Bencher) {
        wp::logger::reset();
        level!([wp::LogLevel::ERROR]);
        handler!(Box::new(|_| {}));
        level!("foo", [wp::LogLevel::INFO]);
        b.iter(|| {
            critical!("{}", "test");
        });
    }

    fn bench_stub(_: &mut Bencher) {
    }

    benchmark_group!(
        benches,

        bench_no_output_single_thread,
        bench_no_output_sub_this_file_single_thread,
        bench_no_output_sub_this_module_single_thread,
        bench_no_output_sub_other_single_thread,

        bench_output_drop_single_thread,
        bench_output_drop_sub_this_module_single_thread,
        bench_output_drop_sub_this_file_single_thread,
        bench_output_drop_sub_other_single_thread,

        bench_stub
    );
}
benchmark_main!(wpb::benches);
