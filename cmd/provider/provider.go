// Package provider contains all `cxc provider` subcommands.
package provider

import (
	"github.com/spf13/cobra"
)

// NewCmd returns the `provider` parent command with all subcommands attached.
func NewCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "provider",
		Short: "Manage API relay endpoint providers",
		Long:  "Add, list, test, switch, and remove API relay endpoint providers.",
	}

	cmd.AddCommand(newAddCmd())
	cmd.AddCommand(newListCmd())
	cmd.AddCommand(newTestCmd())
	cmd.AddCommand(newSwitchCmd())
	cmd.AddCommand(newRemoveCmd())

	return cmd
}
