package provider

import (
	"bufio"
	"fmt"
	"net/url"
	"os"
	"strings"

	"github.com/spf13/cobra"
	"github.com/zhang3say/cxc/internal/config"
	codexadapter "github.com/zhang3say/cxc/internal/target/codex"
)

func newEditCmd() *cobra.Command {
	var nameFlag, baseURLFlag, apiKeyFlag, modelFlag, wireAPIFlag, remarkFlag string

	cmd := &cobra.Command{
		Use:   "edit <name>",
		Short: "Edit an existing provider",
		Long:  "Update the fields of a saved provider. Omit flags to edit interactively.",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			oldName := args[0]

			cfg, err := config.Load()
			if err != nil {
				return err
			}

			// Find target provider
			existing, ok := config.GetProvider(cfg, oldName)
			if !ok {
				return fmt.Errorf("provider %q not found", oldName)
			}

			var name, baseURL, apiKey, model, wireAPI, remark string

			// Detect if any flags were explicitly set
			anyFlagsSet := cmd.Flags().Changed("name") ||
				cmd.Flags().Changed("base-url") ||
				cmd.Flags().Changed("api-key") ||
				cmd.Flags().Changed("model") ||
				cmd.Flags().Changed("wire-api") ||
				cmd.Flags().Changed("remark")

			if anyFlagsSet {
				// Non-interactive: use flags if set, else keep existing
				name = existing.Name
				if cmd.Flags().Changed("name") {
					name = nameFlag
				}
				baseURL = existing.BaseURL
				if cmd.Flags().Changed("base-url") {
					baseURL = baseURLFlag
				}
				apiKey = existing.APIKey
				if cmd.Flags().Changed("api-key") {
					apiKey = apiKeyFlag
				}
				model = existing.Model
				if cmd.Flags().Changed("model") {
					model = modelFlag
				}
				wireAPI = existing.WireAPI
				if cmd.Flags().Changed("wire-api") {
					wireAPI = wireAPIFlag
				}
				remark = existing.Remark
				if cmd.Flags().Changed("remark") {
					remark = remarkFlag
				}
			} else {
				// Interactive: prompt for each field, pre-filling or defaulting to current value
				r := bufio.NewReader(os.Stdin)

				name = promptWithDefault(r, "Provider name", existing.Name)
				baseURL = promptWithDefault(r, "Base URL", existing.BaseURL)
				apiKey = promptWithDefault(r, "API Key", existing.APIKey)
				model = promptWithDefault(r, "Model", existing.Model)
				wireAPI = promptWithDefault(r, "Wire API", existing.WireAPI)
				remark = promptWithDefault(r, "Remark (optional)", existing.Remark)
			}

			// Validate
			if name == "" {
				return fmt.Errorf("name cannot be empty")
			}
			if _, err := url.ParseRequestURI(baseURL); err != nil || !strings.HasPrefix(baseURL, "http") {
				return fmt.Errorf("invalid base_url %q: must be a valid http/https URL", baseURL)
			}
			if apiKey == "" {
				return fmt.Errorf("api_key cannot be empty")
			}
			if model == "" {
				return fmt.Errorf("model cannot be empty")
			}

			updated := config.Provider{
				Name:    name,
				BaseURL: baseURL,
				APIKey:  apiKey,
				Model:   model,
				WireAPI: wireAPI,
				Remark:  remark,
			}

			// Edit in CXC config
			if err := config.EditProvider(cfg, oldName, updated); err != nil {
				return err
			}

			fmt.Printf("✓ Updated provider %q\n", oldName)

			// If it was/is the active provider, re-apply the switch to target config (Codex)
			if cfg.Active == name {
				fmt.Printf("Updating active provider configuration in Codex...\n")
				adapter := codexadapter.New()
				tc := codex2TargetConfig(&updated)
				if err := adapter.Write(&tc); err != nil {
					return fmt.Errorf("updating Codex active config: %w", err)
				}
				fmt.Println("✓ Codex config updated.")
			}

			return nil
		},
	}

	cmd.Flags().StringVar(&nameFlag, "name", "", "New name for the provider")
	cmd.Flags().StringVar(&baseURLFlag, "base-url", "", "New base URL of the API relay endpoint")
	cmd.Flags().StringVar(&apiKeyFlag, "api-key", "", "New API key for authentication")
	cmd.Flags().StringVar(&modelFlag, "model", "", "New model name")
	cmd.Flags().StringVar(&wireAPIFlag, "wire-api", "responses", "New wire API protocol")
	cmd.Flags().StringVar(&remarkFlag, "remark", "", "New remark/note for the provider")

	return cmd
}

// promptWithDefault prompts the user showing the current value as default.
// If input is empty, returns the default value.
func promptWithDefault(r *bufio.Reader, label, defVal string) string {
	fmt.Printf("%s [%s]: ", label, defVal)
	line, _ := r.ReadString('\n')
	line = strings.TrimSpace(line)
	if line == "" {
		return defVal
	}
	return line
}
