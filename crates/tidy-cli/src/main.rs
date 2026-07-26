use std::{env, path::PathBuf, process};

use anyhow::{Context, Result, bail};
use tidy_core::{CrawlLimits, DiscoverOptions, FetchOptions, Vault, discover, fetch, parse_prefix};

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
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).context("failed to serialize report")?
                );
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
        "fetch" => {
            let prefix_arg = args.next().context(
                "usage: tidy fetch <url-prefix> --vault PATH [--limit N] [--no-images] [--json]",
            )?;

            let mut vault = None;
            let mut limit = None;
            let mut download_images = true;
            let mut json = false;

            while let Some(flag) = args.next() {
                match flag.as_str() {
                    "--vault" => {
                        let value = args.next().context("--vault requires a path")?;
                        vault = Some(PathBuf::from(value));
                    }
                    "--limit" => {
                        let value = args.next().context("--limit requires a number")?;
                        limit = Some(value.parse::<usize>().context("invalid --limit")?);
                    }
                    "--no-images" => download_images = false,
                    "--json" => json = true,
                    other => bail!("unknown flag `{other}`"),
                }
            }

            let vault = vault.context("--vault is required")?;
            let prefix = parse_prefix(&prefix_arg)
                .with_context(|| format!("invalid prefix `{prefix_arg}`"))?;

            let report = fetch(FetchOptions {
                url_prefix: prefix,
                vault,
                limit,
                download_images,
                title: None,
                backfill_policy: None,
            })
            .await
            .context("fetch failed")?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).context("failed to serialize report")?
                );
            } else {
                println!("source:       {}", report.source_slug);
                println!("discovered:   {}", report.discovered);
                println!(
                    "added/updated/skipped/failed: {}/{}/{}/{}",
                    report.added, report.updated, report.skipped, report.failed
                );
                if report.needs_review > 0 {
                    println!("needs review: {}", report.needs_review);
                }
                println!();
                for (index, item) in report.articles.iter().enumerate() {
                    println!(
                        "{:>4}. [{:?}] {} ({})\n      {}",
                        index + 1,
                        item.status,
                        item.title,
                        item.quality,
                        item.path
                    );
                }
                if !report.warnings.is_empty() {
                    println!();
                    println!("warnings:");
                    for warning in report.warnings.iter().take(20) {
                        println!("  - {warning}");
                    }
                    if report.warnings.len() > 20 {
                        println!("  … {} more", report.warnings.len() - 20);
                    }
                }
            }
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => bail!("unknown command `{other}`\n\n{HELP}"),
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
    fetch <URL-PREFIX>          Discover, extract, and save articles
        --vault PATH            Vault directory (required)
        --limit N               Cap articles fetched
        --no-images             Skip downloading images
        --json                  Emit machine-readable JSON
    help                        Show this help

Discovery order: feed → sitemap → HTML crawl fallback.
Extraction: readability → markdown + frontmatter → SQLite index.
";
