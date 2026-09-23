import { test, expect } from "@playwright/test";

test("detects grants made in System Settings without a request-button click", async ({
  page,
}) => {
  await page.goto("/tests/permissions.html");
  await expect(
    page.getByRole("heading", { name: "Allow dictation on this Mac." }),
  ).toBeVisible();
  await page.evaluate(() => {
    window.permissionTest.accessibility = true;
    window.permissionTest.microphone = true;
    window.dispatchEvent(new Event("focus"));
  });
  await expect(page.getByRole("status")).toHaveText("Permissions passed");
  expect(await page.evaluate(() => window.permissionTest.requests)).toBe(0);
  expect(await page.evaluate(() => window.permissionTest.completions)).toBe(1);
});

test("a returned request does not count as a grant and recovery remains available", async ({
  page,
}) => {
  await page.goto("/tests/permissions.html");
  await page.getByRole("button", { name: "Grant Permission" }).last().click();
  await expect(page.getByText("Waiting...", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Check again" })).toBeVisible({
    timeout: 10000,
  });
  await page.getByRole("button", { name: "Check again" }).click();
  expect(await page.evaluate(() => window.permissionTest.completions)).toBe(0);
  await page.getByRole("button", { name: "Restart hushtype" }).click();
  await expect
    .poll(() => page.evaluate(() => window.permissionTest.restarts))
    .toBe(1);
  await page.evaluate(() => {
    window.permissionTest.accessibility = true;
    window.permissionTest.microphone = true;
  });
  await expect(page.getByRole("status")).toHaveText("Permissions passed");
});
