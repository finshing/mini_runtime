#![allow(unused_must_use)]

use std::rc::Rc;

use mini_runtime::{
    dns::dns_parse,
    sync::wait_group::{WaitGroup, WaitGroupGuard},
    variable_log,
};

#[rt_entry::main]
async fn main() {
    log::info!("dns parse");
    let wg = Rc::new(WaitGroup::new());

    let lookup = async |domain: &str, _guard: WaitGroupGuard<'_>| {
        variable_log!(info @ dns_parse(domain, 80).await);
    };

    for domain in [
        "localhost",
        "ark.cn-beijing.volces.com",
        "www.zhihu.com",
        "www.jianshu.com",
        "www.bilibili.com",
        // "pre17-es.xiaojukeji.com",
    ] {
        spawn!(lookup(domain, wg.add()));
    }

    wg.wait().await;
    log::info!("all domain dns parsed");
}
