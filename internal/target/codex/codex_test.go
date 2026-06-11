package codex

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/pelletier/go-toml/v2"
	"github.com/zhang3say/cxc/internal/target"
)

// minimalConfigTOML is a realistic Codex config with many unrelated sections.
const minimalConfigTOML = `approval_policy = "never"
model = "gpt-4"
model_provider = "codex"

[model_providers.codex]
base_url = "https://old.example.com/v1"
name = "codex"
requires_openai_auth = true
wire_api = "responses"

[features]
guardian_approval = true
memories = true

[mcp_servers.context7]
args = ["-y", "@upstash/context7-mcp"]
command = "npx"

[projects."/home/user"]
trust_level = "trusted"
`

const minimalAuthJSON = `{
  "auth_mode": "apikey",
  "OPENAI_API_KEY": "sk-old-key"
}`

func setupCodexDir(t *testing.T) (string, *Adapter) {
	t.Helper()
	dir := t.TempDir()

	if err := os.WriteFile(filepath.Join(dir, "config.toml"), []byte(minimalConfigTOML), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "auth.json"), []byte(minimalAuthJSON), 0600); err != nil {
		t.Fatal(err)
	}

	return dir, NewWithDir(dir)
}

func TestRead(t *testing.T) {
	_, adapter := setupCodexDir(t)

	cfg, err := adapter.Read()
	if err != nil {
		t.Fatalf("Read: %v", err)
	}

	if cfg.Model != "gpt-4" {
		t.Errorf("expected model gpt-4, got %q", cfg.Model)
	}
	if cfg.BaseURL != "https://old.example.com/v1" {
		t.Errorf("expected old base_url, got %q", cfg.BaseURL)
	}
	if cfg.APIKey != "sk-old-key" {
		t.Errorf("expected old api_key, got %q", cfg.APIKey)
	}
	if cfg.WireAPI != "responses" {
		t.Errorf("expected wire_api=responses, got %q", cfg.WireAPI)
	}
}

func TestWrite(t *testing.T) {
	dir, adapter := setupCodexDir(t)

	newCfg := &target.Config{
		BaseURL: "https://new.example.com/v1",
		APIKey:  "sk-new-key",
		Model:   "gpt-5",
		WireAPI: "responses",
	}

	if err := adapter.Write(newCfg); err != nil {
		t.Fatalf("Write: %v", err)
	}

	// Read back and verify
	got, err := adapter.Read()
	if err != nil {
		t.Fatalf("Read after Write: %v", err)
	}
	if got.BaseURL != newCfg.BaseURL {
		t.Errorf("base_url: expected %q, got %q", newCfg.BaseURL, got.BaseURL)
	}
	if got.APIKey != newCfg.APIKey {
		t.Errorf("api_key: expected %q, got %q", newCfg.APIKey, got.APIKey)
	}
	if got.Model != newCfg.Model {
		t.Errorf("model: expected %q, got %q", newCfg.Model, got.Model)
	}

	// Verify backups were created
	if _, err := os.Stat(filepath.Join(dir, "config.toml.bak")); os.IsNotExist(err) {
		t.Error("config.toml.bak was not created")
	}
	if _, err := os.Stat(filepath.Join(dir, "auth.json.bak")); os.IsNotExist(err) {
		t.Error("auth.json.bak was not created")
	}
}

func TestWritePreservesUnrelatedSections(t *testing.T) {
	_, adapter := setupCodexDir(t)

	newCfg := &target.Config{
		BaseURL: "https://new.example.com/v1",
		APIKey:  "sk-new",
		Model:   "gpt-5",
		WireAPI: "responses",
	}

	if err := adapter.Write(newCfg); err != nil {
		t.Fatalf("Write: %v", err)
	}

	// Read raw TOML and verify unrelated sections are preserved
	data, err := os.ReadFile(adapter.configPath())
	if err != nil {
		t.Fatalf("reading config: %v", err)
	}

	content := string(data)
	checks := []string{
		"guardian_approval",
		"memories",
		"context7",
		"trusted",
	}
	for _, check := range checks {
		if !strings.Contains(content, check) {
			t.Errorf("unrelated section key %q was lost after Write", check)
		}
	}
}

func TestWriteCreatesBackupWithOriginalContent(t *testing.T) {
	dir, adapter := setupCodexDir(t)

	newCfg := &target.Config{
		BaseURL: "https://new.example.com/v1",
		APIKey:  "sk-new",
		Model:   "gpt-5",
		WireAPI: "responses",
	}
	if err := adapter.Write(newCfg); err != nil {
		t.Fatal(err)
	}

	// Backup should contain the old API key
	bakData, err := os.ReadFile(filepath.Join(dir, "auth.json.bak"))
	if err != nil {
		t.Fatal(err)
	}
	var bak map[string]any
	if err := json.Unmarshal(bakData, &bak); err != nil {
		t.Fatal(err)
	}
	if bak["OPENAI_API_KEY"] != "sk-old-key" {
		t.Errorf("backup should contain old key, got %v", bak["OPENAI_API_KEY"])
	}
}

func TestWriteProducesValidTOML(t *testing.T) {
	dir, adapter := setupCodexDir(t)

	newCfg := &target.Config{
		BaseURL: "https://new.example.com/v1",
		APIKey:  "sk-new",
		Model:   "gpt-5",
		WireAPI: "responses",
	}
	if err := adapter.Write(newCfg); err != nil {
		t.Fatal(err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "config.toml"))
	if err != nil {
		t.Fatal(err)
	}
	var check map[string]any
	if err := toml.Unmarshal(data, &check); err != nil {
		t.Errorf("written TOML is invalid: %v", err)
	}
}

func TestNameReturnsCodex(t *testing.T) {
	_, adapter := setupCodexDir(t)
	if adapter.Name() != "Codex" {
		t.Errorf("expected Name()=Codex, got %q", adapter.Name())
	}
}
