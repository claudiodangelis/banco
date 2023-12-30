package provider

import (
	"github.com/claudiodangelis/banco/item"
)

type Provider interface {
	// Name of the provider
	Name() string
	// List available items
	List() ([]item.Item, error)
	// Sync is a command to initialize and synchronize provider's data
	Sync() error
	// NewItemParameters is a map of parameters for building an item
	NewItemParameters() []item.Parameter
	// SaveItem to actually create the item
	SaveItem(item.Item) (item.Item, error)
}

type ProviderInstance struct {
	Entrypoint string
	Alias      string
}
