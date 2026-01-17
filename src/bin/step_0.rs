use std::time;

use mini_runtime::{init_logger, run, sleep, spawn};

fn main() {
    init_logger(log::LevelFilter::Trace);
    let record = Record::new();
    spawn(a(&record));

    run();
    record.record("finish");
}

async fn a(record: &Record) {
    // 状态节点：init
    let tag = "Future_fa";
    record.record(tag);
    sleep(time::Duration::from_secs(1)).await; // 状态节点：state-1
    record.record(tag);
    let rf = ReadyFuture {};
    rf.await; // 状态节点：state-2
    record.record(tag);
    b(record).await; // 状态节点：state-3
    record.record(tag);
} // 状态节点：done

async fn b(record: &Record) {
    let tag = "Future_fb";
    record.record(tag);
    sleep(time::Duration::from_secs(2)).await;
    record.record(tag);
    sleep(time::Duration::from_secs(2)).await;
}

struct ReadyFuture {}

impl Future for ReadyFuture {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::task::Poll::Ready(())
    }
}

struct Record {
    start_at: time::Instant,
}

impl Record {
    fn new() -> Self {
        Self {
            start_at: time::Instant::now(),
        }
    }

    fn record(&self, tag: &str) {
        log::debug!("{} at {}ms", tag, self.start_at.elapsed().as_millis());
    }
}
