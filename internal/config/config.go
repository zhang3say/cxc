// Package config manages CXC's own configuration file at ~/.config/cxc/config.yaml.
// It stores the list of Providers and tracks the Active Provider.
package config

import (
	"fmt"
	"os"
	"path/filepath"
	"time"

	"gopkg.in/yaml.v3"
)

const configFileName = "config.yaml"
const configDirName = "cxc"

// Provider represents one API relay endpoint configuration.
type Provider struct {
	Name      string     `yaml:"name"`
	BaseURL   string     `yaml:"base_url"`
	APIKey    string     `yaml:"api_key"`
	Model     string     `yaml:"model"`
	WireAPI   string     `yaml:"wire_api"`
	LastTest  *time.Time `yaml:"last_test,omitempty"`
	LatencyMS *int64     `yaml:"latency_ms,omitempty"`
	LastOK    *bool      `yaml:"last_ok,omitempty"`
}

// Config is the root structure of ~/.config/cxc/config.yaml.
type Config struct {
	Active    string     `yaml:"active"`
	Providers []Provider `yaml:"providers"`
}

// Load reads the config file from disk. Returns an empty Config if the file doesn't exist yet.
func Load() (*Config, error) {
	path, err := configPath()
	if err != nil {
		return nil, err
	}

	data, err := os.ReadFile(path)
	if os.IsNotExist(err) {
		return &Config{}, nil
	}
	if err != nil {
		return nil, fmt.Errorf("reading config: %w", err)
	}

	var cfg Config
	if err := yaml.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("parsing config: %w", err)
	}
	return &cfg, nil
}

// Save writes the config to disk, creating the directory if needed.
// File permissions are set to 0600.
func Save(cfg *Config) error {
	path, err := configPath()
	if err != nil {
		return err
	}

	if err := os.MkdirAll(filepath.Dir(path), 0700); err != nil {
		return fmt.Errorf("creating config dir: %w", err)
	}

	data, err := yaml.Marshal(cfg)
	if err != nil {
		return fmt.Errorf("marshaling config: %w", err)
	}

	if err := os.WriteFile(path, data, 0600); err != nil {
		return fmt.Errorf("writing config: %w", err)
	}
	return nil
}

// AddProvider adds a new Provider and saves.
// Returns an error if the name already exists.
func AddProvider(cfg *Config, p Provider) error {
	for _, existing := range cfg.Providers {
		if existing.Name == p.Name {
			return fmt.Errorf("provider %q already exists", p.Name)
		}
	}
	if p.WireAPI == "" {
		p.WireAPI = "responses"
	}
	cfg.Providers = append(cfg.Providers, p)
	if cfg.Active == "" {
		cfg.Active = p.Name
	}
	return Save(cfg)
}

// RemoveProvider removes a Provider by name.
// Returns an error if it's the Active Provider.
func RemoveProvider(cfg *Config, name string) error {
	if cfg.Active == name {
		return fmt.Errorf("cannot remove the active provider %q — switch to another provider first", name)
	}
	for i, p := range cfg.Providers {
		if p.Name == name {
			cfg.Providers = append(cfg.Providers[:i], cfg.Providers[i+1:]...)
			return Save(cfg)
		}
	}
	return fmt.Errorf("provider %q not found", name)
}

// SetActive sets the Active Provider by name and saves.
func SetActive(cfg *Config, name string) error {
	for _, p := range cfg.Providers {
		if p.Name == name {
			cfg.Active = name
			return Save(cfg)
		}
	}
	return fmt.Errorf("provider %q not found", name)
}

// GetProvider finds a Provider by name.
func GetProvider(cfg *Config, name string) (*Provider, bool) {
	for i := range cfg.Providers {
		if cfg.Providers[i].Name == name {
			return &cfg.Providers[i], true
		}
	}
	return nil, false
}

// GetActive returns the Active Provider.
func GetActive(cfg *Config) (*Provider, bool) {
	return GetProvider(cfg, cfg.Active)
}

// UpdateTestResult updates the last test metadata for a Provider.
func UpdateTestResult(cfg *Config, name string, latencyMS int64, ok bool) error {
	for i := range cfg.Providers {
		if cfg.Providers[i].Name == name {
			now := time.Now()
			cfg.Providers[i].LastTest = &now
			cfg.Providers[i].LatencyMS = &latencyMS
			cfg.Providers[i].LastOK = &ok
			return Save(cfg)
		}
	}
	return fmt.Errorf("provider %q not found", name)
}

// configPath returns the path to the config file.
func configPath() (string, error) {
	configDir, err := os.UserConfigDir()
	if err != nil {
		return "", fmt.Errorf("finding config dir: %w", err)
	}
	return filepath.Join(configDir, configDirName, configFileName), nil
}

// ConfigPath returns the public config file path (for display purposes).
func ConfigPath() (string, error) {
	return configPath()
}
