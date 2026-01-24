use mini_runtime::result::{ErrorType, Result};

#[rt_entry::main]
async fn main() -> Result<()> {
    a().await
}

async fn a() -> Result<()> {
    Err(ErrorType::Eof.into())
}
