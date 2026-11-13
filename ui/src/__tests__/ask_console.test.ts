import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { DemoAPI } from "../services/demo.ts";

describe("AI Brainstorm & Query Console Logic", () => {
  it("executes natural language question and returns grounded synthesis", async () => {
    const res = await DemoAPI.askKnox("Why did we choose SQLite over Postgres?");
    assert.ok(res.answer.includes("SQLite"));
    assert.strictEqual(res.grounded, true);
    assert.ok(res.candidates_evaluated > 0);
    assert.ok(res.citations.length >= 2);
    assert.strictEqual(res.citations[0].action_id, "a-001");
  });

  it("handles doubts inquiry with relevant unresolved citations", async () => {
    const res = await DemoAPI.askKnox("What are the open doubts in this project?");
    assert.ok(res.answer.toLowerCase().includes("doubt"));
    assert.ok(res.citations.some((c) => c.action_id === "a-003"));
  });

  it("records brainstorm takeaway as a persisted quick thought", async () => {
    const thoughtId = await DemoAPI.recordQuickThought({
      summary: "[Brainstorm] Why SQLite?",
      category: "thought",
      contextSummary: "SQLite provides embedded zero-daemon ACID.",
      actor: "human",
    });

    assert.ok(thoughtId.startsWith("mock-action-") || thoughtId.startsWith("act-"));
  });

  it("accurately parses inline citation tokens for clickable badges", () => {
    const sampleText = "We chose SQLite [act-a001] in WAL mode [act-a002] for local speed.";
    const citationMatches = sampleText.match(/\[act-([a-zA-Z0-9_\-]+)\]/g);
    assert.ok(citationMatches);
    assert.strictEqual(citationMatches.length, 2);
    assert.strictEqual(citationMatches[0], "[act-a001]");
    assert.strictEqual(citationMatches[1], "[act-a002]");
  });
});
