import { describe, expect, it } from "vitest";

import { topicSkeletons } from "./skeleton";

describe("topicSkeletons", () => {
  it("shows every skeleton while nothing has arrived", () => {
    expect(topicSkeletons(true, false, false)).toEqual({ details: true, screenshots: true, releases: true });
  });

  it("keeps the screenshots skeleton once the first page is in but Steam is not", () => {
    expect(topicSkeletons(true, true, false)).toEqual({ details: false, screenshots: true, releases: false });
  });

  it("drops each skeleton as its data arrives", () => {
    expect(topicSkeletons(true, true, true)).toEqual({ details: false, screenshots: false, releases: false });
  });

  it("shows none once loading is over, even with data missing", () => {
    expect(topicSkeletons(false, false, false)).toEqual({ details: false, screenshots: false, releases: false });
  });
});
