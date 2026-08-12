import { expect, test } from "@playwright/test";

import { expectDecodedPayloadBytes } from "./zxing";

test("decoder comparison preserves payload boundary whitespace", () => {
  const payload = "  line one\nline two\n";
  expectDecodedPayloadBytes(Buffer.from(payload, "utf8"), payload);
});

test("decoder comparison rejects missing or additional boundary bytes", () => {
  expect(() => expectDecodedPayloadBytes(Buffer.from(" padded "), "padded ")).toThrow();
  expect(() => expectDecodedPayloadBytes(Buffer.from("padded \n"), "padded ")).toThrow();
});
