import { describe, expect, it } from "vitest";
import type { TabSpec } from "../../state/workspace";
import { helpChapterFor } from "./helpChapterFor";

const cases: [TabSpec | null, string][] = [
  [{ kind: "installation-form", installationId: null }, "installations"],
  [{ kind: "installation", installationId: "i" }, "installations"],
  [{ kind: "server", installationId: "i", serverId: "s" }, "servers"],
  [{ kind: "about" }, "updates"],
  [{ kind: "settings" }, "getting-started"],
  [{ kind: "help", chapter: "installations" }, "getting-started"],
  [null, "getting-started"],
];

describe("helpChapterFor", () => {
  it.each(cases)("%j → %s", (tab, chapter) => {
    expect(helpChapterFor(tab)).toBe(chapter);
  });
});
