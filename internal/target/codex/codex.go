// Package codex implements the TargetToolAdapter for Codex.
// It reads and writes ~/.codex/config.toml and ~/.codex/auth.json.
package codex

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"

	"github.com/pelletier/go-toml/v2"
	"github.com/zhang3say/cxc/internal/target"
)

const (
	configFile = "config.toml"
	authFile   = "auth.json"
)

// Adapter is the TargetToolAdapter for Codex.
type Adapter struct {
	// codexDir is ~/.codex by default; overridable for tests.
	codexDir string
}

// New returns an Adapter pointed at the real Codex config directory.
func New() *Adapter {
	home, _ := os.UserHomeDir()
	return &Adapter{codexDir: filepath.Join(home, ".codex")}
}

// NewWithDir returns an Adapter using a custom directory (for tests).
func NewWithDir(dir string) *Adapter {
	return &Adapter{codexDir: dir}
}

func (a *Adapter) Name() string { return "Codex" }

// Read returns the current active config from Codex's config files.
func (a *Adapter) Read() (*target.Config, error) {
	tomlData, err := os.ReadFile(a.configPath())
	if err != nil {
		return nil, fmt.Errorf("reading %s: %w", a.configPath(), err)
	}

	var raw map[string]any
	if err := toml.Unmarshal(tomlData, &raw); err != nil {
		return nil, fmt.Errorf("parsing config.toml: %w", err)
	}

	cfg := &target.Config{}
	if m, ok := raw["model"].(string); ok {
		cfg.Model = m
	}

	if providers, ok := raw["model_providers"].(map[string]any); ok {
		if codex, ok := providers["codex"].(map[string]any); ok {
			if bu, ok := codex["base_url"].(string); ok {
				cfg.BaseURL = bu
			}
			if wa, ok := codex["wire_api"].(string); ok {
				cfg.WireAPI = wa
			}
		}
	}

	// Read API key from auth.json
	authData, err := os.ReadFile(a.authPath())
	if err != nil {
		return nil, fmt.Errorf("reading %s: %w", a.authPath(), err)
	}
	var auth map[string]any
	if err := json.Unmarshal(authData, &auth); err != nil {
		return nil, fmt.Errorf("parsing auth.json: %w", err)
	}
	if key, ok := auth["OPENAI_API_KEY"].(string); ok {
		cfg.APIKey = key
	}

	return cfg, nil
}

// Write applies the given Config to Codex's config files.
// Creates .bak backups of both files before writing.
func (a *Adapter) Write(cfg *target.Config) error {
	// Backup both files first
	if err := backup(a.configPath()); err != nil {
		return fmt.Errorf("backing up config.toml: %w", err)
	}
	if err := backup(a.authPath()); err != nil {
		return fmt.Errorf("backing up auth.json: %w", err)
	}

	// Update config.toml
	if err := a.writeConfigTOML(cfg); err != nil {
		return err
	}

	// Update auth.json
	if err := a.writeAuthJSON(cfg.APIKey); err != nil {
		return err
	}

	return nil
}

func (a *Adapter) writeConfigTOML(cfg *target.Config) error {
	data, err := os.ReadFile(a.configPath())
	if err != nil {
		return fmt.Errorf("reading config.toml: %w", err)
	}

	// Parse into a dynamic map to preserve all unrelated sections
	var raw map[string]any
	if err := toml.Unmarshal(data, &raw); err != nil {
		return fmt.Errorf("parsing config.toml: %w", err)
	}

	// Update top-level model
	raw["model"] = cfg.Model

	// Update [model_providers.codex]
	providers, ok := raw["model_providers"].(map[string]any)
	if !ok {
		providers = map[string]any{}
		raw["model_providers"] = providers
	}
	codexSection, ok := providers["codex"].(map[string]any)
	if !ok {
		codexSection = map[string]any{}
		providers["codex"] = codexSection
	}
	codexSection["base_url"] = cfg.BaseURL
	codexSection["wire_api"] = cfg.WireAPI
	codexSection["name"] = "codex"
	codexSection["requires_openai_auth"] = true

	// Marshal back
	out, err := toml.Marshal(raw)
	if err != nil {
		return fmt.Errorf("marshaling config.toml: %w", err)
	}

	// Validate the output parses correctly
	var check map[string]any
	if err := toml.Unmarshal(out, &check); err != nil {
		return fmt.Errorf("validation: written TOML is invalid: %w", err)
	}

	if err := os.WriteFile(a.configPath(), out, 0600); err != nil {
		return fmt.Errorf("writing config.toml: %w", err)
	}
	return nil
}

func (a *Adapter) writeAuthJSON(apiKey string) error {
	data, err := os.ReadFile(a.authPath())
	if err != nil {
		return fmt.Errorf("reading auth.json: %w", err)
	}

	var auth map[string]any
	if err := json.Unmarshal(data, &auth); err != nil {
		return fmt.Errorf("parsing auth.json: %w", err)
	}

	auth["OPENAI_API_KEY"] = apiKey

	out, err := json.MarshalIndent(auth, "", "  ")
	if err != nil {
		return fmt.Errorf("marshaling auth.json: %w", err)
	}

	if err := os.WriteFile(a.authPath(), out, 0600); err != nil {
		return fmt.Errorf("writing auth.json: %w", err)
	}
	return nil
}

func (a *Adapter) configPath() string {
	return filepath.Join(a.codexDir, configFile)
}

func (a *Adapter) authPath() string {
	return filepath.Join(a.codexDir, authFile)
}

// backup creates a .bak copy of the given file.
func backup(path string) error {
	data, err := os.ReadFile(path)
	if os.IsNotExist(err) {
		return nil // nothing to back up
	}
	if err != nil {
		return err
	}
	return os.WriteFile(path+".bak", data, 0600)
}
