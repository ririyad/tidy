use std::{
    path::PathBuf,
    sync::Mutex,
};

use serde::Serialize;
use tauri::{Emitter, State};
use tidy_core::{
    apply_article_state, fetch_with_progress, load_reader_settings, parse_prefix,
    save_reader_settings, source_slug, ArticleDetail, ArticleFilter, ArticleListItem,
    ArticleStatePatch, FetchOptions, FetchProgress, FetchReport, Index, ReaderSettings,
    SourceRow, Vault, VaultSummary,
};
use url::Url;

#[derive(Default)]
struct AppState {
    vault_path: Mutex<Option<PathBuf>>,
}

fn with_vault<T>(
    state: &AppState,
    f: impl FnOnce(&Vault, &Index) -> Result<T, String>,
) -> Result<T, String> {
    let guard = state.vault_path.lock().map_err(|e| e.to_string())?;
    let path = guard
        .as_ref()
        .ok_or_else(|| "no vault open — choose a vault first".to_string())?;
    let vault = Vault::open(path).map_err(|e| e.to_string())?;
    let index = Index::open(vault.database_path()).map_err(|e| e.to_string())?;
    f(&vault, &index)
}

#[tauri::command]
fn select_vault(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<VaultSummary>, String> {
    use tauri_plugin_dialog::DialogExt;

    let path = app
        .dialog()
        .file()
        .set_title("Choose a Tidy vault folder")
        .blocking_pick_folder();

    let Some(file_path) = path else {
        return Ok(None);
    };

    let path = file_path
        .into_path()
        .map_err(|error| format!("invalid vault path: {error}"))?;

    let summary = Vault::initialize(&path).map_err(|e| e.to_string())?;
    *state.vault_path.lock().map_err(|e| e.to_string())? = Some(summary.path.clone());
    Ok(Some(summary))
}

#[tauri::command]
fn open_vault_path(state: State<'_, AppState>, path: String) -> Result<VaultSummary, String> {
    let summary = Vault::initialize(PathBuf::from(path)).map_err(|e| e.to_string())?;
    *state.vault_path.lock().map_err(|e| e.to_string())? = Some(summary.path.clone());
    Ok(summary)
}

#[tauri::command]
fn get_open_vault(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let guard = state.vault_path.lock().map_err(|e| e.to_string())?;
    Ok(guard.as_ref().map(|p| p.display().to_string()))
}

#[tauri::command]
fn list_sources(state: State<'_, AppState>) -> Result<Vec<SourceRow>, String> {
    with_vault(&state, |_, index| index.list_sources().map_err(|e| e.to_string()))
}

#[derive(Debug, serde::Deserialize)]
struct AddSourceRequest {
    url_prefix: String,
    title: Option<String>,
    backfill: String, // "recent" | "full"
    recent_limit: Option<usize>,
}

#[tauri::command]
async fn add_source(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: AddSourceRequest,
) -> Result<FetchReport, String> {
    let vault_path = {
        let guard = state.vault_path.lock().map_err(|e| e.to_string())?;
        guard
            .clone()
            .ok_or_else(|| "no vault open".to_string())?
    };

    let prefix = parse_prefix(&request.url_prefix).map_err(|e| e.to_string())?;
    let limit = match request.backfill.as_str() {
        "full" => None,
        _ => Some(request.recent_limit.unwrap_or(20)),
    };
    let title = request
        .title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| source_slug(&prefix));

    let app_handle = app.clone();
    let report = fetch_with_progress(
        FetchOptions {
            url_prefix: prefix,
            vault: vault_path,
            limit,
            download_images: true,
            title: Some(title),
            backfill_policy: Some(request.backfill),
        },
        move |progress: FetchProgress| {
            let _ = app_handle.emit("fetch-progress", &progress);
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(report)
}

#[tauri::command]
async fn refresh_source(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    source_id: i64,
    limit: Option<usize>,
) -> Result<FetchReport, String> {
    let (vault_path, prefix, title, backfill) = with_vault(&state, |vault, index| {
        let source = index
            .get_source(source_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("source {source_id} not found"))?;
        Ok((
            vault.root().to_path_buf(),
            source.url_prefix,
            source.title,
            source.backfill_policy,
        ))
    })?;

    let prefix = Url::parse(&prefix)
        .or_else(|_| parse_prefix(&prefix).map_err(|e| e.to_string()))
        .map_err(|e| e.to_string())?;

    let limit = limit.or_else(|| {
        if backfill == "full" {
            None
        } else {
            Some(20)
        }
    });

    let app_handle = app.clone();
    fetch_with_progress(
        FetchOptions {
            url_prefix: prefix,
            vault: vault_path,
            limit,
            download_images: true,
            title: Some(title),
            backfill_policy: Some(backfill),
        },
        move |progress: FetchProgress| {
            let _ = app_handle.emit("fetch-progress", &progress);
        },
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_source(state: State<'_, AppState>, source_id: i64) -> Result<bool, String> {
    with_vault(&state, |_, index| {
        index.delete_source(source_id).map_err(|e| e.to_string())
    })
}

#[derive(Debug, serde::Deserialize)]
struct ListArticlesRequest {
    filter: Option<String>,
    source_id: Option<i64>,
    limit: Option<i64>,
}

#[tauri::command]
fn list_articles(
    state: State<'_, AppState>,
    request: ListArticlesRequest,
) -> Result<Vec<ArticleListItem>, String> {
    let filter = match request.filter.as_deref().unwrap_or("inbox") {
        "unread" => ArticleFilter::Unread,
        "starred" => ArticleFilter::Starred,
        "archived" => ArticleFilter::Archived,
        "all" => ArticleFilter::All,
        _ => ArticleFilter::Inbox,
    };
    with_vault(&state, |_, index| {
        index
            .list_articles(filter, request.source_id, request.limit)
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn get_article(state: State<'_, AppState>, id: i64) -> Result<Option<ArticleDetail>, String> {
    with_vault(&state, |_, index| {
        index.get_article(id).map_err(|e| e.to_string())
    })
}

#[derive(Debug, serde::Deserialize)]
struct SetArticleStateRequest {
    id: i64,
    state: Option<String>,
    starred: Option<bool>,
    archived: Option<bool>,
}

#[tauri::command]
fn set_article_state(
    state: State<'_, AppState>,
    request: SetArticleStateRequest,
) -> Result<ArticleDetail, String> {
    with_vault(&state, |vault, index| {
        apply_article_state(
            vault,
            index,
            request.id,
            ArticleStatePatch {
                state: request.state,
                starred: request.starred,
                archived: request.archived,
            },
        )
        .map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn set_article_progress(
    state: State<'_, AppState>,
    id: i64,
    progress: f64,
) -> Result<(), String> {
    with_vault(&state, |_, index| {
        index
            .update_article_progress(id, progress)
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn get_reader_settings(state: State<'_, AppState>) -> Result<ReaderSettings, String> {
    with_vault(&state, |vault, _| {
        load_reader_settings(vault).map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn set_reader_settings(
    state: State<'_, AppState>,
    settings: ReaderSettings,
) -> Result<ReaderSettings, String> {
    with_vault(&state, |vault, _| {
        save_reader_settings(vault, &settings).map_err(|e| e.to_string())?;
        Ok(settings)
    })
}

#[derive(Serialize)]
struct AppInfo {
    name: String,
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo {
        name: tidy_core::APP_NAME.to_string(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            vault_path: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            select_vault,
            open_vault_path,
            get_open_vault,
            list_sources,
            add_source,
            refresh_source,
            remove_source,
            list_articles,
            get_article,
            set_article_state,
            set_article_progress,
            get_reader_settings,
            set_reader_settings,
            get_app_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tidy");
}
