package notes

import (
	"errors"
	"fmt"
	"io/ioutil"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/claudiodangelis/banco/ui"

	"github.com/claudiodangelis/banco/item"
	"github.com/otiai10/copy"
)

// Notes is the module
type Notes struct{}

// Note is a note
type Note struct {
	Title     string
	Size      int64
	UpdatedAt time.Time
	Label     string
}

// Aliases of the module
func (n Notes) Aliases() []string {
	return []string{"n"}
}

// Path of the note
func (n Note) Path() string {
	return filepath.Join("notes", n.Label, n.Title)
}

// Name of the module
func (n Notes) Name() string {
	return "notes"
}

// Singular name of the module
func (n Notes) Singular() string {
	return "note"
}

// Init the module
func (n Notes) Init() error {
	// Create "notes" directory
	if err := os.Mkdir("notes", os.ModePerm); err != nil {
		return err
	}
	return nil
}

// UpdateItemParameters when updating a note
func (n Notes) UpdateItemParameters(current item.Item) []item.Parameter {
	parameters := []item.Parameter{}
	for _, parameter := range n.NewItemParameters() {
		parameter.Default = current[parameter.Name]
		parameters = append(parameters, parameter)
	}
	return parameters
}

// NewItemParameters for a new note
func (n Notes) NewItemParameters() []item.Parameter {
	labels, err := labels()
	if err != nil {
		panic(err)
	}
	return []item.Parameter{
		item.Parameter{
			Name: "Title",
			// TODO: This must be configurable
			Default:   time.Now().Format("20060102"),
			InputType: ui.InputText,
		},
		item.Parameter{
			Name:      "Label",
			Default:   "",
			InputType: ui.InputSelectWithAdd,
			Options:   labels,
		},
	}
}

// DeleteItem from disk
func (n Notes) DeleteItem(item item.Item) error {
	return delete(toNote(item))
}

// save note to disk
func save(note Note) error {
	if note.Label != "" {
		if err := os.MkdirAll("notes/"+note.Label, os.ModePerm); err != nil {
			return err
		}
	}
	filename := filepath.Join("notes", note.Label, note.Title)
	if _, err := os.Stat(filename); err == nil {
		return errors.New("file already exists")
	} else if !os.IsNotExist(err) {
		return err
	}
	// Check if a template exists
	f, err := os.OpenFile(filename, os.O_RDONLY|os.O_CREATE, 0644)
	if err != nil {
		return err
	}
	defer f.Close()
	return nil
}

// UpdateItem updates current item to next item
func (n Notes) UpdateItem(currentItem, nextItem item.Item) error {
	current := toNote(currentItem)
	next := toNote(nextItem)
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

// delete a note
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

// SaveItem saves the note
func (n Notes) SaveItem(item item.Item) error {
	// Convert to note
	note := toNote(item)
	// Save it
	err := save(note)
	return err
}

// labels returns a list of labels
func labels() ([]string, error) {
	notes, err := list()
	if err != nil {
		return []string{}, err
	}
	m := make(map[string]bool)
	for _, note := range notes {
		m[note.Label] = true
	}
	l := []string{}
	for label := range m {
		l = append(l, label)
	}
	sort.Strings(l)
	return l, nil
}

// list all items
func list() ([]Note, error) {
	// Read dir
	var notes []Note
	if _, err := os.Stat("notes"); os.IsNotExist(err) {
		return notes, err
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

// OpenItem with $EDITOR
func (n Notes) OpenItem(item item.Item) error {
	note := toNote(item)
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

// toNote converts an item to a nOte
func toNote(i item.Item) Note {
	note := Note{}
	note.Label = i["Label"]
	note.Title = i["Title"]
	return note
}

// toItem converts Note to item.Item
func toItem(n Note) item.Item {
	i := make(item.Item)
	i["Label"] = n.Label
	i["Title"] = n.Title
	i["Name"] = filepath.Join(n.Label, n.Title)
	return i
}

// all toItemAll converts all Notes to item.Items
func toItemAll(n []Note) []item.Item {
	items := []item.Item{}
	for _, note := range n {
		items = append(items, toItem(note))
	}
	return items
}

// List items
func (n Notes) List() ([]item.Item, error) {
	all, err := list()
	if err != nil {
		return nil, err
	}
	return toItemAll(all), nil
}

// Summary of items
func (n Notes) Summary() string {
	list, err := list()
	if err != nil {
		panic(err)
	}
	labels := make(map[string]bool)
	for _, note := range list {
		labels[note.Label] = true
	}
	return fmt.Sprintf("notes [items: %d, labels: %d]", len(list), len(labels))
}

// Module return the module
func Module() Notes {
	return Notes{}
}
