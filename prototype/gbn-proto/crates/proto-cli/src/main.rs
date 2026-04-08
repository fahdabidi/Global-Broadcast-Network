//! # GBN Prototype CLI
//!
//! Command-line tool that orchestrates the full Media Creation Network pipeline
//! for testing and demonstration.
//!
//! ## Commands
//!
//! ```text
//! gbn-proto keygen                          Generate a Publisher Ed25519/X25519 keypair
//! gbn-proto upload --input <video>          Sanitize, chunk, encrypt, and relay to Publisher
//!   --publisher-key <key>                   Publisher's public key (hex)
//!   --paths <N>                             Number of parallel relay paths (default: 3)
//!   --hops <N>                              Relay hops per path (default: 3)
//!   --chunk-size <bytes>                    Chunk size in bytes (default: 1048576)
//! gbn-proto verify --original <f> --reassembled <f>   Compare SHA-256 hashes
//! ```

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gbn-proto")]
#[command(about = "Global Broadcast Network — Phase 1 Prototype CLI")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new Publisher keypair (Ed25519 + X25519)
    Keygen,

    /// Upload a video through the MCN pipeline
    Upload {
        /// Path to the input video file
        #[arg(short, long)]
        input: String,

        /// Publisher's public key (hex-encoded)
        #[arg(short, long)]
        publisher_key: String,

        /// Number of parallel relay paths
        #[arg(long, default_value = "3")]
        paths: usize,

        /// Number of relay hops per path
        #[arg(long, default_value = "3")]
        hops: usize,

        /// Chunk size in bytes
        #[arg(long, default_value = "1048576")]
        chunk_size: usize,
    },

    /// Verify that a reassembled video matches the original
    Verify {
        /// Path to the original (sanitized) video
        #[arg(long)]
        original: String,

        /// Path to the reassembled video
        #[arg(long)]
        reassembled: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("gbn=info".parse()?),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Keygen => {
            tracing::info!("Generating Publisher keypair...");
            // TODO: Implement with mcn-crypto
            println!("Keygen not yet implemented — coming in Phase 1 execution");
        }
        Commands::Upload {
            input,
            publisher_key,
            paths,
            hops,
            chunk_size,
        } => {
            tracing::info!(
                input = %input,
                paths = paths,
                hops = hops,
                chunk_size = chunk_size,
                "Starting MCN upload pipeline"
            );
            // TODO: Wire together sanitizer → chunker → crypto → router → receiver
            println!("Upload pipeline not yet implemented — coming in Phase 1 execution");
        }
        Commands::Verify {
            original,
            reassembled,
        } => {
            tracing::info!(
                original = %original,
                reassembled = %reassembled,
                "Verifying reassembly integrity"
            );
            // TODO: Compare SHA-256 hashes
            println!("Verify not yet implemented — coming in Phase 1 execution");
        }
    }

    Ok(())
}
