import { describe, expect, it } from "vitest";

import { thumbSlots } from "./featured";

describe("thumbSlots", () => {
  it("shows only empty slots before anything is featured", () => {
    expect(thumbSlots([], true, 6)).toEqual([null, null, null, null, null, null]);
  });

  it("fills slots in order while loading and keeps the row full", () => {
    expect(thumbSlots(["a", "b"], true, 6)).toEqual(["a", "b", null, null, null, null]);
  });

  it("drops the empty slots once loading is over", () => {
    expect(thumbSlots(["a", "b"], false, 6)).toEqual(["a", "b"]);
  });

  it("never shows more than count slots", () => {
    expect(thumbSlots(["a", "b", "c", "d"], true, 3)).toEqual(["a", "b", "c"]);
  });
});
