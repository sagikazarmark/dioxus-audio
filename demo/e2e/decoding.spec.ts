import { expect, test } from "./fixtures";

type DecodingBrowserState = {
  closeRequests: number;
  decodeSampleRates: number[];
  resolvePending?: (buffer: AudioBuffer) => void;
};

test("complete Audio Data decodes with effective metadata and settled cleanup", async ({
  openRoute,
  page,
}) => {
  await installDecodeObserver(page);
  await openRoute("/decoding", "Decode complete audio into planar samples");

  const result = page.getByRole("status", { name: "Decoded Audio result" });
  await page
    .getByRole("button", { name: "Decode generated stereo WAV" })
    .click();
  await expect(result.getByText("Decoded Audio ready")).toBeVisible();
  await expect(result.getByText("Channels: 2")).toBeVisible();
  await expect(result.getByText("Channel views: 2")).toBeVisible();
  await expect(result.getByText(/Frames per channel: [1-9]\d*/)).toBeVisible();
  await expect(result.getByText(/Duration: 0\.\d{6} s/)).toBeVisible();
  await expect(result.getByText("Encoded source rate: 8000 Hz")).toBeVisible();

  const channel0Peak = Number(
    (await result.getByTestId("channel-0-peak").textContent())?.split(": ")[1],
  );
  const channel1Peak = Number(
    (await result.getByTestId("channel-1-peak").textContent())?.split(": ")[1],
  );
  expect(channel0Peak).toBeGreaterThan(0.1);
  expect(channel1Peak).toBeGreaterThan(channel0Peak * 2);

  const reportedRate = Number(
    (await result.getByTestId("effective-sample-rate").textContent())?.match(
      /\d+/,
    )?.[0],
  );
  const state = await decodingState(page);
  expect(reportedRate).toBe(state.decodeSampleRates[0]);
  expect(state.closeRequests).toBe(1);

  await page
    .getByRole("button", { name: "Decode malformed Audio Data" })
    .click();
  await expect(result.getByText("Decode failed: decode rejected")).toBeVisible();
  await expect.poll(async () => (await decodingState(page)).closeRequests).toBe(2);

  await page
    .getByRole("button", { name: "Decode with zero-byte limit" })
    .click();
  await expect(result.getByText("Decode failed: resource limit")).toBeVisible();
  await expect(
    result.getByText(/Required bytes: [1-9]\d*; configured bytes: 0/),
  ).toBeVisible();
  await expect.poll(async () => (await decodingState(page)).closeRequests).toBe(3);
});

test("dropping a pending decode requests cleanup and suppresses publication", async ({
  openRoute,
  page,
}) => {
  await installDecodeObserver(page, true);
  await openRoute("/decoding", "Decode complete audio into planar samples");

  const result = page.getByRole("status", { name: "Decoded Audio result" });
  await page
    .getByRole("button", { name: "Decode generated stereo WAV" })
    .click();
  await expect(result.getByText("Decoding complete Audio Data")).toBeVisible();
  await page.getByRole("button", { name: "Unmount decode controls" }).click();
  await expect.poll(async () => (await decodingState(page)).closeRequests).toBe(1);

  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      decodingState?: DecodingBrowserState;
    };
    const sampleRate = browser.decodingState!.decodeSampleRates[0];
    browser.decodingState!.resolvePending?.(
      new AudioBuffer({ length: 1, numberOfChannels: 1, sampleRate }),
    );
  });
  await page.waitForTimeout(100);
  await expect(result.getByText("Decoding complete Audio Data")).toBeVisible();
  await expect(result.getByText("Decoded Audio ready")).toHaveCount(0);
});

async function installDecodeObserver(
  page: import("@playwright/test").Page,
  holdDecode = false,
) {
  await page.addInitScript((holdDecode) => {
    const browser = globalThis as typeof globalThis & {
      decodingState?: DecodingBrowserState;
    };
    const state: DecodingBrowserState = {
      closeRequests: 0,
      decodeSampleRates: [],
    };
    browser.decodingState = state;

    const close = AudioContext.prototype.close;
    AudioContext.prototype.close = function () {
      state.closeRequests += 1;
      return close.call(this);
    };
    const decode = AudioContext.prototype.decodeAudioData;
    AudioContext.prototype.decodeAudioData = function (buffer: ArrayBuffer) {
      state.decodeSampleRates.push(this.sampleRate);
      if (holdDecode) {
        return new Promise<AudioBuffer>((resolve) => {
          state.resolvePending = resolve;
        });
      }
      return decode.call(this, buffer);
    };
  }, holdDecode);
}

async function decodingState(page: import("@playwright/test").Page) {
  return page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      decodingState?: DecodingBrowserState;
    };
    return browser.decodingState!;
  });
}
