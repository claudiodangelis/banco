package notes

import (
	"errors"
	"fmt"
	"io/ioutil"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/otiai10/copy"
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

// summary of the module's items
func summary() (string, error) {
	// Get all notes
	notes, err := list()
	if err != nil {
		return "", err
	}
	labels := make(map[string]int)
	for _, note := range notes {
		if note.Label != "" {
			labels[note.Label]++
		}
	}
	return fmt.Sprintf("Notes: %d, Labels: %d", len(notes), len(labels)), nil
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

// Rename a note
func rename(current, next Note) error {
	// create next directory
	if err := os.MkdirAll(filepath.Join("notes", next.Label), os.ModePerm); err != nil {
		return err
	}
	// check if file in the next directory already exists with that name
	if _, err := os.Stat(filepath.Join("notes", next.Label, next.Title)); err == nil {
		return errors.New("a note already exists with that name")
	}
	// copy current note to the next directory
	if err := copy.Copy(current.Path(), next.Path()); err != nil {
		return err
	}
	// delete old file (and it's parent, if empty)
	if err := delete(current); err != nil {
		return err
	}
	return nil
}

// Return a list of notes
func list() ([]Note, error) {
	var notes []Note
	if _, err := os.Stat("notes"); os.IsNotExist(err) {
		return nil, err
	}
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

func delete(note Note) error {
	// TODO: We should have a proper function to check if it exists
	if note.Title == "" {
		return errors.New("note does not exist")
	}
	// Delete the note if it exists
	if err := os.Remove(note.Path()); err != nil {
		return err
	}
	// If directory is empty, delete directory
	contents, err := ioutil.ReadDir(filepath.Dir(note.Path()))
	if err != nil {
		return err
	}
	if len(contents) > 0 {
		// Directory is not empty
		return nil
	}
	// Recursively check if label and its parents are empty, if so, delete them
	dir := filepath.Dir(note.Path())
	for {
		if err := os.Remove(dir); err != nil {
			// TODO: this is not the strongest option
			return nil
		}
		dir = filepath.Dir(dir)
		if dir == "notes" {
			break
		}
	}
	return nil
}
