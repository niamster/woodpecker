use logger::Record;

pub fn formatter(record: &Record) -> Box<String> {
    Box::new(format!(
        concat!("|{}| {} {}", wp_separator!(), "{}:{} {}\n"),
        record.level,
        record.ts_utc(),
        record.module,
        record.file,
        record.line,
        record.msg(),
    ))
}
