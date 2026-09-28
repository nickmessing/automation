use std::io::IsTerminal;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use chrono::Local;
use clap::{Parser, Subcommand};
use rates::cli::{self, Output, RateArgs};
use rates::store::Store;
use rates::web::{self, AppState};
use rates::{Client, api};

/// Exchange rates from the Frankfurter API.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Frankfurter API base URL
    #[arg(long, env = "RATES_API", default_value = api::DEFAULT_API, global = true)]
    api: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Get a rate on one date, or a range of rates with a graph
    Rate {
        /// Currency to convert from, e.g. EUR
        base: String,
        /// Currency to convert to, e.g. USD
        quote: String,
        /// Date in YYYY-MM-DD format, latest if omitted
        date: Option<String>,
        /// Start of a date range, draws a graph
        #[arg(long)]
        from: Option<String>,
        /// End of the date range, today if omitted
        #[arg(long)]
        to: Option<String>,
        /// Skip the graph (it is also skipped when stdout is not a terminal)
        #[arg(long)]
        no_graph: bool,
        /// Where data/ and error.log go
        #[arg(long, env = "RATES_ROOT", default_value = ".")]
        root: PathBuf,
    },
    /// Serve the chart page and JSON API
    Serve {
        /// Address to listen on
        #[arg(long, env = "RATES_LISTEN", default_value = "127.0.0.1:3000")]
        listen: SocketAddr,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let client = Client::new(cli.api);

    match cli.command {
        Command::Rate {
            base,
            quote,
            date,
            from,
            to,
            no_graph,
            root,
        } => {
            let args = RateArgs {
                base,
                quote,
                date,
                from,
                to,
            };
            let store = Store::new(root);
            let today = Local::now().date_naive();
            let terminal = std::io::stdout().is_terminal();
            let output = Output {
                graph: (terminal && !no_graph)
                    .then(|| terminal_size::terminal_size().map_or((80, 24), |(w, h)| (w.0 as usize, h.0 as usize))),
                color: terminal,
            };

            match cli::run(&client, &store, &args, today, output, &mut std::io::stdout()).await {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    let _ = store.log_error(&args.context(today), &e);
                    eprintln!("Error: {e}");
                    if let Some(help) = e.help() {
                        eprintln!("  help: {help}");
                    }
                    ExitCode::FAILURE
                }
            }
        }
        Command::Serve { listen } => match web::serve(listen, AppState::new(client)).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("Error: cannot serve on {listen}: {e}");
                ExitCode::FAILURE
            }
        },
    }
}
