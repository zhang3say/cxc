package provider

import (
	"github.com/zhang3say/cxc/internal/config"
	"github.com/zhang3say/cxc/internal/target"
)

// codex2TargetConfig converts a CXC Provider to a target.Config for the Codex adapter.
func codex2TargetConfig(p *config.Provider) target.Config {
	wireAPI := p.WireAPI
	if wireAPI == "" {
		wireAPI = "responses"
	}
	return target.Config{
		BaseURL: p.BaseURL,
		APIKey:  p.APIKey,
		Model:   p.Model,
		WireAPI: wireAPI,
	}
}
