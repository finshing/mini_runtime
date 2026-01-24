use mini_runtime::result::{ErrorType, Result};

#[rt_entry::main]
async fn main() -> Result<()> {
    a().await?;
    Ok(())
}

async fn a() -> Result<()> {
    Err(ErrorType::Eof.into())
}
