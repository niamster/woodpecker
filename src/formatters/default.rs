use logger::Record;

pub fn formatter(record: &Record) -> Box<String> {
    Box::new(format!(
        "|{}| {} {}:{}:{} {}\n",
        record.level,
        record.ts_utc(),
        record.module,
        record.file,
        record.line,
        record.msg(),
    ))
}
