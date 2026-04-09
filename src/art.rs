//! ASCII banner and help copy for the CLI.

/// Pub/Sub “wire” motif (pure ASCII).
pub const BANNER: &str = r"
    .---.        .---.
   /     \ ~~~ /     \        P U B / S U B
  |   o   |   |   o   |       async pipeline lab
   \     / ~~~ \     /
    `---`        `---`
";

/// Short tagline for `--help` headers.
pub const TAGLINE: &str = "In-process pub/sub (broadcast) + bounded mpsc pipelines + fan-in.";

/// Long-form body for `clap` (`long_about`): banner + tagline + tips.
pub const HELP_LONG: &str = r"

    .---.        .---.
   /     \ ~~~ /     \        P U B / S U B
  |   o   |   |   o   |       async pipeline lab
   \     / ~~~ \     /
    `---`        `---`

In-process pub/sub (broadcast) + bounded mpsc pipelines + fan-in.

Tips
  RUST_LOG=info (or debug, trace) works with `pubsub --verbose …`.

Safety model
  This CLI runs against the in-process library only (no network, no persistence).
  Use it for local runs and automated tests.
";
