use std::{env, path::PathBuf, process};

use anyhow::{bail, Context, Result};
use tidy_core::{discover, parse_prefix, CrawlLimits, DiscoverOptions, Vault};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("error: {error:#}");
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".into());

    match command.as_str() {
        "init" => {
            let path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            let summary = Vault::initialize(&path)
                .with_context(|| format!("failed to initialize vault at {}", path.display()))?;

            if summary.created {
                println!("Created Tidy vault at {}", summary.path.display());
            } else {
                println!("Opened existing Tidy vault at {}", summary.path.display());
            }
            println!("Index: {}", summary.database_path.display());
            Ok(())
        }
        "discover" => {
            let prefix_arg = args
                .next()
                .context("usage: tidy discover <url-prefix> [--limit N] [--cache DIR]")?;

            let mut limit = None;
            let mut cache_dir = None;
            let mut json = false;

            while let Some(flag) = args.next() {
                match flag.as_str() {
                    "--limit" => {
                        let value = args.next().context("--limit requires a number")?;
                        limit = Some(value.parse::<usize>().context("invalid --limit")?);
                    }
                    "--cache" => {
                        let value = args.next().context("--cache requires a path")?;
                        cache_dir = Some(PathBuf::from(value));
                    }
                    "--json" => json = true,
                    other => bail!("unknown flag `{other}`"),
                }
            }

            let prefix = parse_prefix(&prefix_arg)
                .with_context(|| format!("invalid prefix `{prefix_arg}`"))?;

            let report = discover(DiscoverOptions {
                url_prefix: prefix,
                limit,
                cache_dir,
                limits: CrawlLimits::default(),
            })
            .await
            .context("discovery failed")?;

            if json {
                let payload = serde_json::to_string_pretty(&report)
                    .context("failed to serialize report")?;
                println!("{payload}");
            } else {
                println!("prefix:          {}", report.prefix);
                println!("primary source:  {:?}", report.primary_source);
                if let Some(feed) = &report.feed_url {
                    println!("feed:            {feed}");
                }
                if !report.sitemap_urls.is_empty() {
                    println!(
                        "sitemaps:        {}",
                        report
                            .sitemap_urls
                            .iter()
                            .map(|u| u.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                println!("urls discovered: {}", report.urls.len());
                println!();
                for (index, item) in report.urls.iter().enumerate() {
                    let title = item.title.as_deref().unwrap_or("-");
                    println!(
                        "{:>4}. [{}] {}\n      {}",
                        index + 1,
                        format!("{:?}", item.source).to_lowercase(),
                        title,
                        item.url
                    );
                }
                if !report.warnings.is_empty() {
                    println!();
                    println!("warnings:");
                    for warning in &report.warnings {
                        println!("  - {warning}");
                    }
                }
            }
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            bail!("unknown command `{other}`\n\n{}", HELP);
        }
    }
}

fn print_help() {
    print!("{HELP}");
}

const HELP: &str = "\
tidy — local-first fetch engine and vault tools

USAGE:
    tidy <COMMAND>

COMMANDS:
    init [PATH]                 Create or open a Tidy vault (default: .)
    discover <URL-PREFIX>       Enumerate posts under a URL prefix
        --limit N               Cap the number of printed URLs
        --cache DIR             HTTP cache directory
        --json                  Emit machine-readable JSON
    help                        Show this help

Discovery order: feed → sitemap → HTML crawl fallback.
";
