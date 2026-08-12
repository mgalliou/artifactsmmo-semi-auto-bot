use log::{Level, LevelFilter, Log, Metadata, Record, SetLoggerError};
use ratatui::{
    prelude::{Buffer, Rect},
    widgets::{Block, List, Widget},
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

const LOG_CAPACITY: usize = 1_000;

#[derive(Clone)]
pub struct LogBuffer {
    entries: Arc<Mutex<VecDeque<String>>>,
    capacity: usize,
}

impl LogBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            capacity,
        }
    }

    fn entries_lock(&self) -> MutexGuard<'_, VecDeque<String>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Log for LogBuffer {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let entry = format!("[{}] {}", record.level(), record.args());
        let mut entries = self.entries_lock();
        if entries.len() == self.capacity {
            entries.pop_front();
        }
        entries.push_back(entry);
    }

    fn flush(&self) {
        // Entries are written synchronously; taking the lock waits for an active write to finish.
        drop(self.entries_lock());
    }
}

pub fn init_logger() -> Result<LogBuffer, SetLoggerError> {
    let logs = LogBuffer::new(LOG_CAPACITY);
    log::set_boxed_logger(Box::new(logs.clone()))?;
    log::set_max_level(LevelFilter::Info);
    Ok(logs)
}

pub struct LogWidget {
    logs: LogBuffer,
}

impl LogWidget {
    #[must_use]
    pub const fn new(logs: LogBuffer) -> Self {
        Self { logs }
    }
}

impl Widget for LogWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let entries = self.logs.entries_lock();
        let block = Block::bordered().title(format!("Log ({})", entries.len()));
        let inner = block.inner(area);
        block.render(area, buf);

        let visible = usize::from(inner.height);
        let skip = entries.len().saturating_sub(visible);
        List::new(entries.iter().skip(skip).map(String::as_str)).render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::LogBuffer;
    use log::{Level, Log, Record};

    #[test]
    fn discards_oldest_entries_at_capacity() {
        let logs = LogBuffer::new(2);
        for message in ["first", "second", "third"] {
            logs.log(
                &Record::builder()
                    .level(Level::Info)
                    .args(format_args!("{message}"))
                    .build(),
            );
        }

        assert_eq!(
            logs.entries_lock().iter().cloned().collect::<Vec<_>>(),
            ["[INFO] second", "[INFO] third"]
        );
    }
}
