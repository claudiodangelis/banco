package provider

import (
	"github.com/claudiodangelis/banco/item"
)

type Provider interface {
	// List available items
	List() ([]item.Item, error)
}

type ProviderInstance struct {
	Entrypoint string
}
