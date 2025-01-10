use clap::Parser;
use ai_file_organize::run_app;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// 要整理的文件夹
    #[arg(long)]
    temp_dir: String,
    /// 配置文件路径
    #[arg(long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    // 调用业务逻辑
    run_app(cli.temp_dir, cli.config).await?;
    Ok(())
}
