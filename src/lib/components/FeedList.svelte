<script lang="ts">
  import { groupByDay } from '../format';
  import type { ArticleListItem } from '../types';

  let {
    articles,
    selectedId,
    progressMessage,
    fetching,
    searchQuery,
    searchInput = $bindable(null),
    onSelect,
    onSearchChange,
    onStopFetch
  }: {
    articles: ArticleListItem[];
    selectedId: number | null;
    progressMessage: string;
    fetching: boolean;
    searchQuery: string;
    searchInput?: HTMLInputElement | null;
    onSelect: (id: number) => void;
    onSearchChange: (value: string) => void;
    onStopFetch: () => void;
  } = $props();

  const groups = $derived(groupByDay(articles));
</script>

<section class="panel flex h-full min-h-0 flex-col overflow-hidden">
  <header class="shrink-0 border-b border-[var(--line)] px-5 py-4">
    <p class="text-xs font-semibold tracking-[0.14em] text-[var(--ink-soft)] uppercase">Feed</p>
    <label class="mt-3 block">
      <span class="sr-only">Search articles</span>
      <input
        bind:this={searchInput}
        class="w-full rounded-xl border border-[var(--line)] bg-white/80 px-3 py-2 text-sm outline-none focus:border-[var(--accent)]"
        placeholder="Search title, excerpt, body…"
        value={searchQuery}
        oninput={(event) => onSearchChange((event.currentTarget as HTMLInputElement).value)}
      />
    </label>
    <div class="mt-2 flex items-start justify-between gap-3">
      <p class="min-w-0 flex-1 break-words text-sm text-[var(--ink-soft)]">
        {articles.length} articles
        {#if progressMessage}
          <span class="text-[var(--accent)]"> · {progressMessage}</span>
        {:else if fetching}
          <span class="text-[var(--accent)]"> · Fetching…</span>
        {/if}
      </p>
      {#if fetching}
        <button
          class="shrink-0 rounded-lg border border-[var(--line)] bg-white px-2.5 py-1 text-xs font-semibold text-[var(--ink)] hover:bg-[var(--accent-soft)] hover:text-[var(--accent)]"
          onclick={onStopFetch}
        >
          Stop
        </button>
      {/if}
    </div>
  </header>

  <div
    class="feed-scroll min-h-0 flex-1 overflow-x-hidden overflow-y-auto overscroll-contain px-2 py-3"
  >
    {#if articles.length === 0}
      <div class="px-4 py-10 text-sm leading-7 text-[var(--ink-soft)]">
        {#if searchQuery.trim()}
          No articles match this search. Try fewer terms or clear the query.
        {:else}
          Nothing here yet. Add a source, switch filters, or open Review for thin extractions.
        {/if}
      </div>
    {:else}
      {#each groups as group}
        <div class="mb-4">
          <p class="px-3 pb-2 text-[0.7rem] font-semibold tracking-[0.12em] text-[var(--ink-soft)] uppercase">
            {group.label}
          </p>
          {#each group.articles as article}
            <button
              class="feed-item mb-1 w-full rounded-2xl px-3 py-3 text-left transition
                {selectedId === article.id
                ? 'bg-[var(--accent-soft)]'
                : 'hover:bg-white/70'}"
              onclick={() => onSelect(article.id)}
            >
              <div class="flex items-start justify-between gap-3">
                <p
                  class="text-[0.95rem] leading-snug
                    {article.state === 'unread' ? 'font-semibold' : 'font-medium text-[var(--ink-soft)]'}"
                >
                  {article.title}
                </p>
                {#if article.starred}
                  <span class="text-[var(--accent)]" aria-label="Starred">★</span>
                {/if}
              </div>
              <p class="mt-1 line-clamp-2 text-xs leading-5 text-[var(--ink-soft)]">
                {article.source_title}
                · {article.reading_time} min
                {#if article.author}
                  · {article.author}
                {/if}
                {#if article.quality === 'needs_review'}
                  · <span class="text-[var(--warn)]">needs review</span>
                {/if}
              </p>
            </button>
          {/each}
        </div>
      {/each}
    {/if}
  </div>
</section>
