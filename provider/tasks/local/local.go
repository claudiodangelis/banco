package local

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/claudiodangelis/banco/config"
	"github.com/claudiodangelis/banco/item"
	"github.com/claudiodangelis/banco/provider"
)

// TODO: list of statuses should be configurable
var statuses = []string{"backlog", "doing", "done"}

// TODO: We may find another name for this as it's only used internally?
type LocalTaskProvider provider.ProviderInstance

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
				"Title":  task.Name(),
				"Status": status.Name(),
				"IsDir":  boolToYesNo(task.IsDir()),
			})
		}
	}
	return results, nil
}

func (p LocalTaskProvider) Sync() error {
	fmt.Println("showroom dummies")
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
	}
}
