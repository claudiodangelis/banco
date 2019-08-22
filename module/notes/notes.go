package notes

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// Note represent a text note
type Note struct {
	Title     string
	UpdatedAt time.Time
	Size      int64
	Label     string
}

// Module module
type Module struct{}

// Name of this module
func (b Module) Name() string {
	return "notes"
}

// Init initializes the module
func (b Module) Init() error {
	return os.Mkdir("notes", os.ModePerm)
}

// Check sanity of module
func (b Module) Check() error {
	return nil
}

// New module
func New() Module {
	return Module{}
}

func get(title, label string) (Note, error) {
	n := Note{}
	filename := filepath.Join("notes", label, title)
	s, err := os.Stat(filename)
	if err != nil {
		return n, err
	}
	n.Label = label
	n.Title = s.Name()
	n.Size = s.Size()
	n.UpdatedAt = s.ModTime()
	return n, nil
}

func validateLabel(label string) (bool, error) {
	if strings.HasPrefix(label, "/") {
		return false, errors.New(`a label cannot start with "/"`)
	}
	return true, nil
}

func create(title, label string) error {
	if label != "" {
		if err := os.MkdirAll("notes/"+label, os.ModePerm); err != nil {
			return err
		}
	}
	filename := filepath.Join("notes", label, title)
	if _, err := os.Stat(filename); err == nil {
		return errors.New("file already exists")
	} else if !os.IsNotExist(err) {
		return err
	}
	f, err := os.OpenFile(filename, os.O_RDONLY|os.O_CREATE, 0644)
	if err != nil {
		return err
	}
	defer f.Close()
	return nil
}
