<script lang="ts">
  import { page } from '$app/stores';
</script>

<div class="flex flex-col items-center justify-center min-h-[60vh] text-center">
  <h1 class="text-6xl font-bold text-gray-300 dark:text-gray-700">{$page.status}</h1>
  <p class="text-lg text-gray-500 mt-4">{$page.error?.message ?? 'Something went wrong'}</p>
  <a href="/" class="mt-6 text-indigo-600 hover:text-indigo-500">Go home</a>
</div>
