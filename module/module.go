package module

import (
	"errors"
	"os"
	"os/exec"

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
	Providers map[string]provider.Provider
}

func (m Module) ListItems() ([]item.Item, error) {
	var items []item.Item
	for _, prv := range m.Providers {
		list, err := prv.List()
		if err != nil {
			return items, err
		}
		items = append(items, list...)
	}
	return items, nil
}

func (m Module) OpenItem(item item.Item) error {
	// TODO: implement module-based opening (URLs, documents)
	editor := os.Getenv("EDITOR")
	if editor == "" {
		return errors.New("$EDITOR is not defined")
	}
	cmd := exec.Command(editor, item.Resource)
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	return cmd.Run()
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

// TODO: this function is very poorly written
func getEnabledProviders(name ModuleName, cfg config.Config) map[string]provider.Provider {
	providers := make(map[string]provider.Provider)
	if name == ModuleTasks {
		for _, cfgprovider := range cfg.Tasks.Providers {
			var prv provider.Provider
			if !cfgprovider.Disabled {
				// TODO: THIS IS HARDCODED
				if cfgprovider.Provider == "local" {
					prv = localtasks.New("tasks/local", cfgprovider)
				}
				prvName := prv.Name()
				if cfgprovider.Alias != "" {
					prvName = cfgprovider.Alias
				}
				providers[prvName] = prv
			}
		}
	}
	return providers
}
