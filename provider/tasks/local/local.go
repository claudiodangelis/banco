package local

import (
	"errors"
	"os"
	"path/filepath"

	"github.com/claudiodangelis/banco/config"
	"github.com/claudiodangelis/banco/item"
	"github.com/claudiodangelis/banco/provider"
	"github.com/claudiodangelis/banco/ui"
)

type Task struct {
	Title  string
	Status string
	IsDir  bool
}

func toTask(item item.Item) Task {
	isdir := false
	if item.Parameters["IsDir"] == "Yes" {
		isdir = true
	}
	return Task{
		Title:  item.Parameters["Title"],
		Status: item.Parameters["Status"],
		IsDir:  isdir,
	}
}

func toItem(task Task) item.Item {
	isdir := "No"
	if task.IsDir {
		isdir = "Yes"
	}
	return item.Item{
		Parameters: map[string]string{
			"Title":  task.Title,
			"Status": task.Status,
			"IsDir":  isdir,
		},
	}
}

// TODO: list of statuses should be configurable
// TODO: this list should only be used when initializing the module
var statuses = []string{"backlog", "doing", "done"}

// TODO: We may find another name for this as it's only used internally?
type LocalTaskProvider provider.ProviderInstance

func (p LocalTaskProvider) Name() string {
	return "local"
}

func (p LocalTaskProvider) NewItemParameters() []item.Parameter {
	return []item.Parameter{
		{
			Name:      "Title",
			InputType: ui.InputText,
			// TODO: implement default values
			Default: "",
		},
		{
			Name:      "Status",
			InputType: ui.InputSelectWithAdd,
			Options:   statuses,
			Default:   statuses[0],
		},
		{
			Name:      "Is a directory",
			InputType: ui.InputSelect,
			Options:   []string{"Yes", "No"},
			Default:   "No",
		},
	}
}

func (p LocalTaskProvider) SaveItem(item item.Item) (item.Item, error) {
	task := toTask(item)
	if task.Status == "" || task.Title == "" {
		// TODO: Add a proper error message
		return item, errors.New("invalid task")
	}
	// If it's a dir, create it
	filename := filepath.Join(p.Entrypoint, task.Status, task.Title)
	if task.IsDir {
		if err := os.MkdirAll(filepath.Join(p.Entrypoint, task.Status, task.Title), os.ModePerm); err != nil {
			return item, err
		}
		// TODO: What should be the name of the default file?
		filename = filepath.Join(p.Entrypoint, task.Status, task.Title, "task")
	}
	if _, err := os.Stat(filename); err == nil {
		return item, errors.New("file already exists")
	} else if !os.IsNotExist(err) {
		return item, err
	}
	f, err := os.OpenFile(filename, os.O_RDONLY|os.O_CREATE, 0644)
	if err != nil {
		return item, err
	}
	defer f.Close()
	item.Resource = filepath.Join(p.Entrypoint, task.Status, task.Title)
	return item, nil
}

func (p LocalTaskProvider) List() ([]item.Item, error) {
	var results []item.Item
	statuses, err := os.ReadDir(p.Entrypoint)
	if err != nil {
		return results, err
	}
	for _, status := range statuses {
		// Should we spot here non-dirs and warn the user?
		if !status.IsDir() {
			continue
		}
		tasks, err := os.ReadDir(filepath.Join(p.Entrypoint, status.Name()))
		if err != nil {
			return results, err
		}
		for _, task := range tasks {
			// Populate item
			// TODO: should we have some util here?
			results = append(results, item.Item{
				Parameters: map[string]string{
					"Title":  task.Name(),
					"Status": status.Name(),
					"IsDir":  boolToYesNo(task.IsDir()),
				},
			})
		}
	}
	return results, nil
}

func (p LocalTaskProvider) Sync() error {
	// Create default tasks directories
	for _, status := range statuses {
		dir := filepath.Join(p.Entrypoint, status)
		if err := os.MkdirAll(dir, os.ModePerm); err != nil {
			return err
		}
	}
	return nil
}

func New(entrypoint string, cfg config.ProviderConfig) LocalTaskProvider {
	return LocalTaskProvider{
		// TODO: Should we check if entrypoint exists?
		Entrypoint: entrypoint,
		// TODO: do we need to know what's the alias?
		Alias: cfg.Alias,
	}
}
