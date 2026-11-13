import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { DemoAPI } from "../services/demo.ts";

describe("Config Update & Toggle Persistence Logic", () => {
  it("preserves unmodified fields during partial updates", async () => {
    // Initial state check
    const initial = await DemoAPI.getConfig();
    assert.strictEqual(initial.model_profile, "off");
    assert.strictEqual(initial.allow_cloud_egress, false);

    // Update only modelProfile
    const updatedProfile = await DemoAPI.updateConfig({ modelProfile: "local" });
    assert.strictEqual(updatedProfile.model_profile, "local");
    assert.strictEqual(updatedProfile.allow_cloud_egress, false);
    assert.strictEqual(updatedProfile.session_timeout_seconds, 1800);

    // Update only allowCloudEgress
    const updatedEgress = await DemoAPI.updateConfig({ allowCloudEgress: true });
    assert.strictEqual(updatedEgress.allow_cloud_egress, true);
    assert.strictEqual(updatedEgress.model_profile, "local"); // retained from previous update

    // Update strayDbPrompted
    const updatedPrompt = await DemoAPI.updateConfig({ strayDbPrompted: true });
    assert.strictEqual(updatedPrompt.stray_db_prompted, true);
    assert.strictEqual(updatedPrompt.allow_cloud_egress, true);
    assert.strictEqual(updatedPrompt.model_profile, "local");
  });

  it("masks API key and preserves model endpoint", async () => {
    const updated = await DemoAPI.updateConfig({
      modelEndpoint: "http://localhost:11434",
      modelApiKey: "sk-secret-key-12345",
      embeddingModel: "nomic-embed-text",
      chatModel: "llama3.2",
    });

    assert.strictEqual(updated.model_endpoint, "http://localhost:11434");
    assert.strictEqual(updated.model_api_key_masked, "••••••••");
    assert.strictEqual(updated.embedding_model, "nomic-embed-text");
    assert.strictEqual(updated.chat_model, "llama3.2");
  });
});
