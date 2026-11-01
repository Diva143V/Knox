import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  isThemePreference,
  isResolvedTheme,
  resolveTheme,
  DEFAULT_THEME_PREFERENCE,
  type ThemePreference,
} from "../lib/theme.ts";

describe("Theme State & Resolution Logic", () => {
  it("validates theme preference types accurately", () => {
    assert.equal(isThemePreference("dark"), true);
    assert.equal(isThemePreference("light"), true);
    assert.equal(isThemePreference("system"), true);
    assert.equal(isThemePreference("cyberpunk"), false);
    assert.equal(isThemePreference(""), false);
    assert.equal(isThemePreference(null), false);
    assert.equal(isThemePreference(undefined), false);
  });

  it("validates resolved theme mode types", () => {
    assert.equal(isResolvedTheme("dark"), true);
    assert.equal(isResolvedTheme("light"), true);
    assert.equal(isResolvedTheme("system"), false);
    assert.equal(isResolvedTheme(123), false);
  });

  it("resolves static preferences without ambiguity", () => {
    assert.equal(resolveTheme("dark"), "dark");
    assert.equal(resolveTheme("light"), "light");
  });

  it("resolves system preference to dark or light fallback in headless node env", () => {
    const resolved = resolveTheme("system");
    assert.ok(resolved === "dark" || resolved === "light");
  });

  it("defaults to 'dark' for default theme preference", () => {
    assert.equal(DEFAULT_THEME_PREFERENCE, "dark");
  });
});

