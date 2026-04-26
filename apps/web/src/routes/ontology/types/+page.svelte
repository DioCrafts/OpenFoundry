<script lang="ts">
  import { createObjectType } from '$lib/api/ontology';
  import { goto } from '$app/navigation';
  import Glyph from '$components/ui/Glyph.svelte';

  let name = $state('');
  let displayName = $state('');
  let description = $state('');
  let icon = $state('');
  let color = $state('#4d8cf0');
  let saving = $state(false);
  let error = $state('');

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (!name.trim()) {
      error = 'Name is required';
      return;
    }
    saving = true;
    error = '';
    try {
      await createObjectType({
        name: name.trim(),
        display_name: displayName.trim() || undefined,
        description: description.trim() || undefined,
        icon: icon.trim() || undefined,
        color: color || undefined
      });
      goto('/ontology');
    } catch (err: any) {
      error = err.message || 'Failed to create object type';
    } finally {
      saving = false;
    }
  }
</script>

<div class="mx-auto max-w-5xl space-y-5">
  <section class="of-hero-strip">
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div>
        <div class="of-heading-xl">Create object type</div>
        <div class="mt-2 max-w-3xl text-[15px] text-[var(--text-muted)]">
          Define metadata, identifier, visual treatment and semantic description before wiring properties
          and links.
        </div>
      </div>
      <a href="/ontology" class="of-btn">
        <Glyph name="chevron-right" size={16} />
        <span>Back to ontology</span>
      </a>
    </div>
  </section>

  {#if error}
    <div class="of-inline-note">{error}</div>
  {/if}

  <form onsubmit={handleSubmit} class="of-panel overflow-hidden">
    <div class="flex items-center justify-between border-b border-[var(--border-subtle)] px-5 py-4">
      <div class="of-heading-sm">Metadata</div>
      <div class="text-xs text-[var(--text-muted)]">Object type editor</div>
    </div>

    <div class="space-y-5 p-5">
      <div class="grid gap-5 lg:grid-cols-[120px_minmax(0,1fr)]">
        <div class="space-y-2">
          <div
            class="flex h-[86px] w-[86px] items-center justify-center rounded-[8px] text-white shadow-sm"
            style={`background:${color}`}
          >
            {#if icon.trim()}
              <span class="text-3xl">{icon}</span>
            {:else}
              <Glyph name="cube" size={34} />
            {/if}
          </div>
          <input id="object-type-color" type="color" bind:value={color} class="h-10 w-[86px] cursor-pointer rounded-[6px] border border-[var(--border-default)] bg-white p-1" />
        </div>

        <div class="grid gap-4 md:grid-cols-2">
          <div>
            <label for="object-type-display-name" class="mb-1 block text-sm font-medium text-[var(--text-default)]">
              Name and icon
            </label>
            <input
              id="object-type-display-name"
              type="text"
              bind:value={displayName}
              placeholder="Customer Invoice"
              class="of-input"
            />
          </div>

          <div>
            <label for="object-type-plural-name" class="mb-1 block text-sm font-medium text-[var(--text-default)]">
              Plural name
            </label>
            <input
              id="object-type-plural-name"
              type="text"
              placeholder="Customer Invoices"
              class="of-input"
            />
          </div>

          <div>
            <label for="object-type-name" class="mb-1 block text-sm font-medium text-[var(--text-default)]">
              Object type ID
            </label>
            <input
              id="object-type-name"
              type="text"
              bind:value={name}
              required
              placeholder="customer-invoice"
              class="of-input font-mono"
            />
          </div>

          <div>
            <label for="object-type-icon" class="mb-1 block text-sm font-medium text-[var(--text-default)]">
              Icon / glyph
            </label>
            <input id="object-type-icon" type="text" bind:value={icon} placeholder="📄" class="of-input" />
          </div>
        </div>
      </div>

      <div>
        <label for="object-type-description" class="mb-1 block text-sm font-medium text-[var(--text-default)]">
          Description
        </label>
        <textarea
          id="object-type-description"
          bind:value={description}
          rows={4}
          placeholder="Example object type created by your organization and used in exploration, graph and action flows."
          class="of-textarea"
        ></textarea>
      </div>

      <div class="rounded-[6px] border border-[var(--border-default)] bg-[#fbfcfe]">
        <div class="flex items-center justify-between border-b border-[var(--border-subtle)] px-4 py-3">
          <div class="of-heading-sm">Properties</div>
          <div class="flex gap-2">
            <button type="button" class="of-btn">
              <Glyph name="plus" size={16} />
              <span>Add property</span>
            </button>
            <button type="button" class="of-btn">
              <Glyph name="search" size={16} />
              <span>Search properties</span>
            </button>
          </div>
        </div>

        <div class="space-y-3 p-4">
          {#each [
            { left: 'Invoice ID', right: 'Primary key', tone: '#d5b14b' },
            { left: 'Invoice Name', right: 'Title', tone: '#507dbd' },
            { left: 'Embeddings', right: 'Vector', tone: '#3f7be0' }
          ] as row}
            <div class="grid gap-3 md:grid-cols-[minmax(0,1fr)_48px_minmax(0,1fr)_auto]">
              <div class="of-input flex items-center gap-2 bg-white">
                <Glyph name="object" size={15} />
                <span>{row.left}</span>
              </div>
              <div class="flex items-center justify-center text-[var(--text-soft)]">→</div>
              <div class="of-input flex items-center gap-2 bg-white">
                <span class="flex h-7 min-w-7 items-center justify-center rounded-[4px] text-white" style={`background:${row.tone}`}>
                  <Glyph name="cube" size={14} />
                </span>
                <span>{row.right}</span>
              </div>
              <div class="flex items-center gap-2">
                <span class="of-chip of-status-success">mapped</span>
                <button type="button" class="of-btn h-[38px] w-[38px] px-0">…</button>
              </div>
            </div>
          {/each}
        </div>
      </div>

      <div class="flex justify-end gap-3 border-t border-[var(--border-subtle)] pt-5">
        <a href="/ontology" class="of-btn">Cancel</a>
        <button type="submit" disabled={saving} class="of-btn of-btn-primary">
          <Glyph name="plus" size={16} />
          <span>{saving ? 'Creating...' : 'Create type'}</span>
        </button>
      </div>
    </div>
  </form>
</div>
