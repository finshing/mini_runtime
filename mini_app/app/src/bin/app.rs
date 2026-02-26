use core::time;

use app::app::create_app;
use common::{config, result::HttpResult};

#[rt_entry::main]
async fn main() -> HttpResult<()> {
    let mut app = create_app(
        config::HTTP_SERVER_IP,
        config::HTTP_SERVER_PORT,
        time::Duration::from_secs(2),
    )?;
    app.set_max_wait_time(time::Duration::from_secs(10))
        .run()
        .await?;

    Ok(())
}
