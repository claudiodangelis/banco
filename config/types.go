package config

type NewConfig struct {
	Notes struct {
		Title string
	}
	Tasks struct {
		Title     string
		Providers []TasksProvider
	}
	Bookmarks struct {
		Browser struct {
			Cmd  string
			Args []string
		}
	}
}

type TasksProvider struct {
	Provider   string            `yaml:"provider"`
	Name       string            `yaml:"name"`
	Parameters map[string]string `yaml:"parameters"`
}
