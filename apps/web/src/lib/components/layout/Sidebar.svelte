<script lang="ts">
  import { page } from '$app/stores';

  const nav = [
    { href: '/',          label: 'Home',       icon: '🏠' },
    { href: '/apps',      label: 'Apps',       icon: '🧩' },
    { href: '/ai',        label: 'AI Platform', icon: '🛰️' },
    { href: '/fusion',    label: 'Fusion',     icon: '🧬' },
    { href: '/streaming', label: 'Streaming',  icon: '🌊' },
    { href: '/reports',   label: 'Reports',    icon: '🧾' },
    { href: '/control-panel', label: 'Control Panel', icon: '🛠️' },
    { href: '/geospatial', label: 'Geospatial', icon: '🗺️' },
    { href: '/audit',     label: 'Audit',      icon: '🛡️' },
    { href: '/nexus',     label: 'Nexus',      icon: '🤝' },
    { href: '/code-repos', label: 'Code Repos', icon: '🧱' },
    { href: '/developers', label: 'Developers', icon: '</>' },
    { href: '/marketplace', label: 'Marketplace', icon: '🛍️' },
    { href: '/ml',        label: 'ML Studio',  icon: '🧠' },
    { href: '/contour',   label: 'Contour',    icon: '🧿' },
    { href: '/quiver',    label: 'Quiver',     icon: '📈' },
    { href: '/dashboards', label: 'Dashboards', icon: '📊' },
    { href: '/datasets',  label: 'Datasets',   icon: '🗄️' },
    { href: '/workflows', label: 'Workflows',  icon: '🧭' },
    { href: '/queries',   label: 'Queries',    icon: '🔍' },
    { href: '/pipelines', label: 'Pipelines',  icon: '⚙️' },
    { href: '/ontology',  label: 'Ontology',   icon: '🔗' },
    { href: '/notebooks', label: 'Notebooks',  icon: '📓' },
    { href: '/notepad',   label: 'Notepad',    icon: '📝' },
    { href: '/settings',  label: 'Settings',   icon: '⚙️' },
  ];
</script>

<aside class="w-60 bg-white dark:bg-gray-900 border-r border-gray-200 dark:border-gray-800 flex flex-col">
  <div class="p-4 border-b border-gray-200 dark:border-gray-800">
    <a href="/" class="flex items-center gap-2 text-lg font-bold">
      <span class="text-indigo-500">◆</span>
      <span>OpenFoundry</span>
    </a>
  </div>

  <nav class="flex-1 p-3 space-y-1">
    {#each nav as item}
      <a
        href={item.href}
        class="flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors
               {$page.url.pathname === item.href || ($page.url.pathname.startsWith(item.href) && item.href !== '/')
                 ? 'bg-indigo-50 dark:bg-indigo-950 text-indigo-700 dark:text-indigo-300 font-medium'
                 : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800'}"
      >
        <span class="text-base">{item.icon}</span>
        <span>{item.label}</span>
      </a>
    {/each}
  </nav>

  <div class="p-3 border-t border-gray-200 dark:border-gray-800 text-xs text-gray-400">
    OpenFoundry v0.1.0
  </div>
</aside>
