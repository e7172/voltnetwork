//! CLI wallet for the chainless token transfer network.

mod commands;
mod config;
mod errors;
mod wallet;

use anyhow::Result;
use colored::Colorize;
use commands::{balance, export_seed, init_seed, mint, send, issue_token, mint_token};
use config::WalletConfig;
use errors::WalletError;
use std::path::PathBuf;
use structopt::StructOpt;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

/// Command line arguments for the CLI wallet.
#[derive(Debug, StructOpt)]
#[structopt(name = "wallet", about = "Chainless token transfer network wallet")]
struct Opt {
    /// Path to the configuration file
    #[structopt(short, long, parse(from_os_str))]
    config: Option<PathBuf>,

    /// Path to the wallet file
    #[structopt(short, long, parse(from_os_str))]
    wallet: Option<PathBuf>,

    /// Node to connect to
    #[structopt(short, long)]
    node: Option<String>,

    /// Subcommand to run
    #[structopt(subcommand)]
    cmd: Command,
}

/// Subcommands for the CLI wallet.
#[derive(Debug, StructOpt)]
enum Command {
    /// Get the balance of an account
    #[structopt(name = "balance")]
    Balance,

    /// Send tokens to another account
    #[structopt(name = "send")]
    Send {
        /// Recipient address
        #[structopt(long)]
        to: String,

        /// Amount to send
        #[structopt(long)]
        amount: u128,
    },

    /// Mint new tokens (treasury only)
    #[structopt(name = "mint")]
    Mint {
        /// Recipient address
        #[structopt(long)]
        to: String,

        /// Amount to mint
        #[structopt(long)]
        amount: u128,
    },

    /// Initialize a new seed
    #[structopt(name = "init-seed")]
    InitSeed,

    /// Export the seed
    #[structopt(name = "export-seed")]
    ExportSeed,

    /// Issue a new token
    #[structopt(name = "issue-token")]
    IssueToken {
        /// Token metadata (name, symbol, decimals, etc.)
        #[structopt(long)]
        metadata: String,

        /// Collateral amount (optional)
        #[structopt(long)]
        collateral: Option<u128>,
    },

    /// Mint tokens for a specific token ID
    #[structopt(name = "mint-token")]
    MintToken {
        /// Token ID
        #[structopt(long)]
        token_id: u64,

        /// Recipient address
        #[structopt(long)]
        to: String,

        /// Amount to mint
        #[structopt(long)]
        amount: u128,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Parse command line arguments
    let opt = Opt::from_args();

    // Load configuration
    let mut config = match &opt.config {
        Some(path) => WalletConfig::from_file(path)?,
        None => WalletConfig::default(),
    };

    // Override node if specified
    if let Some(node) = opt.node {
        config.node = node;
    }

    // Determine wallet file
    let wallet_file = match opt.wallet {
        Some(path) => path,
        None => {
            let mut dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
            dir.push("stateless-token");
            dir.push("wallet.dat");
            dir
        }
    };

    // Create parent directory if it doesn't exist
    if let Some(parent) = wallet_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Run the appropriate command
    match opt.cmd {
        Command::Balance => {
            let balance = balance::run(&config, &wallet_file).await?;
            println!("{} {}", "Balance:".green(), balance);
        }
        Command::Send { to, amount } => {
            let tx_hash = send::run(&config, &wallet_file, &to, amount).await?;
            println!("{} {}", "Transaction sent:".green(), tx_hash);
        }
        Command::Mint { to, amount } => {
            let tx_hash = mint::run(&config, &wallet_file, &to, amount).await?;
            println!("{} {}", "Tokens minted:".green(), tx_hash);
        }
        Command::InitSeed => {
            init_seed::run(&wallet_file).await?;
            println!("{} {}", "Seed initialized:".green(), wallet_file.display());
        }
        Command::ExportSeed => {
            let seed = export_seed::run(&wallet_file).await?;
            println!("{} {}", "Seed:".green(), seed);
            println!("{}", "WARNING: Keep this seed safe and private!".red());
        }
        Command::IssueToken { metadata, collateral } => {
            let token_id = issue_token::run(&config, &wallet_file, &metadata, collateral).await?;
            println!("{} {}", "Token issued:".green(), token_id);
        }
        Command::MintToken { token_id, to, amount } => {
            let tx_hash = mint_token::run(&config, &wallet_file, token_id, &to, amount).await?;
            println!("{} {}", "Tokens minted:".green(), tx_hash);
        }
    }

    Ok(())
}
