import { expect, test } from '@playwright/test';

test('home page renders with the OpenFoundry title', async ({ page }) => {
  await page.goto('/');

  await expect(page).toHaveTitle(/OpenFoundry/);
  await expect(page.locator('body')).toBeVisible();
});