package module

import (
	"os"

	"github.com/claudiodangelis/banco/config"
	"github.com/claudiodangelis/banco/item"
	"github.com/claudiodangelis/banco/provider"

	// TODO: this is ugly!
	localtasks "github.com/claudiodangelis/banco/provider/tasks/local"
)

type ModuleName string

const ModuleTasks ModuleName = "tasks"
const ModuleNotes ModuleName = "notes"
const ModuleBookmarks ModuleName = "bookmarks"
const ModuleDocuments ModuleName = "documents"

type Module struct {
	Name      ModuleName
	Providers []provider.Provider
}

func (m Module) ListItems() []item.Item {
	return []item.Item{}
}

func (m Module) Init() error {
	if err := os.Mkdir(string(m.Name), os.ModePerm); err != nil {
		return err
	}
	for _, prv := range m.Providers {
		if err := prv.Sync(); err != nil {
			return err
		}
	}
	return nil
}

func All() []Module {
	var modules []Module
	cfg := config.New()
	for _, m := range []ModuleName{
		ModuleTasks,
		// ModuleNotes,
		// ModuleBookmarks,
		// ModuleDocuments,
	} {
		module := Module{
			Name:      m,
			Providers: getEnabledProviders(m, cfg),
		}
		modules = append(modules, module)
	}
	return modules
}

func New(name ModuleName) Module {
	// TODO: implement this
	cfg := config.New()
	return Module{
		Name:      name,
		Providers: getEnabledProviders(name, cfg),
	}
}

func getEnabledProviders(name ModuleName, cfg config.Config) []provider.Provider {
	var providers []provider.Provider
	if name == ModuleTasks {
		for _, cfgprovider := range cfg.Tasks.Providers {
			var prv provider.Provider
			if !cfgprovider.Disabled {
				// TODO: THIS IS HARDCODED
				if cfgprovider.Provider == "local" {
					prv = localtasks.New("tasks/local", cfgprovider)
				}
				providers = append(providers, prv)
			}

		}
	}
	return providers
}
