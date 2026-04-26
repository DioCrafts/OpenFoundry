<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import NotificationBell from '$components/layout/NotificationBell.svelte';
  import Glyph from '$components/ui/Glyph.svelte';
  import { auth } from '$stores/auth';

  const isAuthenticated = auth.isAuthenticated;
  const user = auth.user;

  const titleMap: Record<string, string> = {
    '/': 'Home',
    '/ontology': 'Ontology',
    '/queries': 'SQL Preview',
    '/datasets': 'Datasets',
    '/pipelines': 'Pipelines'
  };

  let quickSearch = $state('');

  const pageTitle = $derived.by(() => {
    const pathname = $page.url.pathname;
    const sorted = Object.keys(titleMap).sort((a, b) => b.length - a.length);
    const match = sorted.find((key) => pathname === key || pathname.startsWith(`${key}/`));
    return match ? titleMap[match] : 'OpenFoundry';
  });

  function handleLogout() {
    auth.logout();
    goto('/auth/login');
  }
</script>

<header class="border-b border-[var(--border-default)] bg-white">
  <div class="flex h-[58px] items-stretch">
    <a
      href="/search"
      class="flex min-w-[178px] items-center gap-2 border-r border-[var(--border-default)] px-14 text-[15px] font-medium text-[var(--text-strong)] transition hover:bg-[var(--bg-hover)]"
    >
      <span class="of-icon-box h-7 w-7">
        <Glyph name="search" size={15} />
      </span>
      <span>Search for "{quickSearch || 'pass'}"</span>
    </a>

    <a
      href="/ontology"
      class="flex min-w-[172px] items-center gap-2 border-r border-[var(--border-default)] px-10 text-[15px] font-medium text-[var(--text-strong)] transition hover:bg-[var(--bg-hover)]"
    >
      <span class="of-icon-box h-7 w-7 bg-[#eef3fb] text-[var(--text-muted)]">
        <Glyph name="plus" size={14} />
      </span>
      <span>New exploration</span>
    </a>

    <div class="flex min-w-0 flex-1 items-center justify-between gap-6 px-6">
      <div class="min-w-0">
        <div class="truncate text-[15px] font-semibold text-[var(--text-strong)]">{pageTitle}</div>
        <div class="truncate text-xs text-[var(--text-muted)]">
          Enterprise object workflows, search, graph and query surfaces
        </div>
      </div>

      <div class="flex items-center gap-3">
        <label class="of-search-shell min-w-[340px] max-w-[460px]">
          <div class="of-search-input-wrap">
            <Glyph name="search" size={18} />
            <input
              bind:value={quickSearch}
              type="text"
              class="of-search-input"
              placeholder="Search object types, properties and artifacts..."
            />
          </div>
        </label>

        <button type="button" class="of-btn gap-2 px-3 text-[13px]">
          <Glyph name="object" size={16} />
          <span>Explorations</span>
          <Glyph name="chevron-down" size={14} />
        </button>

        <button type="button" class="of-btn gap-2 px-3 text-[13px]">
          <Glyph name="list" size={16} />
          <span>Lists</span>
          <Glyph name="chevron-down" size={14} />
        </button>

        {#if $isAuthenticated}
          <NotificationBell />
          <div class="hidden text-right md:block">
            <div class="text-[13px] font-medium text-[var(--text-strong)]">{$user?.name ?? 'Operator'}</div>
            <div class="text-[11px] text-[var(--text-muted)]">Workspace session</div>
          </div>
          <button type="button" class="of-btn px-3" onclick={handleLogout} aria-label="Logout">
            <Glyph name="logout" size={16} />
          </button>
        {:else}
          <a href="/auth/login" class="of-btn">Login</a>
        {/if}
      </div>
    </div>
  </div>
</header>
