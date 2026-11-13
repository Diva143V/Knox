import React, { useState, useEffect } from "react";
import {
  BrainCircuit,
  Server,
  Play,
  Sparkles,
  Lock,
  CheckCircle2,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { KnoxAPI } from "@/services/api";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { cn } from "@/lib/cn";
import type { ModelTestResultDto, Project } from "@/types/knox";

export const ModelTab: React.FC = () => {
  const { config, updateConfig, showToast } = useKnoxStore();

  const [profile, setProfile] = useState<string>("off");
  const [endpoint, setEndpoint] = useState<string>("");
  const [apiKey, setApiKey] = useState<string>("");
  const [embeddingModel, setEmbeddingModel] = useState<string>("");
  const [chatModel, setChatModel] = useState<string>("");
  const [allowCloudEgress, setAllowCloudEgress] = useState<boolean>(false);

  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<ModelTestResultDto | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);

  useEffect(() => {
    if (config) {
      setProfile(config.model_profile || "off");
      setEndpoint(config.model_endpoint || "");
      setApiKey(config.model_api_key_masked || "");
      setEmbeddingModel(config.embedding_model || "");
      setChatModel(config.chat_model || "");
      setAllowCloudEgress(Boolean(config.allow_cloud_egress));
    }
  }, [config]);

  useEffect(() => {
    KnoxAPI.getProjects()
      .then(setProjects)
      .catch((e) => console.warn("Failed to load projects for egress:", e));
  }, []);

  const handleProfileSelect = async (newProfile: string) => {
    setProfile(newProfile);
    try {
      await updateConfig({ modelProfile: newProfile });
      showToast(`Model profile updated to ${newProfile.toUpperCase()}`);
    } catch (e) {
      console.warn("Failed to auto-save profile:", e);
    }
  };

  const handleToggleCloudEgress = async (val: boolean) => {
    setAllowCloudEgress(val);
    try {
      await updateConfig({ allowCloudEgress: val });
      showToast(
        val
          ? "Global cloud egress authorized: outbound requests permitted across all projects."
          : "Global cloud egress revoked: repositories restricted by per-project policy."
      );
    } catch (e) {
      showToast(String(e), true);
    }
  };

  const handleFieldBlur = async () => {
    try {
      await updateConfig({
        modelEndpoint: endpoint.trim() || null,
        modelApiKey: apiKey.trim() || null,
        embeddingModel: embeddingModel.trim() || null,
        chatModel: chatModel.trim() || null,
      });
    } catch (e) {
      console.warn("Failed to auto-save model coordinates:", e);
    }
  };

  const handleToggleProjectEgress = async (projId: string, currentVal: boolean) => {
    const nextVal = !currentVal;
    setProjects((prev) =>
      prev.map((p) => (p.id === projId ? { ...p, egress: nextVal } : p))
    );
    try {
      await KnoxAPI.setProjectEgress(projId, nextVal);
      showToast(
        nextVal
          ? "Project cloud egress enabled for this repository."
          : "Project cloud egress revoked: repository restricted to local only."
      );
    } catch (err) {
      setProjects((prev) =>
        prev.map((p) => (p.id === projId ? { ...p, egress: currentVal } : p))
      );
      showToast(`Failed to update project egress: ${String(err)}`, true);
    }
  };

  const handleTestConnection = async () => {
    try {
      setIsTesting(true);
      setTestResult(null);
      const res = await KnoxAPI.testModelConnection({
        profile,
        endpoint: endpoint.trim() || null,
        api_key: apiKey.trim() || null,
        embedding_model: embeddingModel.trim() || null,
        chat_model: chatModel.trim() || null,
        allow_cloud_egress: allowCloudEgress,
      });
      setTestResult(res);
      if (res.success) {
        showToast(`Model connection operational! (${res.latency_ms}ms)`);
      } else {
        showToast(res.error || "Model connection failed", true);
      }
    } catch (err) {
      const msg = String(err);
      setTestResult({
        success: false,
        latency_ms: 0,
        embedding_ok: false,
        embedding_dim: 0,
        error: msg,
      });
      showToast(msg, true);
    } finally {
      setIsTesting(false);
    }
  };

  return (
    <div className="space-y-6 animate-in fade-in-50 duration-150">
      <div>
        <h3 className="text-sm font-semibold text-foreground">Model & AI Engine</h3>
        <p className="text-xs text-muted-foreground">
          Configure local or cloud LLM providers for semantic search, vector embeddings, and synthesis
        </p>
      </div>

      <div className="space-y-4">
        {/* Profile Selector */}
        <div className="p-4 rounded-lg border border-border bg-card/60 space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <BrainCircuit className="size-4 text-primary" />
              <span className="text-xs font-medium text-foreground">Active Model Profile</span>
            </div>
            <Badge variant="outline" className="font-mono text-2xs uppercase">
              {profile}
            </Badge>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
            {[
              {
                id: "off",
                name: "Off (Default)",
                desc: "Zero inference. Fast SQLite FTS and heuristics.",
              },
              {
                id: "mock",
                name: "Mock",
                desc: "Deterministic test vectors without network.",
              },
              {
                id: "local",
                name: "Local",
                desc: "Ollama / LM Studio on localhost.",
              },
              {
                id: "cloud",
                name: "Cloud",
                desc: "OpenAI, Anthropic, or Gemini provider.",
              },
            ].map((p) => (
              <button
                key={p.id}
                type="button"
                onClick={() => handleProfileSelect(p.id)}
                className={cn(
                  "p-3 rounded-lg border text-left transition-all cursor-pointer flex flex-col justify-between",
                  profile === p.id
                    ? "bg-primary/10 border-primary text-foreground ring-1 ring-primary/40 shadow-xs"
                    : "bg-muted/40 border-border hover:bg-muted/70 text-muted-foreground"
                )}
              >
                <div>
                  <span className="text-xs font-semibold block text-foreground">{p.name}</span>
                  <span className="text-2xs text-muted-foreground leading-snug mt-1 block">
                    {p.desc}
                  </span>
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Local / Cloud Endpoint & Models */}
        {profile !== "off" && (
          <div className="p-4 rounded-lg border border-border bg-card/60 space-y-4">
            <div className="flex items-center gap-2 border-b border-border/60 pb-2.5">
              <Server className="size-4 text-primary" />
              <span className="text-xs font-semibold text-foreground">Provider Coordinates</span>
            </div>

            <div className="space-y-3">
              <div>
                <label className="text-xs font-medium text-foreground block mb-1">
                  API Endpoint URL
                </label>
                <input
                  type="text"
                  value={endpoint}
                  onChange={(e) => setEndpoint(e.target.value)}
                  onBlur={handleFieldBlur}
                  placeholder={
                    profile === "local" ? "http://localhost:11434" : "https://api.openai.com/v1"
                  }
                  className="w-full px-3 py-1.5 rounded-md bg-muted/60 border border-border text-xs text-foreground font-mono focus:outline-none focus:ring-1 focus:ring-primary"
                />
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div>
                  <label className="text-xs font-medium text-foreground block mb-1">
                    Embedding Model Identifier
                  </label>
                  <input
                    type="text"
                    value={embeddingModel}
                    onChange={(e) => setEmbeddingModel(e.target.value)}
                    onBlur={handleFieldBlur}
                    placeholder={
                      profile === "local" ? "nomic-embed-text" : "text-embedding-3-small"
                    }
                    className="w-full px-3 py-1.5 rounded-md bg-muted/60 border border-border text-xs text-foreground font-mono focus:outline-none focus:ring-1 focus:ring-primary"
                  />
                </div>
                <div>
                  <label className="text-xs font-medium text-foreground block mb-1">
                    Chat / Synthesis Model Identifier
                  </label>
                  <input
                    type="text"
                    value={chatModel}
                    onChange={(e) => setChatModel(e.target.value)}
                    onBlur={handleFieldBlur}
                    placeholder={profile === "local" ? "llama3.2" : "gpt-4o-mini"}
                    className="w-full px-3 py-1.5 rounded-md bg-muted/60 border border-border text-xs text-foreground font-mono focus:outline-none focus:ring-1 focus:ring-primary"
                  />
                </div>
              </div>

              <div>
                <label className="text-xs font-medium text-foreground block mb-1">
                  API Key (Secret)
                </label>
                <div className="relative">
                  <input
                    type="password"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    onBlur={handleFieldBlur}
                    placeholder="Enter provider API key (persisted safely in config.json)"
                    className="w-full px-3 py-1.5 rounded-md bg-muted/60 border border-border text-xs text-foreground font-mono focus:outline-none focus:ring-1 focus:ring-primary pr-20"
                  />
                  {apiKey && (
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => {
                        setApiKey("");
                        updateConfig({ modelApiKey: null }).catch((e) => console.warn(e));
                      }}
                      className="absolute right-1 top-1 h-6 text-2xs text-muted-foreground hover:text-danger px-2"
                    >
                      Clear
                    </Button>
                  )}
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Cloud Egress & Data Privacy Guard */}
        <div className="p-4 rounded-lg border border-border bg-card/60 space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Lock className="size-4 text-primary" />
              <span className="text-xs font-semibold text-foreground">
                Cloud Egress Authorization
              </span>
            </div>
            <label className="relative inline-flex items-center cursor-pointer">
              <input
                type="checkbox"
                checked={allowCloudEgress}
                onChange={(e) => handleToggleCloudEgress(e.target.checked)}
                className="sr-only peer"
              />
              <div className="w-9 h-5 bg-muted peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-primary"></div>
            </label>
          </div>

          <p className="text-xs text-muted-foreground leading-relaxed">
            Outbound requests to remote cloud AI models transmit source code summaries, decisions,
            and question prompts. Knox enforces zero-egress by default to preserve IP privacy.
          </p>

          {projects.length > 0 && (
            <div className="pt-2 border-t border-border/60">
              <p className="text-2xs uppercase font-mono text-muted-foreground mb-2">
                Project Scope Egress Flags ({projects.length} Tracked)
              </p>
              <div className="max-h-48 overflow-y-auto space-y-1.5 pr-1">
                {projects.map((p) => {
                  const isAgentWorkspace =
                    p.path.includes(".gemini") ||
                    p.path.includes("antigravity\\brain") ||
                    p.path.includes("antigravity/brain") ||
                    /^[0-9a-f]{8}-[0-9a-f]{4}/i.test(p.name);
                  const isEgressAllowed = allowCloudEgress || Boolean(p.egress);

                  return (
                    <div
                      key={p.id}
                      className="flex items-center justify-between p-2.5 rounded-md bg-muted/40 border border-border/60 text-xs"
                    >
                      <div className="min-w-0 flex-1 mr-3">
                        <div className="flex items-center gap-1.5">
                          <span
                            className="font-medium text-foreground truncate max-w-[200px]"
                            title={p.path}
                          >
                            {isAgentWorkspace ? "Agent Scratch Space" : p.name}
                          </span>
                          {isAgentWorkspace && (
                            <Badge
                              variant="outline"
                              className="text-3xs uppercase font-mono px-1 py-0 text-muted-foreground shrink-0"
                            >
                              Agent
                            </Badge>
                          )}
                        </div>
                        <div
                          className="text-3xs font-mono text-muted-foreground truncate"
                          title={p.path}
                        >
                          {p.path}
                        </div>
                      </div>

                      <div className="flex items-center gap-2 shrink-0">
                        <span
                          className={cn(
                            "text-2xs font-mono px-1.5 py-0.5 rounded",
                            isEgressAllowed
                              ? "bg-success/10 text-success border border-success/20"
                              : "bg-muted text-muted-foreground"
                          )}
                        >
                          {allowCloudEgress
                            ? "Global Allowed"
                            : p.egress
                            ? "Project Allowed"
                            : "Local Only"}
                        </span>

                        <label
                          className="relative inline-flex items-center cursor-pointer"
                          title={
                            allowCloudEgress
                              ? "Global egress overrides project settings"
                              : p.egress
                              ? "Click to revoke egress for this project"
                              : "Click to permit cloud egress for this project"
                          }
                        >
                          <input
                            type="checkbox"
                            checked={Boolean(p.egress)}
                            disabled={allowCloudEgress}
                            onChange={() => handleToggleProjectEgress(p.id, Boolean(p.egress))}
                            className="sr-only peer"
                          />
                          <div className="w-7 h-4 bg-muted peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-3 after:w-3 after:transition-all peer-checked:bg-primary peer-disabled:opacity-40 peer-disabled:cursor-not-allowed"></div>
                        </label>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>

        {/* Runtime Diagnostics & Live Status */}
        <div className="p-4 rounded-lg border border-border bg-card/60 space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Sparkles className="size-4 text-primary" />
              <span className="text-xs font-semibold text-foreground">Resolved Runtime State</span>
            </div>
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-1.5 text-2xs text-muted-foreground font-mono">
                <CheckCircle2 className="size-3 text-success shrink-0" />
                <span>Auto-saved</span>
              </div>

              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={handleTestConnection}
                loading={isTesting}
                disabled={profile === "off"}
                className="gap-1.5 h-7 text-xs"
              >
                <Play className="size-3 text-primary" />
                <span>Test Connection</span>
              </Button>
            </div>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2.5 text-xs font-mono">
            <div className="p-2.5 rounded-md bg-muted/40 border border-border">
              <p className="text-2xs text-muted-foreground uppercase">Effective Profile</p>
              <p className="font-semibold text-foreground mt-0.5 uppercase">{profile}</p>
            </div>
            <div className="p-2.5 rounded-md bg-muted/40 border border-border">
              <p className="text-2xs text-muted-foreground uppercase">Endpoint</p>
              <p className="font-semibold text-foreground mt-0.5 truncate">
                {endpoint || (profile === "local" ? "http://localhost:11434" : "none")}
              </p>
            </div>
            <div className="p-2.5 rounded-md bg-muted/40 border border-border">
              <p className="text-2xs text-muted-foreground uppercase">Embeddings</p>
              <p className="font-semibold text-primary mt-0.5">
                {profile === "off" ? "Off" : embeddingModel || "default"}
              </p>
            </div>
            <div className="p-2.5 rounded-md bg-muted/40 border border-border">
              <p className="text-2xs text-muted-foreground uppercase">Egress State</p>
              <p
                className={cn(
                  "font-semibold mt-0.5",
                  allowCloudEgress ? "text-success" : "text-muted-foreground"
                )}
              >
                {allowCloudEgress ? "Authorized" : "Blocked"}
              </p>
            </div>
          </div>

          {testResult && (
            <div
              className={cn(
                "p-3 rounded-md border text-xs font-mono space-y-1",
                testResult.success
                  ? "bg-success/10 border-success/30 text-success"
                  : "bg-danger/10 border-danger/30 text-danger"
              )}
            >
              <div className="flex items-center justify-between">
                <span className="font-semibold">
                  {testResult.success ? "✓ Test Connection Successful" : "✗ Test Connection Failed"}
                </span>
                <span className="text-2xs">{testResult.latency_ms}ms</span>
              </div>
              {testResult.embedding_ok && (
                <p className="text-2xs text-foreground">
                  Vector Embeddings: Operational ({testResult.embedding_dim}-dim vector generated)
                </p>
              )}
              {testResult.answer_preview && (
                <p className="text-2xs text-muted-foreground italic truncate">
                  Preview: &ldquo;{testResult.answer_preview}&rdquo;
                </p>
              )}
              {testResult.error && (
                <p className="text-2xs text-danger break-words">{testResult.error}</p>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
