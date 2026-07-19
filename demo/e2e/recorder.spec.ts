import { expect, test } from "./fixtures";

test("a recording can be paused, resumed, completed, and played", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      capturedRecorderConstraints?: MediaStreamConstraints[];
    };
    const mediaDevices = navigator.mediaDevices;
    const getUserMedia = mediaDevices.getUserMedia.bind(mediaDevices);
    Object.defineProperty(mediaDevices, "getUserMedia", {
      value: (constraints: MediaStreamConstraints) => {
        (browser.capturedRecorderConstraints ??= []).push(constraints);
        return getUserMedia(constraints);
      },
    });
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });

  await expect(
    demo.getByRole("combobox", { name: "Audio input" }),
  ).toBeVisible();
  await demo.getByRole("button", { name: "Start recording" }).click();

  await expect(
    demo.getByText("Requested sample rate: ideal 48000 Hz"),
  ).toBeVisible();
  const audioConstraints = await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      capturedRecorderConstraints?: MediaStreamConstraints[];
    };
    return browser.capturedRecorderConstraints?.at(-1)?.audio;
  });
  expect(audioConstraints).toMatchObject({
    channelCount: { ideal: 1 },
    sampleRate: { ideal: 48_000 },
    echoCancellation: { ideal: false },
    noiseSuppression: { ideal: false },
    latency: { ideal: 0.02 },
  });
  await demo
    .getByRole("button", { name: "Use 44100 Hz for future recordings" })
    .click();
  await expect(
    demo.getByText("Requested sample rate: ideal 48000 Hz"),
  ).toBeVisible();
  await expect(demo.getByText(/Effective sample rate: \d+ Hz/)).toBeVisible();
  await expect(demo.getByText(/Selected media type: audio\//)).toBeVisible();

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

  await demo.getByRole("button", { name: "Start recording" }).click();
  await expect(
    demo.getByText("Requested sample rate: ideal 44100 Hz"),
  ).toBeVisible();
  await expect
    .poll(async () =>
      page.evaluate(() => {
        const browser = globalThis as typeof globalThis & {
          capturedRecorderConstraints?: MediaStreamConstraints[];
        };
        const audio = browser.capturedRecorderConstraints?.at(-1)?.audio;
        return typeof audio === "object" && audio !== null
          ? (audio as { sampleRate?: { ideal?: number } }).sampleRate?.ideal
          : undefined;
      }),
    )
    .toBe(44_100);
  await demo.getByRole("button", { name: "Cancel recording" }).click();

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

test("recognized constraints remain distinct from unsupported fields", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const mediaDevices = navigator.mediaDevices;
    const supported = mediaDevices.getSupportedConstraints();
    Object.defineProperty(mediaDevices, "getSupportedConstraints", {
      value: () => ({ ...supported, noiseSuppression: false }),
    });
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });

  await expect(demo.getByText("Sample rate: recognized")).toBeVisible();
  await expect(demo.getByText("Noise suppression: unrecognized")).toBeVisible();
});

test("an impossible exact constraint reports structured failure", async ({
  openRoute,
  page,
}) => {
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });

  await demo
    .getByRole("button", { name: "Require impossible sample rate" })
    .click();
  await demo.getByRole("button", { name: "Start recording" }).click();

  await expect(demo.getByRole("alert")).toHaveText(
    "Rejected exact constraint: sampleRate",
  );
});
