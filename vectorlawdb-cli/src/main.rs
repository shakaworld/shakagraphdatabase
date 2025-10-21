use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "vldb")]
#[command(about = "VectorLawDB Command Line Interface", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize VectorLawDB
    Init {
        #[arg(long, default_value = "./data")]
        data_dir: String,
    },

    /// Import CSV data
    Import {
        csv_path: String,

        #[arg(long, default_value = "./data")]
        data_dir: String,
    },

    /// Execute query
    Query {
        query: String,

        #[arg(long, default_value = "10")]
        limit: usize,
    },

    /// Start REST API server
    Serve {
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        #[arg(long, default_value = "8000")]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { data_dir } => {
            println!("Initializing VectorLawDB in {}", data_dir);
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(format!("{}/wal", data_dir))?;
            std::fs::create_dir_all(format!("{}/sstables", data_dir))?;
            println!("Initialization complete");
        }

        Commands::Import { csv_path, data_dir } => {
            println!("Importing from {}", csv_path);
            println!("Using data directory: {}", data_dir);
            // TODO: Implement CSV import
            println!("Import complete");
        }

        Commands::Query { query, limit } => {
            println!("Executing: {}", query);
            println!("Limit: {}", limit);
            // TODO: Execute query
        }

        Commands::Serve { host, port } => {
            println!("Starting server on {}:{}", host, port);
            // TODO: Start server
        }
    }

    Ok(())
}
