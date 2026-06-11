package provider

import (
	"fmt"

	"github.com/spf13/cobra"
	"github.com/zhang3say/cxc/internal/config"
	"github.com/zhang3say/cxc/internal/connectivity"
)

func newTestCmd() *cobra.Command {
	var all bool

	cmd := &cobra.Command{
		Use:   "test [name]",
		Short: "Test a provider's connectivity",
		Long: `Test a provider by sending a real chat completion request.
If no name is given, the active provider is tested.
Use the --all flag to test all saved providers sequentially.`,
		Args: cobra.MaximumNArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			cfg, err := config.Load()
			if err != nil {
				return err
			}

			if all {
				if len(args) > 0 {
					return fmt.Errorf("cannot specify a provider name when using --all")
				}
				if len(cfg.Providers) == 0 {
					fmt.Println("No providers saved. Run `cxc provider add` to add one.")
					return nil
				}

				anyFailed := false
				tester := connectivity.New(nil)

				for i, p := range cfg.Providers {
					if i > 0 {
						fmt.Println()
					}
					fmt.Printf("Testing provider %q (%s, model: %s)…\n", p.Name, p.BaseURL, p.Model)
					result := tester.Test(p.BaseURL, p.APIKey, p.Model)

					if result.OK {
						fmt.Printf("✓ Connected in %dms\n", result.LatencyMS)
						if result.Response != "" {
							fmt.Printf("  Model response: %q\n", result.Response)
						}
					} else {
						fmt.Printf("✗ Failed: %s\n", result.Error)
						anyFailed = true
					}

					// Persist result
					_ = config.UpdateTestResult(cfg, p.Name, result.LatencyMS, result.OK)
				}

				if anyFailed {
					return fmt.Errorf("one or more connectivity tests failed")
				}
				return nil
			}

			var p *config.Provider
			if len(args) == 1 {
				found, ok := config.GetProvider(cfg, args[0])
				if !ok {
					return fmt.Errorf("provider %q not found\n\nRun `cxc provider list` to see available providers", args[0])
				}
				p = found
			} else {
				found, ok := config.GetActive(cfg)
				if !ok {
					return fmt.Errorf("no active provider — run `cxc provider add` first")
				}
				p = found
			}

			fmt.Printf("Testing provider %q (%s, model: %s)…\n", p.Name, p.BaseURL, p.Model)

			tester := connectivity.New(nil)
			result := tester.Test(p.BaseURL, p.APIKey, p.Model)

			if result.OK {
				fmt.Printf("✓ Connected in %dms\n", result.LatencyMS)
				if result.Response != "" {
					fmt.Printf("  Model response: %q\n", result.Response)
				}
			} else {
				fmt.Printf("✗ Failed: %s\n", result.Error)
			}

			// Persist result
			_ = config.UpdateTestResult(cfg, p.Name, result.LatencyMS, result.OK)

			if !result.OK {
				return fmt.Errorf("connectivity test failed")
			}
			return nil
		},
	}

	cmd.Flags().BoolVarP(&all, "all", "a", false, "Test all saved providers sequentially")

	return cmd
}
