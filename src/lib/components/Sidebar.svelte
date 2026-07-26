<script lang="ts">
  import { formatInterval, formatRelative } from '../format';
  import type { FeedFilter, FetchRunRow, SourceRow } from '../types';

  let {
    sources,
    filter,
    selectedSourceId,
    fetching,
    fetchRuns,
    onFilter,
    onSelectSource,
    onAddSource,
    onRefresh,
    onRemoveSource,
    onChangeVault,
    onToggleEnabled,
    onChangeInterval
  }: {
    sources: SourceRow[];
    filter: FeedFilter;
    selectedSourceId: number | null;
    fetching: boolean;
    fetchRuns: FetchRunRow[];
    onFilter: (filter: FeedFilter) => void;
    onSelectSource: (id: number | null) => void;
    onAddSource: () => void;
    onRefresh: () => void;
    onRemoveSource: (id: number) => void;
    onChangeVault: () => void;
    onToggleEnabled: (id: number, enabled: boolean) => void;
    onChangeInterval: (id: number, minutes: number) => void;
  } = $props();

  const filters: { id: FeedFilter; label: string }[] = [
    { id: 'inbox', label: 'Inbox' },
    { id: 'starred', label: 'Starred' },
    { id: 'archived', label: 'Archive' },
    { id: 'all', label: 'All' }
  ];

  const intervalChoices = [
    { label: '1h', value: 60 },
    { label: '6h', value: 360 },
    { label: '12h', value: 720 },
    { label: '1d', value: 1440 },
    { label: '1w', value: 10080 }
  ];

  const selected = $derived(sources.find((source) => source.id === selectedSourceId) ?? null);
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
            : 'hover:bg-white/55'}
            {!source.enabled ? 'opacity-55' : ''}"
        >
          <button class="w-full text-left" onclick={() => onSelectSource(source.id)}>
            <p class="truncate text-sm font-medium">{source.title}</p>
            <p class="mt-0.5 text-xs text-[var(--ink-soft)]">
              {source.unread_count} unread · every {formatInterval(source.interval_minutes)} ·
              {formatRelative(source.last_fetch_at)}
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
              onclick={() => onToggleEnabled(source.id, !source.enabled)}
            >
              {source.enabled ? 'Pause' : 'Resume'}
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

    {#if selected}
      <div class="mt-4 rounded-2xl border border-[var(--line)] bg-white/70 p-3">
        <p class="text-xs font-semibold tracking-[0.12em] text-[var(--ink-soft)] uppercase">
          Schedule
        </p>
        <label class="mt-2 block text-xs text-[var(--ink-soft)]">
          Interval
          <select
            class="mt-1 w-full rounded-lg border border-[var(--line)] bg-white px-2 py-1.5 text-sm"
            value={selected.interval_minutes}
            onchange={(event) =>
              onChangeInterval(selected.id, Number((event.currentTarget as HTMLSelectElement).value))}
          >
            {#each intervalChoices as choice}
              <option value={choice.value}>{choice.label}</option>
            {/each}
          </select>
        </label>
        <p class="mt-2 text-xs text-[var(--ink-soft)]">
          Last fetch {formatRelative(selected.last_fetch_at)}
          {#if !selected.enabled}
            · paused
          {/if}
        </p>

        {#if fetchRuns.length > 0}
          <p class="mt-3 text-xs font-semibold tracking-[0.12em] text-[var(--ink-soft)] uppercase">
            Recent runs
          </p>
          <ul class="mt-1.5 space-y-1.5">
            {#each fetchRuns.slice(0, 5) as run}
              <li class="text-xs leading-5 text-[var(--ink-soft)]">
                <span class="font-medium text-[var(--ink)]">{run.status}</span>
                · +{run.added}/~{run.updated}
                · {formatRelative(run.started_at)}
              </li>
            {/each}
          </ul>
        {/if}
      </div>
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
