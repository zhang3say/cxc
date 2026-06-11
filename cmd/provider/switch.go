package provider

import (
	"fmt"

	"github.com/spf13/cobra"
	"github.com/zhang3say/cxc/internal/config"
	codexadapter "github.com/zhang3say/cxc/internal/target/codex"
)

func newSwitchCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "switch <name>",
		Short: "Switch the active provider",
		Long: `Switch to a named provider. This updates CXC's active provider
and modifies Codex's configuration files to use the new API relay endpoint.
Backups of Codex's config files are created before writing.`,
		Args: cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			name := args[0]

			cfg, err := config.Load()
			if err != nil {
				return err
			}

			// Check already active
			if cfg.Active == name {
				fmt.Printf("Provider %q is already active.\n", name)
				return nil
			}

			// Find the target provider
			p, ok := config.GetProvider(cfg, name)
			if !ok {
				names := availableNames(cfg)
				return fmt.Errorf("provider %q not found\n\nAvailable providers: %s", name, names)
			}

			// Get old config for display
			adapter := codexadapter.New()
			oldCfg, _ := adapter.Read()

			// Apply to Codex config files
			fmt.Printf("Switching Codex to provider %q…\n", name)

			tc := codex2TargetConfig(p)
			if err := adapter.Write(&tc); err != nil {
				return fmt.Errorf("updating Codex config: %w", err)
			}

			// Update CXC's own active pointer
			if err := config.SetActive(cfg, name); err != nil {
				return err
			}

			fmt.Printf("✓ Switched to %q\n", name)
			if oldCfg != nil {
				fmt.Printf("  base_url: %s → %s\n", oldCfg.BaseURL, p.BaseURL)
				fmt.Printf("  model:    %s → %s\n", oldCfg.Model, p.Model)
			}
			fmt.Println("  (Codex config.toml and auth.json updated; .bak backups created)")
			return nil
		},
	}
}

// availableNames returns a comma-separated list of provider names.
func availableNames(cfg *config.Config) string {
	names := make([]string, len(cfg.Providers))
	for i, p := range cfg.Providers {
		names[i] = p.Name
	}
	result := ""
	for i, n := range names {
		if i > 0 {
			result += ", "
		}
		result += n
	}
	return result
}
