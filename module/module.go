package module

import (
	"github.com/claudiodangelis/banco/item"
	"github.com/claudiodangelis/banco/module/bookmarks"
	"github.com/claudiodangelis/banco/module/documents"
	"github.com/claudiodangelis/banco/module/notes"
	"github.com/claudiodangelis/banco/module/tasks"
)

// Module for banco
type Module interface {
	// Name of the module
	Name() string
	// Aliases of the module
	Aliases() []string
	// Singular name of the module
	Singular() string
	// NewItemParameters to be input when creating a new item
	NewItemParameters() []item.Parameter
	// UpdateItemParameters to be input when updating an item
	UpdateItemParameters(item.Item) []item.Parameter
	// SaveItem stores a new item
	SaveItem(item.Item) error
	// OpenItem opens the item
	OpenItem(item.Item) error
	// UpdateItem updates current item to next item
	UpdateItem(current, next item.Item) error
	// DeleteItem from the module folder
	DeleteItem(item.Item) error
	// Init initializes the module
	Init() error
	// List items
	List() ([]item.Item, error)
	// Summary of the module
	Summary() string
	// Wether or not the module supports templating
	HasTemplates() bool
}

// All modules
func All() []Module {
	return []Module{
		notes.Module(),
		tasks.Module(),
		bookmarks.Module(),
		documents.Module(),
	}
}

// All module names
func AllNamesWithTemplates() []string {
	var names []string
	for _, module := range All() {
		if module.HasTemplates() {
			names = append(names, module.Name())
		}
	}
	return names
}
