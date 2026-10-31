/**
 * Graphite Vault semantic color maps for action types and actors.
 * Constrained to design tokens / 6-swatch palette — no ad-hoc purple scatter.
 */

/** Badge surface classes: muted fill + foreground + border. */
export function actionTypeClasses(type: string): string {
  const norm = (type || "").toLowerCase();

  if (norm.includes("write")) {
    return "bg-success/15 text-success border-success/30";
  }
  if (norm.includes("plan")) {
    return "bg-plan/15 text-plan border-plan/30";
  }
  if (norm.includes("decide")) {
    return "bg-warning/15 text-warning border-warning/30";
  }
  if (norm.includes("debug")) {
    return "bg-danger/15 text-danger border-danger/30";
  }
  return "bg-primary/15 text-primary border-primary/30";
}

/** Actor badge classes mapped to the 6-swatch vault palette. */
export function actorClasses(actor: string): string {
  const norm = (actor || "").toLowerCase();

  switch (norm) {
    case "human":
      return "bg-swatch-d/15 text-swatch-d border-swatch-d/30";
    case "gemini":
      return "bg-swatch-b/15 text-swatch-b border-swatch-b/30";
    case "cursor":
      return "bg-swatch-c/15 text-swatch-c border-swatch-c/30";
    case "antigravity":
      return "bg-swatch-a/15 text-swatch-a border-swatch-a/30";
    case "copilot":
      return "bg-swatch-e/15 text-swatch-e border-swatch-e/30";
    default:
      return "bg-swatch-f/15 text-swatch-f border-swatch-f/30";
  }
}

/** Distribution bar fill colors (stats charts). */
export function actorBarClass(actor: string): string {
  const norm = (actor || "").toLowerCase();
  switch (norm) {
    case "human":
      return "bg-swatch-d";
    case "gemini":
      return "bg-swatch-b";
    case "cursor":
      return "bg-swatch-c";
    case "antigravity":
      return "bg-swatch-a";
    case "copilot":
      return "bg-swatch-e";
    default:
      return "bg-swatch-f";
  }
}

export function actionTypeBarClass(type: string): string {
  const norm = (type || "").toLowerCase();
  if (norm.includes("write")) return "bg-success";
  if (norm.includes("plan")) return "bg-plan";
  if (norm.includes("decide")) return "bg-warning";
  if (norm.includes("debug")) return "bg-danger";
  return "bg-primary";
}
