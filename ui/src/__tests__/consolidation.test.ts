import test, { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { StrayDatabaseDto, ConsolidationReport } from "../types/knox";

interface PromptEvaluation {
  shouldPrompt: boolean;
  autoMarkPrompted: boolean;
}

function evaluateStrayPromptState(
  configPrompted: boolean | undefined,
  detectedStrays: StrayDatabaseDto[]
): PromptEvaluation {
  if (configPrompted) {
    return { shouldPrompt: false, autoMarkPrompted: false };
  }
  const totalActions = detectedStrays.reduce((acc, s) => acc + s.action_count, 0);
  if (detectedStrays.length > 0 && totalActions > 0) {
    return { shouldPrompt: true, autoMarkPrompted: false };
  }
  return { shouldPrompt: false, autoMarkPrompted: true };
}

function aggregateConsolidationResults(reports: ConsolidationReport[]): {
  totalActions: number;
  totalEdges: number;
  totalSessions: number;
} {
  return reports.reduce(
    (acc, rep) => ({
      totalActions: acc.totalActions + rep.actions_moved,
      totalEdges: acc.totalEdges + rep.edges_moved,
      totalSessions: acc.totalSessions + rep.sessions_moved,
    }),
    { totalActions: 0, totalEdges: 0, totalSessions: 0 }
  );
}

describe("Stray Database Prompt & Consolidation Logic", () => {
  it("prompts user when stray databases exist and user has not been prompted", () => {
    const mockStrays: StrayDatabaseDto[] = [
      {
        project_path: "/workspace/alpha",
        db_path: "/workspace/alpha/.knox/knox.db",
        action_count: 14,
      },
    ];

    const evalResult = evaluateStrayPromptState(false, mockStrays);
    assert.equal(evalResult.shouldPrompt, true);
    assert.equal(evalResult.autoMarkPrompted, false);
  });

  it("does not prompt and automatically marks prompted when no stray databases exist", () => {
    const evalResult = evaluateStrayPromptState(false, []);
    assert.equal(evalResult.shouldPrompt, false);
    assert.equal(evalResult.autoMarkPrompted, true);
  });

  it("does not prompt and automatically marks prompted when detected strays have 0 actions", () => {
    const mockStrays: StrayDatabaseDto[] = [
      {
        project_path: "/workspace/empty-a",
        db_path: "/workspace/empty-a/.knox/knox.db",
        action_count: 0,
      },
      {
        project_path: "/workspace/empty-b",
        db_path: "/workspace/empty-b/.knox/knox.db",
        action_count: 0,
      },
    ];

    const evalResult = evaluateStrayPromptState(false, mockStrays);
    assert.equal(evalResult.shouldPrompt, false);
    assert.equal(evalResult.autoMarkPrompted, true);
  });

  it("never prompts if config already records stray_db_prompted as true", () => {
    const mockStrays: StrayDatabaseDto[] = [
      {
        project_path: "/workspace/beta",
        db_path: "/workspace/beta/.knox/knox.db",
        action_count: 5,
      },
    ];

    const evalResult = evaluateStrayPromptState(true, mockStrays);
    assert.equal(evalResult.shouldPrompt, false);
    assert.equal(evalResult.autoMarkPrompted, false);
  });

  it("accurately aggregates consolidation metrics across multiple stray imports", () => {
    const mockReports: ConsolidationReport[] = [
      {
        source_path: "/workspace/a/.knox/knox.db",
        target_path: "~/.knox/knox.db",
        projects_moved: 1,
        sessions_moved: 2,
        actions_moved: 12,
        actions_skipped: 1,
        edges_moved: 8,
      },
      {
        source_path: "/workspace/b/.knox/knox.db",
        target_path: "~/.knox/knox.db",
        projects_moved: 1,
        sessions_moved: 1,
        actions_moved: 7,
        actions_skipped: 0,
        edges_moved: 4,
      },
    ];

    const aggregated = aggregateConsolidationResults(mockReports);
    assert.equal(aggregated.totalActions, 19);
    assert.equal(aggregated.totalEdges, 12);
    assert.equal(aggregated.totalSessions, 3);
  });
});
