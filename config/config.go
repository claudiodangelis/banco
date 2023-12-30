package config

// TODO: This is hardcoded
type Config struct {
	Tasks struct {
		Providers []ProviderConfig
	}
}

type ProviderConfig struct {
	Provider string
	Disabled bool
	Alias    string
}

func New() Config {
	var providers []ProviderConfig
	providers = append(providers, ProviderConfig{
		Provider: "local",
		Disabled: false,
	})
	return Config{
		Tasks: struct{ Providers []ProviderConfig }{
			Providers: providers,
		},
	}
}
