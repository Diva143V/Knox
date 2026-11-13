import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { LOCAL_PRESETS, CLOUD_PRESETS } from "../types/knox.ts";
import { DemoAPI } from "../services/demo.ts";

describe("Model Presets & Daemon Detection Logic", () => {
  it("defines all canonical local presets with correct defaults", () => {
    assert.strictEqual(LOCAL_PRESETS.length, 3);

    const ollama = LOCAL_PRESETS.find((p) => p.id === "ollama");
    assert.ok(ollama);
    assert.strictEqual(ollama.endpoint, "http://localhost:11434");
    assert.strictEqual(ollama.embeddingModel, "nomic-embed-text");
    assert.strictEqual(ollama.chatModel, "llama3.2");
    assert.strictEqual(ollama.requiresApiKey, false);
    assert.strictEqual(ollama.defaultPort, 11434);

    const lmStudio = LOCAL_PRESETS.find((p) => p.id === "lm-studio");
    assert.ok(lmStudio);
    assert.strictEqual(lmStudio.endpoint, "http://localhost:1234/v1");
    assert.strictEqual(lmStudio.defaultPort, 1234);
  });

  it("defines all canonical cloud presets with credential requirements", () => {
    assert.strictEqual(CLOUD_PRESETS.length, 4);

    const openai = CLOUD_PRESETS.find((p) => p.id === "openai");
    assert.ok(openai);
    assert.strictEqual(openai.endpoint, "https://api.openai.com/v1");
    assert.strictEqual(openai.embeddingModel, "text-embedding-3-small");
    assert.strictEqual(openai.chatModel, "gpt-4o-mini");
    assert.strictEqual(openai.requiresApiKey, true);

    const gemini = CLOUD_PRESETS.find((p) => p.id === "gemini");
    assert.ok(gemini);
    assert.ok(gemini.endpoint.includes("googleapis.com"));
    assert.strictEqual(gemini.requiresApiKey, true);
  });

  it("checks local daemons and returns structured port status", async () => {
    const daemons = await DemoAPI.checkLocalDaemons();
    assert.ok(Array.isArray(daemons));
    assert.strictEqual(daemons.length, 3);

    const ollama = daemons.find((d) => d.name === "Ollama");
    assert.ok(ollama);
    assert.strictEqual(ollama.port, 11434);
    assert.strictEqual(ollama.is_online, true);
  });

  it("executes askKnox query and returns grounded citations", async () => {
    const res = await DemoAPI.askKnox("Why did we choose SQLite?");
    assert.ok(res.answer.includes("SQLite"));
    assert.ok(res.citations.length >= 1);
    assert.strictEqual(res.grounded, true);
    assert.ok(res.confidence > 0.5);
    assert.strictEqual(res.citations_count, res.citations.length);
  });
});
