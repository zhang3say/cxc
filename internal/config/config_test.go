package config

import (
	"os"
	"path/filepath"
	"testing"
	"time"
)

// setupTestConfig sets up a temporary config dir and returns the cleanup func.
func setupTestConfig(t *testing.T) (cleanup func()) {
	t.Helper()
	dir := t.TempDir()
	// Override XDG_CONFIG_HOME so configPath() uses our temp dir
	t.Setenv("XDG_CONFIG_HOME", dir)
	return func() {}
}

func TestAddProvider(t *testing.T) {
	setupTestConfig(t)

	cfg := &Config{}
	p := Provider{Name: "test", BaseURL: "https://example.com/v1", APIKey: "sk-xxx", Model: "gpt-4"}
	if err := AddProvider(cfg, p); err != nil {
		t.Fatalf("AddProvider: %v", err)
	}

	if len(cfg.Providers) != 1 {
		t.Errorf("expected 1 provider, got %d", len(cfg.Providers))
	}
	if cfg.Active != "test" {
		t.Errorf("expected active=test, got %q", cfg.Active)
	}
	if cfg.Providers[0].WireAPI != "responses" {
		t.Errorf("expected default wire_api=responses, got %q", cfg.Providers[0].WireAPI)
	}
}

func TestAddProviderDuplicate(t *testing.T) {
	setupTestConfig(t)

	cfg := &Config{}
	p := Provider{Name: "test", BaseURL: "https://example.com/v1", APIKey: "sk-xxx", Model: "gpt-4"}
	_ = AddProvider(cfg, p)
	err := AddProvider(cfg, p)
	if err == nil {
		t.Fatal("expected error for duplicate provider name")
	}
}

func TestFirstProviderBecomesActive(t *testing.T) {
	setupTestConfig(t)

	cfg := &Config{}
	_ = AddProvider(cfg, Provider{Name: "a", BaseURL: "https://a.com/v1", APIKey: "k1", Model: "m1"})
	_ = AddProvider(cfg, Provider{Name: "b", BaseURL: "https://b.com/v1", APIKey: "k2", Model: "m2"})

	if cfg.Active != "a" {
		t.Errorf("first provider should be active, got %q", cfg.Active)
	}
}

func TestRemoveProvider(t *testing.T) {
	setupTestConfig(t)

	cfg := &Config{}
	_ = AddProvider(cfg, Provider{Name: "a", BaseURL: "https://a.com/v1", APIKey: "k1", Model: "m1"})
	_ = AddProvider(cfg, Provider{Name: "b", BaseURL: "https://b.com/v1", APIKey: "k2", Model: "m2"})
	_ = SetActive(cfg, "b")

	if err := RemoveProvider(cfg, "a"); err != nil {
		t.Fatalf("RemoveProvider: %v", err)
	}
	if len(cfg.Providers) != 1 {
		t.Errorf("expected 1 provider after removal, got %d", len(cfg.Providers))
	}
}

func TestRemoveActiveProviderFails(t *testing.T) {
	setupTestConfig(t)

	cfg := &Config{}
	_ = AddProvider(cfg, Provider{Name: "a", BaseURL: "https://a.com/v1", APIKey: "k1", Model: "m1"})

	err := RemoveProvider(cfg, "a")
	if err == nil {
		t.Fatal("expected error when removing active provider")
	}
}

func TestRemoveNonExistentProviderFails(t *testing.T) {
	setupTestConfig(t)

	cfg := &Config{}
	_ = AddProvider(cfg, Provider{Name: "a", BaseURL: "https://a.com/v1", APIKey: "k1", Model: "m1"})
	_ = AddProvider(cfg, Provider{Name: "b", BaseURL: "https://b.com/v1", APIKey: "k2", Model: "m2"})
	_ = SetActive(cfg, "b")

	err := RemoveProvider(cfg, "nonexistent")
	if err == nil {
		t.Fatal("expected error for non-existent provider")
	}
}

