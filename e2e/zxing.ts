import { spawnSync } from "node:child_process";

import { expect } from "@playwright/test";

export function expectDecodedPayload(reader: string, artifact: string, payload: string): void {
  const result = spawnSync(reader, ["-formats", "QRCode", "-single", "-bytes", artifact]);
  expect(result.status, result.stderr.toString("utf8")).toBe(0);
  expectDecodedPayloadBytes(result.stdout, payload);
}

export function expectDecodedPayloadBytes(decoded: Uint8Array, payload: string): void {
  expect(Buffer.from(decoded)).toEqual(Buffer.from(payload, "utf8"));
}
