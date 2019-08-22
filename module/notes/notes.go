package notes

import (
	"errors"
	"os"
	"os/exec"
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

// Path of the note
func (n Note) Path() string {
	return filepath.Join("notes", n.Label, n.Title)
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

func open(note Note) error {
	editor := os.Getenv("EDITOR")
	if editor == "" {
		return errors.New("$EDITOR is not defined")
	}
	cmd := exec.Command(editor, note.Path())
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	err := cmd.Run()
	return err
}

func validateLabel(label string) (bool, error) {
	if strings.HasPrefix(label, "/") {
		return false, errors.New(`a label cannot start with "/"`)
	}
	return true, nil
}

// Return a list of notes
func list() ([]Note, error) {
	var notes []Note
	if err := filepath.Walk("notes", func(path string, info os.FileInfo, err error) error {
		if info.IsDir() {
			return nil
		}
		note := Note{
			Title:     info.Name(),
			Size:      info.Size(),
			UpdatedAt: info.ModTime(),
		}
		label := filepath.Dir(strings.TrimPrefix(path, "notes/"))
		if label != "." {
			note.Label = label
		}
		notes = append(notes, note)
		return nil
	}); err != nil {
		return notes, err
	}
	return notes, nil
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
