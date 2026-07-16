import { expect, test } from "./fixtures";

test("microphone permission reveals the available audio inputs", async ({
  openRoute,
  page,
}) => {
  await openRoute("/devices", "Discover and select microphones");

  await page.getByRole("button", { name: "Request access" }).click();

  await expect(
    page.getByText("permission: Granted", { exact: true }),
  ).toBeVisible();
  await expect(page.getByText("devices: Ready", { exact: true })).toBeVisible();
  await expect(page.getByText(/[1-9]\d* audio input\(s\) found/)).toBeVisible();

  const microphone = page.getByRole("combobox", { name: "Microphone" });
  await expect(microphone).toBeEnabled();
  await expect(microphone.locator("option")).not.toHaveCount(1);
});

test("denied microphone permission is visible and can be retried", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    navigator.mediaDevices.getUserMedia = async () => {
      throw new DOMException("Permission denied by test", "NotAllowedError");
    };
  });
  await openRoute("/devices", "Discover and select microphones");

  const requestAccess = page.getByRole("button", { name: "Request access" });
  await requestAccess.click();

  await expect(
    page.getByText("permission: Denied", { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("alert")).toContainText(
    "Permission denied by test",
  );
  await expect(requestAccess).toBeEnabled();
});
