use crater::Error;

use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    #[cfg(debug_assertions)]
    let _ = dotenv::from_filename(".env.local")?;

    #[cfg(not(debug_assertions))]
    let _ = dotenv::dotenv();

    #[cfg(debug_assertions)]
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    if std::env::var("RUST_LOG").is_err() {
        #[cfg(not(debug_assertions))]
        let val = "crater=info";

        #[cfg(debug_assertions)]
        let val = "crater=debug";

        std::env::set_var("RUST_LOG", val);
    }

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "crater=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let token = std::env::var("RAILWAY_API_TOKEN").map_err(|_| Error::MissingEnvVar("RAILWAY_API_TOKEN"))?;
    crater::run(token).await?;

    Ok(())
}
