package provider

import (
	"github.com/claudiodangelis/banco/item"
)

type Provider interface {
	// List available items
	List() ([]item.Item, error)
	// Sync is a command to initialize and synchronize provider's data
	Sync() error
}

type ProviderInstance struct {
	Entrypoint string
}
