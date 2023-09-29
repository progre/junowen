mod cui;

use anyhow::Result;
use junowen_lib::lang::Lang;

#[tokio::main]
async fn main() -> Result<()> {
    cui::main_menu(&Lang::new("ja")).await?;
    Ok(())
}
