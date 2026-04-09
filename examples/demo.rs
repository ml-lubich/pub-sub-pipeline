//! `cargo run --example demo`
//!
//! Same flow as `pubsub demo`, kept as a library-only example entrypoint.

use pub_sub_pipeline::scenarios::run_demo_report;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = run_demo_report().await?;
    print!("{report}");
    Ok(())
}
