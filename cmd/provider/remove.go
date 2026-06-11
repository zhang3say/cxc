package provider

import (
	"fmt"

	"github.com/spf13/cobra"
	"github.com/zhang3say/cxc/internal/config"
)

func newRemoveCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "remove <name>",
		Short: "Remove a saved provider",
		Long:  "Remove a provider. Cannot remove the currently active provider.",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			name := args[0]

			cfg, err := config.Load()
			if err != nil {
				return err
			}

			if err := config.RemoveProvider(cfg, name); err != nil {
				return err
			}

			fmt.Printf("✓ Removed provider %q\n", name)
			return nil
		},
	}
}
