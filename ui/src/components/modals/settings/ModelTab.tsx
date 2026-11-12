import React, { useState, useEffect } from "react";
import {
  BrainCircuit,
  Server,
  ShieldAlert,
  Play,
  Sparkles,
  Lock,
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
  const [isSaving, setIsSaving] = useState(false);
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

  const handleSaveSettings = async () => {
    try {
      setIsSaving(true);
      await updateConfig({
        modelProfile: profile,
        modelEndpoint: endpoint.trim() || null,
        modelApiKey: apiKey.trim() || null,
        embeddingModel: embeddingModel.trim() || null,
        chatModel: chatModel.trim() || null,
        allowCloudEgress,
      });
      showToast("Model settings saved to ~/.knox/config.json");
    } catch (err) {
      showToast(String(err), true);
    } finally {
      setIsSaving(false);
    }
  };

  const isCloudWithoutEgress = profile === "cloud" && !allowCloudEgress;

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
                onClick={() => setProfile(p.id)}
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
                    placeholder="Enter provider API key (persisted safely in config.json)"
                    className="w-full px-3 py-1.5 rounded-md bg-muted/60 border border-border text-xs text-foreground font-mono focus:outline-none focus:ring-1 focus:ring-primary pr-20"
                  />
                  {apiKey && (
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => setApiKey("")}
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
                onChange={(e) => setAllowCloudEgress(e.target.checked)}
                className="sr-only peer"
              />
              <div className="w-9 h-5 bg-muted peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-primary"></div>
            </label>
          </div>

          <p className="text-xs text-muted-foreground leading-relaxed">
            Outbound requests to remote cloud AI models transmit source code summaries, decisions,
            and question prompts. Knox enforces zero-egress by default to preserve IP privacy.
          </p>

          {isCloudWithoutEgress && (
            <div className="p-3 rounded-md bg-danger/10 border border-danger/30 flex items-start gap-2.5 text-xs text-danger">
              <ShieldAlert className="size-4 shrink-0 mt-0.5" />
              <div>
                <p className="font-semibold">Cloud Profile Blocked</p>
                <p className="text-2xs text-danger/90 mt-0.5">
                  Cloud model profile cannot be activated without explicitly authorizing outbound
                  cloud egress above.
                </p>
              </div>
            </div>
          )}

          {projects.length > 0 && (
            <div className="pt-2 border-t border-border/60">
              <p className="text-2xs uppercase font-mono text-muted-foreground mb-2">
                Project Scope Egress Flags ({projects.length} Tracked)
              </p>
              <div className="max-h-32 overflow-y-auto space-y-1.5 pr-1">
                {projects.map((p) => (
                  <div
                    key={p.id}
                    className="flex items-center justify-between p-2 rounded-md bg-muted/40 text-xs font-mono"
                  >
                    <span className="truncate max-w-[240px] text-foreground">{p.name}</span>
                    <Badge variant="outline" className="text-3xs">
                      {allowCloudEgress ? "Egress Permitted" : "Local Only"}
                    </Badge>
                  </div>
                ))}
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
            <div className="flex items-center gap-2">
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={handleTestConnection}
                loading={isTesting}
                disabled={profile === "off" || isCloudWithoutEgress}
                className="gap-1.5 h-7 text-xs"
              >
                <Play className="size-3 text-primary" />
                <span>Test Connection</span>
              </Button>

              <Button
                type="button"
                variant="primary"
                size="sm"
                onClick={handleSaveSettings}
                loading={isSaving}
                className="h-7 text-xs"
              >
                Save Preferences
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
