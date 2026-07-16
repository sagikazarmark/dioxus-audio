import { expect, test } from "./fixtures";

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
