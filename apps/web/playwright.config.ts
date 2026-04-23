import { defineConfig } from '@playwright/test';

export default defineConfig({
	testDir: './tests/e2e',
	fullyParallel: true,
	retries: process.env.CI ? 2 : 0,
	use: {
		baseURL: 'http://127.0.0.1:4173',
		trace: 'on-first-retry',
	},
	webServer: {
		command: 'pnpm run build && pnpm exec vite preview --host 127.0.0.1 --port 4173',
		port: 4173,
		reuseExistingServer: !process.env.CI,
		timeout: 120000,
	},
});
