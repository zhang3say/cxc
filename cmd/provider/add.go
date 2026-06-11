package provider

import (
	"bufio"
	"fmt"
	"net/url"
	"os"
	"strings"

	"github.com/spf13/cobra"
	"github.com/zhang3say/cxc/internal/config"
)

func newAddCmd() *cobra.Command {
	var name, baseURL, apiKey, model, wireAPI string

	cmd := &cobra.Command{
		Use:   "add",
		Short: "Add a new provider",
		Long:  "Add a new API relay endpoint provider. Omit flags to be prompted interactively.",
		RunE: func(cmd *cobra.Command, args []string) error {
			cfg, err := config.Load()
			if err != nil {
				return err
			}

			r := bufio.NewReader(os.Stdin)

			name = promptIfEmpty(r, name, "Provider name")
			baseURL = promptIfEmpty(r, baseURL, "Base URL (e.g. https://api.example.com/v1)")
			apiKey = promptIfEmpty(r, apiKey, "API key")
			model = promptIfEmpty(r, model, "Model (e.g. gpt-4)")
			if wireAPI == "" {
				wireAPI = "responses"
			}

			// Validate
			if name == "" {
				return fmt.Errorf("name is required")
			}
			if _, err := url.ParseRequestURI(baseURL); err != nil || !strings.HasPrefix(baseURL, "http") {
				return fmt.Errorf("invalid base_url %q: must be a valid http/https URL", baseURL)
			}
			if apiKey == "" {
				return fmt.Errorf("api_key is required")
			}
			if model == "" {
				return fmt.Errorf("model is required")
			}

			p := config.Provider{
				Name:    name,
				BaseURL: baseURL,
				APIKey:  apiKey,
				Model:   model,
				WireAPI: wireAPI,
			}

			if err := config.AddProvider(cfg, p); err != nil {
				return err
			}

			fmt.Printf("✓ Added provider %q\n", name)
			if cfg.Active == name {
				fmt.Printf("  Set as active provider.\n")
			}
			return nil
		},
	}

	cmd.Flags().StringVar(&name, "name", "", "Provider name (unique identifier)")
	cmd.Flags().StringVar(&baseURL, "base-url", "", "Base URL of the API relay endpoint")
	cmd.Flags().StringVar(&apiKey, "api-key", "", "API key for authentication")
	cmd.Flags().StringVar(&model, "model", "", "Model name (e.g. gpt-4, gpt-5)")
	cmd.Flags().StringVar(&wireAPI, "wire-api", "responses", "Wire API protocol (default: responses)")

	return cmd
}

// promptIfEmpty prompts the user for a value if it's not already set.
func promptIfEmpty(r *bufio.Reader, val, prompt string) string {
	if val != "" {
		return val
	}
	fmt.Printf("%s: ", prompt)
	line, _ := r.ReadString('\n')
	return strings.TrimSpace(line)
}