func TestSetActive(t *testing.T) {
	setupTestConfig(t)

	cfg := &Config{}
	_ = AddProvider(cfg, Provider{Name: "a", BaseURL: "https://a.com/v1", APIKey: "k1", Model: "m1"})
	_ = AddProvider(cfg, Provider{Name: "b", BaseURL: "https://b.com/v1", APIKey: "k2", Model: "m2"})

	if err := SetActive(cfg, "b"); err != nil {
		t.Fatalf("SetActive: %v", err)
	}
	if cfg.Active != "b" {
		t.Errorf("expected active=b, got %q", cfg.Active)
	}
}

func TestPersistence(t *testing.T) {
	setupTestConfig(t)

	cfg := &Config{}
	_ = AddProvider(cfg, Provider{Name: "a", BaseURL: "https://a.com/v1", APIKey: "sk-abc", Model: "gpt-4"})

	loaded, err := Load()
	if err != nil {
		t.Fatalf("Load: %v", err)
	}
	if len(loaded.Providers) != 1 {
		t.Errorf("expected 1 provider after reload, got %d", len(loaded.Providers))
	}
	if loaded.Providers[0].APIKey != "sk-abc" {
		t.Errorf("expected api_key sk-abc, got %q", loaded.Providers[0].APIKey)
	}
}

func TestConfigFilePermissions(t *testing.T) {
	setupTestConfig(t)

	cfg := &Config{}
	_ = AddProvider(cfg, Provider{Name: "a", BaseURL: "https://a.com/v1", APIKey: "k1", Model: "m1"})

	path, _ := configPath()
	info, err := os.Stat(path)
	if err != nil {
		t.Fatalf("stat config file: %v", err)
	}
	if perm := info.Mode().Perm(); perm != 0600 {
		t.Errorf("expected 0600 permissions, got %o", perm)
	}
}

func TestUpdateTestResult(t *testing.T) {
	setupTestConfig(t)

	cfg := &Config{}
	_ = AddProvider(cfg, Provider{Name: "a", BaseURL: "https://a.com/v1", APIKey: "k1", Model: "m1"})

	if err := UpdateTestResult(cfg, "a", 123, true); err != nil {
		t.Fatalf("UpdateTestResult: %v", err)
	}

	p, ok := GetProvider(cfg, "a")
	if !ok {
		t.Fatal("provider not found")
	}
	if p.LatencyMS == nil || *p.LatencyMS != 123 {
		t.Errorf("expected latency 123, got %v", p.LatencyMS)
	}
	if p.LastOK == nil || !*p.LastOK {
		t.Error("expected last_ok=true")
	}
	if p.LastTest == nil || p.LastTest.After(time.Now()) {
		t.Error("expected valid last_test timestamp")
	}
}

func TestConfigDir(t *testing.T) {
	setupTestConfig(t)

	cfg := &Config{}
	_ = AddProvider(cfg, Provider{Name: "a", BaseURL: "https://a.com/v1", APIKey: "k1", Model: "m1"})

	path, err := configPath()
	if err != nil {
		t.Fatal(err)
	}
	dir := filepath.Dir(path)
	info, err := os.Stat(dir)
	if err != nil {
		t.Fatalf("stat config dir: %v", err)
	}
	if !info.IsDir() {
		t.Error("config dir should be a directory")
	}
}

func TestRemarkPersistence(t *testing.T) {
	setupTestConfig(t)

	cfg := &Config{}
	_ = AddProvider(cfg, Provider{
		Name:    "a",
		BaseURL: "https://a.com/v1",
		APIKey:  "k1",
		Model:   "m1",
		Remark:  "My backup endpoint",
	})

	loaded, err := Load()
	if err != nil {
		t.Fatalf("Load: %v", err)
	}
	if len(loaded.Providers) != 1 {
		t.Fatalf("expected 1 provider, got %d", len(loaded.Providers))
	}
	if loaded.Providers[0].Remark != "My backup endpoint" {
		t.Errorf("expected remark 'My backup endpoint', got %q", loaded.Providers[0].Remark)
	}
}
