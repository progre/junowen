mod cui;
mod tracing_helper;

use anyhow::Result;
use junowen_lib::lang::Lang;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_helper::init_tracing("./", concat!(env!("CARGO_PKG_NAME"), ".log"), true);
    cui::main_menu(&Lang::resolve()).await?;
    Ok(())
}
