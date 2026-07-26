export type VaultSummary = {
  path: string;
  database_path: string;
  created: boolean;
};

export type SourceRow = {
  id: number;
  url_prefix: string;
  title: string;
  feed_url: string | null;
  backfill_policy: string;
  last_fetch_at: string | null;
  enabled: boolean;
  article_count: number;
  unread_count: number;
};

export type ArticleListItem = {
  id: number;
  source_id: number;
  source_title: string;
  url: string;
  path: string;
  title: string;
  author: string | null;
  published_at: string | null;
  fetched_at: string;
  word_count: number;
  reading_time: number;
  excerpt: string;
  state: string;
  starred: boolean;
  archived: boolean;
  quality: string;
};

export type ArticleDetail = ArticleListItem & {
  body: string;
  rendered_html: string;
  progress: number;
  revision: number;
};

export type ReaderSettings = {
  theme: 'paper' | 'ink' | 'sepia' | string;
  font: 'serif' | 'sans' | string;
  font_size: number;
  line_height: number;
  measure: 'narrow' | 'wide' | string;
};

export type FetchProgress = {
  source_id: number;
  phase: string;
  current: number;
  total: number;
  url: string | null;
  title: string | null;
  message: string;
};

export type FeedFilter = 'inbox' | 'unread' | 'starred' | 'archived' | 'all';

export type DayGroup = {
  label: string;
  key: string;
  articles: ArticleListItem[];
};
