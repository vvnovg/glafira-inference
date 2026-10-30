import type { ChapterId } from "../../guide/chapters";
import type { TabSpec } from "../../state/workspace";

/** Глава руководства для экрана (F1). */
export function helpChapterFor(tab: TabSpec | null): ChapterId {
  switch (tab?.kind) {
    case "installation-form":
    case "installation":
      return "installations";
    case "server":
      return "servers";
    case "about":
      return "updates";
    default:
      return "getting-started";
  }
}
