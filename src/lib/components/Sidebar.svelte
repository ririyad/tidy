<script lang="ts">
  import type { FeedFilter, SourceRow } from '../types';

  let {
    sources,
    filter,
    selectedSourceId,
    fetching,
    onFilter,
    onSelectSource,
    onAddSource,
    onRefresh,
    onRemoveSource,
    onChangeVault
  }: {
    sources: SourceRow[];
    filter: FeedFilter;
    selectedSourceId: number | null;
    fetching: boolean;
    onFilter: (filter: FeedFilter) => void;
    onSelectSource: (id: number | null) => void;
    onAddSource: () => void;
    onRefresh: () => void;
    onRemoveSource: (id: number) => void;
    onChangeVault: () => void;
  } = $props();

  const filters: { id: FeedFilter; label: string }[] = [
    { id: 'inbox', label: 'Inbox' },
    { id: 'starred', label: 'Starred' },
    { id: 'archived', label: 'Archive' },
    { id: 'all', label: 'All' }
  ];
</script>

<aside class="panel flex h-full flex-col px-4 py-5">
  <div class="mb-6 flex items-center gap-3">
    <div class="grid size-9 place-items-center rounded-full bg-[var(--ink)] text-[var(--paper)]">
      <span class="text-sm font-semibold">T</span>
    </div>
    <div>
      <p class="text-sm font-semibold tracking-[0.16em] uppercase">Tidy</p>
      <p class="text-xs text-[var(--ink-soft)]">Information feed</p>
    </div>
  </div>

  <nav class="mb-6 space-y-1">
    {#each filters as item}
      <button
        class="flex w-full items-center justify-between rounded-xl px-3 py-2 text-left text-sm transition
          {filter === item.id
          ? 'bg-[var(--accent-soft)] text-[var(--accent)]'
          : 'text-[var(--ink-soft)] hover:bg-white/60'}"
        onclick={() => {
          onFilter(item.id);
          onSelectSource(null);
        }}
      >
        <span>{item.label}</span>
      </button>
    {/each}
  </nav>

  <div class="mb-3 flex items-center justify-between">
    <p class="text-xs font-semibold tracking-[0.14em] text-[var(--ink-soft)] uppercase">Sources</p>
    <button
      class="rounded-lg px-2 py-1 text-xs font-semibold text-[var(--accent)] hover:bg-[var(--accent-soft)]"
      onclick={onAddSource}
    >
      Add
    </button>
  </div>

  <div class="min-h-0 flex-1 space-y-1 overflow-y-auto pr-1">
    {#if sources.length === 0}
      <p class="px-2 text-sm leading-6 text-[var(--ink-soft)]">
        Add a blog URL prefix to begin gathering posts.
      </p>
    {:else}
      {#each sources as source}
        <div
          class="group rounded-xl px-3 py-2 transition
            {selectedSourceId === source.id
            ? 'bg-white shadow-[inset_0_0_0_1px_var(--line)]'
            : 'hover:bg-white/55'}"
        >
          <button class="w-full text-left" onclick={() => onSelectSource(source.id)}>
            <p class="truncate text-sm font-medium">{source.title}</p>
            <p class="mt-0.5 text-xs text-[var(--ink-soft)]">
              {source.unread_count} unread · {source.article_count} saved
            </p>
          </button>
          <div class="mt-2 hidden gap-2 group-hover:flex">
            <button
              class="text-xs font-medium text-[var(--accent)]"
              onclick={() => onSelectSource(source.id)}
            >
              Open
            </button>
            <button
              class="text-xs font-medium text-[var(--ink-soft)]"
              onclick={() => onRemoveSource(source.id)}
            >
              Remove
            </button>
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <div class="mt-4 space-y-2 border-t border-[var(--line)] pt-4">
    <button
      class="w-full rounded-xl bg-[var(--ink)] px-3 py-2.5 text-sm font-semibold text-[var(--paper)] disabled:opacity-60"
      onclick={onRefresh}
      disabled={fetching}
    >
      {fetching ? 'Refreshing…' : 'Refresh'}
    </button>
    <button
      class="w-full rounded-xl px-3 py-2 text-sm text-[var(--ink-soft)] hover:bg-white/60"
      onclick={onChangeVault}
    >
      Change vault
    </button>
  </div>
</aside>
