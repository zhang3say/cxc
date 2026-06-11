package provider

import (
	"fmt"
	"os"
	"text/tabwriter"
	"time"

	"github.com/spf13/cobra"
	"github.com/zhang3say/cxc/internal/config"
)

func newListCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "list",
		Short: "List all saved providers",
		RunE: func(cmd *cobra.Command, args []string) error {
			cfg, err := config.Load()
			if err != nil {
				return err
			}

			if len(cfg.Providers) == 0 {
				fmt.Println("No providers saved. Run `cxc provider add` to add one.")
				return nil
			}

			w := tabwriter.NewWriter(os.Stdout, 0, 0, 2, ' ', 0)
			fmt.Fprintln(w, "  NAME\tBASE URL\tMODEL\tLAST TEST\tLATENCY\tREMARK\t")
			fmt.Fprintln(w, "  ----\t--------\t-----\t---------\t-------\t------\t")
			for _, p := range cfg.Providers {
				active := "  "
				if p.Name == cfg.Active {
					active = "★ "
				}

				lastTest := "-"
				if p.LastTest != nil {
					lastTest = p.LastTest.Format(time.RFC3339)
				}
				latency := "-"
				if p.LatencyMS != nil {
					latency = fmt.Sprintf("%dms", *p.LatencyMS)
					if p.LastOK != nil {
						if *p.LastOK {
							latency = "✓ " + latency
						} else {
							latency = "✗ " + latency
						}
					}
				}

				fmt.Fprintf(w, "%s%s\t%s\t%s\t%s\t%s\t%s\t\n",
					active, p.Name, p.BaseURL, p.Model, lastTest, latency, p.Remark)
			}
			w.Flush()
			return nil
		},
	}
}
