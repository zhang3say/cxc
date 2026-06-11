package provider

import (
	"fmt"
	"sync"

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
Use the --all flag to test all saved providers concurrently.`,
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

				type testResult struct {
					p      config.Provider
					result connectivity.Result
				}

				resultsChan := make(chan testResult, len(cfg.Providers))
				var wg sync.WaitGroup

				fmt.Printf("Testing all %d providers concurrently…\n", len(cfg.Providers))

				for _, p := range cfg.Providers {
					wg.Add(1)
					go func(prov config.Provider) {
						defer wg.Done()
						tester := connectivity.New(nil)
						res := tester.Test(prov.BaseURL, prov.APIKey, prov.Model)
						resultsChan <- testResult{p: prov, result: res}
					}(p)
				}

				wg.Wait()
				close(resultsChan)

				// Map results by provider name to print them in the original order
				resultsMap := make(map[string]connectivity.Result)
				for res := range resultsChan {
					resultsMap[res.p.Name] = res.result
				}

				anyFailed := false
				for i, p := range cfg.Providers {
					if i > 0 {
						fmt.Println()
					}
					res := resultsMap[p.Name]
					fmt.Printf("Testing provider %q (%s, model: %s)…\n", p.Name, p.BaseURL, p.Model)
					if res.OK {
						fmt.Printf("✓ Connected in %dms\n", res.LatencyMS)
						if res.Response != "" {
							fmt.Printf("  Model response: %q\n", res.Response)
						}
					} else {
						fmt.Printf("✗ Failed: %s\n", res.Error)
						anyFailed = true
					}

					// Persist result
					_ = config.UpdateTestResult(cfg, p.Name, res.LatencyMS, res.OK)
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

	cmd.Flags().BoolVarP(&all, "all", "a", false, "Test all saved providers concurrently")

	return cmd
}
