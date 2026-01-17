use std::time;

use mini_runtime::{run, sleep, spawn};

fn main() {
    let start_at = time::Instant::now();

    for i in 0..5 {
        spawn(a(
            format!("a-{}", i),
            time::Duration::from_secs(1),
            &start_at,
        ));
    }

    spawn(b(&start_at));
    spawn(c(&start_at));

    run();

    println!("total cost {}ms", start_at.elapsed().as_millis());
}

async fn a(idx: impl AsRef<str>, delay: time::Duration, start_at: &time::Instant) {
    sleep(delay).await;
    println!(
        "sleep-{} done at {:?}ms",
        idx.as_ref(),
        start_at.elapsed().as_millis()
    );
}

async fn b(start_at: &time::Instant) {
    a("b-1", time::Duration::from_secs(1), start_at).await;
    a("b-2", time::Duration::from_secs(1), start_at).await;
}

async fn c(start_at: &time::Instant) {
    a("c-1", time::Duration::from_millis(1500), start_at).await;
}
