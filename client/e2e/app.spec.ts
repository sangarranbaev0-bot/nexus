// Сквозные тесты NexusChat. Запуск: pnpm e2e (сервер должен работать на http://localhost:3001).
// Другой адрес: set E2E_URL=https://... && pnpm e2e
import { expect, test } from '@playwright/test';

const id = Date.now().toString(36);

test('страница входа открывается в новом дизайне', async ({ page }) => {
  await page.goto('/login');
  await expect(page.getByRole('heading', { name: 'С возвращением!' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Вход' })).toBeVisible();
  const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  expect(bg).toBe('rgb(13, 13, 13)');
});

test('регистрация → свой сервер → сообщение → поиск', async ({ page }) => {
  await page.goto('/register');
  await page.getByLabel('Email').fill(`e2e-${id}@example.test`);
  await page.getByLabel('Имя пользователя').fill(`e2e_${id}`);
  await page.getByLabel('Пароль').fill(`E2e-${id}-pass`);
  await page.getByRole('button', { name: 'Продолжить' }).click();
  await expect(page).toHaveURL(/\/channels/);

  await page.getByRole('button', { name: 'Добавить сервер' }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByLabel('Название сервера').fill(`E2E ${id}`);
  await dialog.locator('button[class]').last().click();
  await expect(dialog).toBeHidden();

  const input = page.getByRole('textbox', { name: /^Сообщение/ });
  await expect(input).toBeVisible();
  const text = `Проверочные сообщения ${id}`;
  await input.fill(text);
  await input.press('Enter');
  await expect(page.getByText(text)).toBeVisible();
});
