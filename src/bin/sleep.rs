use std::{rc::Rc, time};

use mini_runtime::sleep;

/*
* 修改成宏方式后有两个问题：
* 1. 日志时间不再准确————通过Rc来解决（因为被提前释放了）
* 2. 最后的输出有问题————可以通过之后的WaitGroup来解决
*/
#[rt_entry::main]
async fn main() {
    let start_at = Rc::new(time::Instant::now());

    for i in 0..5 {
        spawn(a(
            format!("a-{}", i),
            time::Duration::from_secs(1),
            start_at.clone(),
        ));
    }

    spawn(b(start_at.clone()));
    spawn(c(start_at.clone()));

    log::info!("total cost {}ms", start_at.elapsed().as_millis());
}

async fn a(idx: impl AsRef<str>, delay: time::Duration, start_at: Rc<time::Instant>) {
    sleep(delay).await;
    log::info!(
        "sleep-{} done at {:?}ms",
        idx.as_ref(),
        start_at.elapsed().as_millis()
    );
}

async fn b(start_at: Rc<time::Instant>) {
    a("b-1", time::Duration::from_secs(1), start_at.clone()).await;
    a("b-2", time::Duration::from_secs(1), start_at.clone()).await;
}

async fn c(start_at: Rc<time::Instant>) {
    a("c-1", time::Duration::from_millis(1500), start_at.clone()).await;
}
