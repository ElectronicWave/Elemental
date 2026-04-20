pub fn classpath_separator() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}

pub fn join_classpath(entries: Vec<String>) -> String {
    entries.join(classpath_separator())
}
