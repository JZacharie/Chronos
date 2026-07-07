use std::sync::Mutex;
use std::sync::OnceLock;

static LOGS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

pub fn get_logs() -> &'static Mutex<Vec<String>> {
    LOGS.get_or_init(|| Mutex::new(Vec::new()))
}

pub struct LogWriter;

impl std::io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(s) = std::str::from_utf8(buf) {
            if let Ok(mut logs) = get_logs().lock() {
                logs.push(s.to_string());
                if logs.len() > 1000 {
                    logs.remove(0);
                }
            }
        }
        std::io::stdout().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}
