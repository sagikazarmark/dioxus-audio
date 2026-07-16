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

test("a recording can be paused, resumed, completed, and played", async ({
  openRoute,
  page,
}) => {
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });

  await expect(
    demo.getByRole("combobox", { name: "Audio input" }),
  ).toBeVisible();
  await demo.getByRole("button", { name: "Start recording" }).click();

  const pause = demo.getByRole("button", { name: "Pause", exact: true });
  await expect(pause).toBeVisible();
  await pause.click();
  await expect(
    demo.getByText("Recording paused", { exact: true }),
  ).toBeVisible();

  await demo.getByRole("button", { name: "Resume", exact: true }).click();
  await expect(pause).toBeVisible();
  await page.waitForTimeout(1_000);
  await demo.getByRole("button", { name: "Stop recording" }).click();

  const completed = demo
    .getByRole("status")
    .filter({ hasText: "Recording ready" });
  await expect(completed).toContainText(
    /\d+:\d{2} \| audio\/[^|]+ \| [1-9]\d* bytes/,
  );

  const waveform = demo.getByRole("img", { name: "Recorded waveform" });
  await expect(waveform).toBeVisible();
  await expect(waveform.locator("rect").first()).toBeVisible();

  await demo.getByRole("button", { name: "Play", exact: true }).click();
  await expect(
    demo.getByRole("button", { name: "Pause", exact: true }),
  ).toBeVisible();
});

test("a cancelled recording is discarded and another can be completed", async ({
  openRoute,
  page,
}) => {
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });
  const start = demo.getByRole("button", { name: "Start recording" });
  const completed = demo
    .getByRole("status")
    .filter({ hasText: "Recording ready" });

  await start.click();
  await demo.getByRole("button", { name: "Cancel recording" }).click();

  await expect(start).toBeVisible();
  await expect(completed).toHaveCount(0);

  await start.click();
  const stop = demo.getByRole("button", { name: "Stop recording" });
  await expect(stop).toBeVisible();
  await page.waitForTimeout(500);
  await stop.click();

  await expect(completed).toHaveCount(1);
  await expect(completed).toContainText("Recording ready");
});
