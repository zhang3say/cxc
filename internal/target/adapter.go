// Package target defines the TargetToolAdapter interface and implementations
// for each AI coding tool whose configuration CXC manages.
package target

// Config holds the values CXC writes into a Target Tool's configuration.
type Config struct {
	BaseURL string
	APIKey  string
	Model   string
	WireAPI string
}

// Adapter reads and writes configuration for a specific Target Tool.
// Implementing this interface is all that's needed to add a new Target Tool.
type Adapter interface {
	// Name returns the human-readable name of the Target Tool.
	Name() string
	// Read returns the current active configuration from the Target Tool's config files.
	Read() (*Config, error)
	// Write applies the given Config to the Target Tool's config files.
	// Implementations must create .bak backups before writing.
	Write(cfg *Config) error
}
