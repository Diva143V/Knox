import test, { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { Project } from "../types/knox";

function evaluateEffectiveEgress(globalEgress: boolean, projectEgress: boolean | undefined): boolean {
  return globalEgress || Boolean(projectEgress);
}

function isAgentWorkspace(path: string, name: string): boolean {
  return (
    path.includes(".gemini") ||
    path.includes("antigravity\\brain") ||
    path.includes("antigravity/brain") ||
    /^[0-9a-f]{8}-[0-9a-f]{4}/i.test(name)
  );
}

function updateProjectEgressList(projects: Project[], targetId: string, nextVal: boolean): Project[] {
  return projects.map((p) => (p.id === targetId ? { ...p, egress: nextVal } : p));
}

describe("Project Egress & Workspace Detection Logic", () => {
  it("authorizes outbound egress globally when global cloud egress is enabled", () => {
    assert.equal(evaluateEffectiveEgress(true, false), true);
    assert.equal(evaluateEffectiveEgress(true, true), true);
    assert.equal(evaluateEffectiveEgress(true, undefined), true);
  });

  it("restricts egress per-project when global cloud egress is disabled", () => {
    assert.equal(evaluateEffectiveEgress(false, false), false);
    assert.equal(evaluateEffectiveEgress(false, true), true);
    assert.equal(evaluateEffectiveEgress(false, undefined), false);
  });

  it("updates targeted project egress state without mutating other projects", () => {
    const mockProjects: Project[] = [
      { id: "p1", name: "Alpha", path: "/path/to/alpha", created_at: "2026-11-01", egress: false },
      { id: "p2", name: "Beta", path: "/path/to/beta", created_at: "2026-11-02", egress: false },
    ];

    const updated = updateProjectEgressList(mockProjects, "p1", true);
    assert.equal(updated.find((p) => p.id === "p1")?.egress, true);
    assert.equal(updated.find((p) => p.id === "p2")?.egress, false);
  });

  it("detects agent scratch workspaces and raw UUID patterns", () => {
    assert.equal(
      isAgentWorkspace("C:\\Users\\User\\.gemini\\antigravity\\brain\\60c9262f-c815", "60c9262f-c815"),
      true
    );
    assert.equal(
      isAgentWorkspace("D:\\Knox", "Knox"),
      false
    );
    assert.equal(
      isAgentWorkspace("D:\\Alive", "Alive"),
      false
    );
    assert.equal(
      isAgentWorkspace("D:\\Workspace\\custom-repo", "12345678-abcd-1234-abcd-123456789abc"),
      true
    );
  });
});

