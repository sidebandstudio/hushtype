import { test, expect } from "@playwright/test";

test("each mode saves its chosen style", async ({ page }) => {
  await page.goto("/tests/modes.html");
  await expect(page.getByText("2 app or URL matchers · Casual")).toBeVisible();
  await expect(page.getByText("Everywhere else · Serious")).toBeVisible();

  await page.getByText("Messaging", { exact: true }).click();
  const messaging = page.locator("details[open]");
  await expect(messaging.getByRole("radio", { name: /^Casual/ })).toBeChecked();
  await messaging.getByText("Very casual", { exact: true }).click();
  await expect(
    messaging.getByRole("radio", { name: /^Very casual/ }),
  ).toBeChecked();
  await page.getByRole("button", { name: "Save changes" }).click();

  await expect
    .poll(() => page.evaluate(() => window.savedModes?.map((m) => m.style)))
    .toEqual(["very_casual", "serious", "serious"]);
  await expect(
    page.getByText("2 app or URL matchers · Very casual"),
  ).toBeVisible();
});
