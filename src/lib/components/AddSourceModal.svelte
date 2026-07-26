<script lang="ts">
  let {
    open,
    busy,
    onClose,
    onSubmit
  }: {
    open: boolean;
    busy: boolean;
    onClose: () => void;
    onSubmit: (payload: {
      url_prefix: string;
      title?: string;
      backfill: 'recent' | 'full';
      recent_limit: number;
    }) => void;
  } = $props();

  let url = $state('');
  let title = $state('');
  let backfill = $state<'recent' | 'full'>('recent');
  let recentLimit = $state(20);

  function submit(event: Event) {
    event.preventDefault();
    if (!url.trim()) return;
    onSubmit({
      url_prefix: url.trim(),
      title: title.trim() || undefined,
      backfill,
      recent_limit: recentLimit
    });
  }
</script>

{#if open}
  <div class="fixed inset-0 z-50 grid place-items-center bg-[rgb(19_32_51/45%)] p-4">
    <form
      class="w-full max-w-lg rounded-3xl border border-[var(--line)] bg-[var(--paper)] p-6 shadow-2xl"
      onsubmit={submit}
    >
      <h2 class="font-[family-name:var(--font-reader)] text-3xl tracking-[-0.03em]">Add a source</h2>
      <p class="mt-2 text-sm leading-6 text-[var(--ink-soft)]">
        Paste a URL prefix such as <code class="rounded bg-black/5 px-1">example.com/blog</code>.
        Tidy will discover posts under that path.
      </p>

      <label class="mt-5 block text-sm font-medium">
        URL prefix
        <input
          class="mt-1.5 w-full rounded-xl border border-[var(--line)] bg-white px-3 py-2.5"
          placeholder="https://example.com/articles"
          bind:value={url}
          required
        />
      </label>

      <label class="mt-4 block text-sm font-medium">
        Display name <span class="font-normal text-[var(--ink-soft)]">(optional)</span>
        <input
          class="mt-1.5 w-full rounded-xl border border-[var(--line)] bg-white px-3 py-2.5"
          placeholder="Example Blog"
          bind:value={title}
        />
      </label>

      <fieldset class="mt-4">
        <legend class="text-sm font-medium">Backfill</legend>
        <div class="mt-2 space-y-2 text-sm">
          <label class="flex items-center gap-2">
            <input type="radio" name="backfill" value="recent" bind:group={backfill} />
            Recent posts only
          </label>
          <label class="flex items-center gap-2">
            <input type="radio" name="backfill" value="full" bind:group={backfill} />
            Full archive discovery
          </label>
        </div>
      </fieldset>

      {#if backfill === 'recent'}
        <label class="mt-4 block text-sm font-medium">
          Recent limit
          <input
            class="mt-1.5 w-28 rounded-xl border border-[var(--line)] bg-white px-3 py-2.5"
            type="number"
            min="1"
            max="200"
            bind:value={recentLimit}
          />
        </label>
      {/if}

      <div class="mt-6 flex justify-end gap-2">
        <button
          type="button"
          class="rounded-xl px-4 py-2.5 text-sm text-[var(--ink-soft)] hover:bg-white"
          onclick={onClose}
          disabled={busy}
        >
          Cancel
        </button>
        <button
          type="submit"
          class="rounded-xl bg-[var(--ink)] px-4 py-2.5 text-sm font-semibold text-[var(--paper)]"
          disabled={busy}
        >
          {busy ? 'Fetching…' : 'Add & fetch'}
        </button>
      </div>
    </form>
  </div>
{/if}
