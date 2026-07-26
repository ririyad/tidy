use std::{env, path::PathBuf, process};

use anyhow::{Context, Result, bail};
use tidy_core::{
    CrawlLimits, DiscoverOptions, FetchOptions, Index, Vault, discover, fetch, list_run_history,
    parse_prefix, schedule_status,
};

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
                interval_minutes: None,
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
        "schedule" => {
            let mut vault = None;
            let mut json = false;
            let mut runs = false;
            let mut source_id = None;
            let mut limit = 20i64;

            while let Some(flag) = args.next() {
                match flag.as_str() {
                    "--vault" => {
                        let value = args.next().context("--vault requires a path")?;
                        vault = Some(PathBuf::from(value));
                    }
                    "--json" => json = true,
                    "--runs" => runs = true,
                    "--source" => {
                        let value = args.next().context("--source requires an id")?;
                        source_id = Some(value.parse::<i64>().context("invalid --source")?);
                    }
                    "--limit" => {
                        let value = args.next().context("--limit requires a number")?;
                        limit = value.parse::<i64>().context("invalid --limit")?;
                    }
                    other => bail!("unknown flag `{other}`"),
                }
            }

            let vault_path = vault.context(
                "usage: tidy schedule --vault PATH [--runs] [--source ID] [--limit N] [--json]",
            )?;
            let vault = Vault::open(&vault_path)
                .with_context(|| format!("failed to open vault at {}", vault_path.display()))?;
            let index = Index::open(vault.database_path()).context("failed to open index")?;

            if runs {
                let history = list_run_history(&index, source_id, limit)
                    .context("failed to list fetch runs")?;
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&history)
                            .context("failed to serialize runs")?
                    );
                } else if history.is_empty() {
                    println!("No fetch runs yet.");
                } else {
                    for run in history {
                        println!(
                            "#{:<4} {:<20} {:<8} +{}/~{}/={}/!{}  {}",
                            run.id,
                            run.source_title,
                            run.status,
                            run.added,
                            run.updated,
                            run.skipped,
                            run.failed,
                            run.started_at
                        );
                    }
                }
            } else {
                let statuses =
                    schedule_status(&index).context("failed to load schedule status")?;
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&statuses)
                            .context("failed to serialize schedule")?
                    );
                } else if statuses.is_empty() {
                    println!("No sources registered.");
                } else {
                    for item in statuses {
                        let due = if item.due { "due" } else { "ok" };
                        let enabled = if item.enabled { "on" } else { "off" };
                        println!(
                            "#{:<4} {:<24} every {:>4}m  [{enabled}] [{due}]  last {}",
                            item.source_id,
                            item.title,
                            item.interval_minutes,
                            item.last_fetch_at.as_deref().unwrap_or("never")
                        );
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
    schedule --vault PATH       Show per-source intervals and due status
        --runs                  Show recent fetch run history
        --source ID             Filter runs to one source
        --limit N               Cap history rows (default 20)
        --json                  Emit machine-readable JSON
    help                        Show this help

Discovery order: feed → sitemap → HTML crawl fallback.
Extraction: readability → markdown + frontmatter → SQLite index.
Scheduler: due sources = enabled and (never fetched or interval elapsed).
";
