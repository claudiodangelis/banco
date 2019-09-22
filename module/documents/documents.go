package documents

import (
	"os"
	"time"
)

// Document represents a document
type Document struct {
	// TODO: or maybe "Title"?
	Name      string
	Directory string
	MimeType  string
	CreatedAt time.Time
	UpdatedAt time.Time
}

// Module module
type Module struct{}

// Name of this module
func (b Module) Name() string {
	return "documents"
}

// Init initializes the module
func (b Module) Init() error {
	// Create "documents" directory
	if err := os.Mkdir("documents", os.ModePerm); err != nil {
		return err
	}
	return nil
}

// Check checks module's sanity
func (b Module) Check() error {
	return nil
}

// New module
func New() Module {
	return Module{}
}

// list documents
func list() ([]Document, error) {
	documents := []Document{}
	return documents, nil
}
