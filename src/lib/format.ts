import type { ArticleListItem, DayGroup } from './types';

export function groupByDay(articles: ArticleListItem[]): DayGroup[] {
  const groups = new Map<string, DayGroup>();

  for (const article of articles) {
    const raw = article.published_at || article.fetched_at;
    const date = new Date(raw);
    const key = Number.isNaN(date.getTime())
      ? 'unknown'
      : date.toISOString().slice(0, 10);
    const label =
      key === 'unknown'
        ? 'Undated'
        : date.toLocaleDateString(undefined, {
            weekday: 'long',
            month: 'long',
            day: 'numeric',
            year: 'numeric'
          });

    if (!groups.has(key)) {
      groups.set(key, { key, label, articles: [] });
    }
    groups.get(key)!.articles.push(article);
  }

  return [...groups.values()];
}

export function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

export function formatRelative(iso: string | null | undefined): string {
  if (!iso) return 'never';
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return 'unknown';
  const deltaMs = date.getTime() - Date.now();
  const absMin = Math.round(Math.abs(deltaMs) / 60000);
  const suffix = deltaMs <= 0 ? 'ago' : 'from now';
  if (absMin < 1) return deltaMs <= 0 ? 'just now' : 'soon';
  if (absMin < 60) return `${absMin}m ${suffix}`;
  const hours = Math.round(absMin / 60);
  if (hours < 48) return `${hours}h ${suffix}`;
  const days = Math.round(hours / 24);
  return `${days}d ${suffix}`;
}

export function formatInterval(minutes: number): string {
  if (minutes < 60) return `${minutes}m`;
  if (minutes % 60 === 0) {
    const hours = minutes / 60;
    return hours === 1 ? '1h' : `${hours}h`;
  }
  const hours = Math.floor(minutes / 60);
  const rem = minutes % 60;
  return `${hours}h ${rem}m`;
}
