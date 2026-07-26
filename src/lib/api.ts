import { invoke } from '@tauri-apps/api/core';
import type {
  ArticleDetail,
  ArticleListItem,
  FeedFilter,
  ReaderSettings,
  SourceRow,
  VaultSummary
} from './types';

export const api = {
  selectVault: () => invoke<VaultSummary | null>('select_vault'),
  listSources: () => invoke<SourceRow[]>('list_sources'),
  addSource: (payload: {
    url_prefix: string;
    title?: string;
    backfill: 'recent' | 'full';
    recent_limit?: number;
  }) => invoke('add_source', { request: payload }),
  refreshSource: (sourceId: number, limit?: number) =>
    invoke('refresh_source', { sourceId, limit }),
  removeSource: (sourceId: number) => invoke<boolean>('remove_source', { sourceId }),
  listArticles: (filter: FeedFilter, sourceId?: number | null) =>
    invoke<ArticleListItem[]>('list_articles', {
      request: {
        filter,
        source_id: sourceId ?? null,
        limit: null
      }
    }),
  getArticle: (id: number) => invoke<ArticleDetail | null>('get_article', { id }),
  setArticleState: (payload: {
    id: number;
    state?: string;
    starred?: boolean;
    archived?: boolean;
  }) => invoke<ArticleDetail>('set_article_state', { request: payload }),
  setArticleProgress: (id: number, progress: number) =>
    invoke('set_article_progress', { id, progress }),
  getReaderSettings: () => invoke<ReaderSettings>('get_reader_settings'),
  setReaderSettings: (settings: ReaderSettings) =>
    invoke<ReaderSettings>('set_reader_settings', { settings })
};
