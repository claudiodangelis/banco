package documents

import (
	"os"
	"path/filepath"
	"strings"
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
	Size      int64
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

// sort documents
func sort(documents []Document) []Document {
	// TODO: Not implemented yet
	return documents
}

// list documents
func list() ([]Document, error) {
	documents := []Document{}
	// Read directory
	if _, err := os.Stat("documents"); os.IsNotExist(err) {
		return nil, err
	}
	if err := filepath.Walk("documents", func(path string, info os.FileInfo, err error) error {
		if info.IsDir() {
			return nil
		}
		document := Document{
			Name:      info.Name(),
			UpdatedAt: info.ModTime(),
			Size:      info.Size(),
		}
		directory := filepath.Dir(strings.TrimPrefix(path, "documents/"))
		if directory != "." {
			document.Directory = directory
		}
		documents = append(documents, document)
		return nil
	}); err != nil {
		return documents, err
	}
	return documents, nil
}
