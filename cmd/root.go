// Package cmd is the Cobra CLI entry point for CXC.
package cmd

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"
	"github.com/zhang3say/cxc/cmd/provider"
	"github.com/zhang3say/cxc/tui"
)

var rootCmd = &cobra.Command{
	Use:   "cxc",
	Short: "Codex Cross-Connect — manage AI coding tool API relay endpoints",
	Long: `CXC (Codex Cross-Connect) lets you save, test, and switch between
API relay endpoints for AI coding tools like Codex.

Run without arguments to launch the interactive TUI.`,
	RunE: func(cmd *cobra.Command, args []string) error {
		return tui.Run()
	},
}

// Execute runs the root command.
func Execute() {
	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func init() {
	rootCmd.AddCommand(provider.NewCmd())
}
