<script lang="ts">
  import { page } from '$app/stores';
  import Glyph from '$components/ui/Glyph.svelte';

  const primaryNav = [
    { href: '/', label: 'Home', icon: 'home' as const },
    { href: '/ontology', label: 'Ontology', icon: 'ontology' as const },
    { href: '/queries', label: 'Queries', icon: 'query' as const },
    { href: '/datasets', label: 'Datasets', icon: 'database' as const },
    { href: '/pipelines', label: 'Pipelines', icon: 'graph' as const },
    { href: '/reports', label: 'Artifacts', icon: 'artifact' as const }
  ];

  const secondaryNav = [
    { href: '/history', label: 'History', icon: 'history' as const },
    { href: '/search', label: 'Search', icon: 'search' as const },
    { href: '/settings', label: 'Settings', icon: 'settings' as const }
  ];

  function isActive(href: string, pathname: string) {
    return href === '/'
      ? pathname === '/'
      : pathname === href || pathname.startsWith(`${href}/`);
  }
</script>

<aside
  class="of-scrollbar flex w-[72px] shrink-0 flex-col border-r border-slate-800 bg-[var(--bg-sidebar)] text-white"
>
  <div class="flex h-[58px] items-center justify-center border-b border-white/10">
    <a
      href="/"
      class="flex h-10 w-10 items-center justify-center rounded-lg bg-white/8 text-white transition hover:bg-white/12"
      aria-label="OpenFoundry home"
      title="OpenFoundry"
    >
      <Glyph name="cube" size={20} />
    </a>
  </div>

  <div class="flex flex-1 flex-col items-center gap-2 px-3 py-4">
    <button type="button" class="of-sidebar-icon-btn" title="Navigation">
      <Glyph name="menu" size={19} />
    </button>

    {#each primaryNav as item}
      <a
        href={item.href}
        class="of-sidebar-icon-btn"
        data-active={isActive(item.href, $page.url.pathname)}
        title={item.label}
        aria-label={item.label}
      >
        <Glyph name={item.icon} size={19} />
      </a>
    {/each}

    <div class="my-2 h-px w-8 bg-white/10"></div>

    {#each secondaryNav as item}
      <a
        href={item.href}
        class="of-sidebar-icon-btn"
        data-active={isActive(item.href, $page.url.pathname)}
        title={item.label}
        aria-label={item.label}
      >
        <Glyph name={item.icon} size={19} />
      </a>
    {/each}
  </div>

  <div class="flex flex-col items-center gap-2 border-t border-white/10 px-3 py-4">
    <a href="/help" class="of-sidebar-icon-btn" title="Help" aria-label="Help">
      <Glyph name="help" size={18} />
    </a>
    <div
      class="flex h-9 w-9 items-center justify-center rounded-full border border-white/12 bg-white/8 text-[11px] font-semibold text-white/90"
      title="Current user"
    >
      OF
    </div>
  </div>
</aside>
