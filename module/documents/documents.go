package documents

import (
	"errors"
	"io/ioutil"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"time"

	"github.com/claudiodangelis/banco/ui"

	"github.com/claudiodangelis/banco/item"
	"github.com/otiai10/copy"
)

const (
	filetypePlain    = "plain"
	filetypeLowriter = "lowriter"
	filetypeLocalc   = "localc"
)

// availableTypes is a map of available mime types
// this map is initialized only when user is creating a new item
var availableTypes map[string]string

// types return a list of available types
func types() map[string]string {
	m := make(map[string]string)
	m["Plain text"] = filetypePlain
	// TODO: Implement for other platforms
	if runtime.GOOS != "linux" {
		return m
	}
	// Check if libreoffice or openoffice are installed
	if _, err := exec.LookPath("loffice"); err == nil {
		m["LibreOffice Writer"] = filetypeLowriter
		m["LibreOffice Calc"] = filetypeLocalc
	}
	return m
}

// Documents is the module
type Documents struct{}

// Document is a document
type Document struct {
	Title     string
	Size      int64
	UpdatedAt time.Time
	Directory string
	filetype  string
}

// Path of the document
func (d Document) Path() string {
	return filepath.Join("documents", d.Directory, d.Title)
}

// Name of the module
func (d Documents) Name() string {
	return "documents"
}

// Singular name of the module
func (d Documents) Singular() string {
	return "document"
}

// Init the module
func (d Documents) Init() error {
	// Create "notes" directory
	if err := os.Mkdir("documents", os.ModePerm); err != nil {
		return err
	}
	return nil
}

// UpdateItemParameters when updating a note
func (d Documents) UpdateItemParameters(current item.Item) []item.Parameter {
	parameters := []item.Parameter{}
	for _, parameter := range d.NewItemParameters() {
		if parameter.Name == "Type" {
			continue
		}
		parameter.Default = current[parameter.Name]
		parameters = append(parameters, parameter)
	}
	return parameters
}

// NewItemParameters for a new document
func (d Documents) NewItemParameters() []item.Parameter {
	availableTypes = types()
	types := []string{}
	for t := range availableTypes {
		types = append(types, t)
	}
	allDirs, err := dirs()
	if err != nil {
		panic(err)
	}
	return []item.Parameter{
		item.Parameter{
			Name:      "Title",
			Default:   "",
			InputType: ui.InputText,
		},
		item.Parameter{
			Name:      "Directory",
			Default:   "",
			InputType: ui.InputSelectWithAdd,
			Options:   allDirs,
		},
		item.Parameter{
			Name:      "Type",
			Default:   "",
			InputType: ui.InputSelect,
			Options:   types,
		},
	}
}

// DeleteItem from disk
func (d Documents) DeleteItem(item item.Item) error {
	return delete(toDocument(item))
}

// save document to disk
func save(document Document) error {
	if document.Directory != "" {
		if err := os.MkdirAll("documents/"+document.Directory, os.ModePerm); err != nil {
			return err
		}
	}
	filename := filepath.Join("documents", document.Directory, document.Title)
	if _, err := os.Stat(filename); err == nil {
		return errors.New("file already exists")
	} else if !os.IsNotExist(err) {
		return err
	}
	if document.filetype == filetypePlain {
		f, err := os.OpenFile(filename, os.O_RDONLY|os.O_CREATE, 0644)
		if err != nil {
			return err
		}
		defer f.Close()
	} else if document.filetype == filetypeLowriter {
		// Create temporary file in temporary dir
		tmpdir, err := ioutil.TempDir(os.TempDir(), "banco-")
		if err != nil {
			return err
		}
		f, err := os.OpenFile(filepath.Join(tmpdir, document.Title), os.O_RDONLY|os.O_CREATE, 0644)
		if err != nil {
			return err
		}
		defer f.Close()
		defer os.Remove(tmpdir)
		// Convert the temporary file using --convert-to odt
		cmd := exec.Command("loffice", "--headless", "--convert-to", "odt", "--outdir", filepath.Join("documents", document.Directory), filepath.Join(tmpdir, document.Title))
		if err := cmd.Run(); err != nil {
			return err
		}
	} else if document.filetype == filetypeLocalc {
		// Create temporary file in temporary dir
		tmpdir, err := ioutil.TempDir(os.TempDir(), "banco-")
		if err != nil {
			return err
		}
		f, err := os.OpenFile(filepath.Join(tmpdir, document.Title), os.O_RDONLY|os.O_CREATE, 0644)
		if err != nil {
			return err
		}
		defer f.Close()
		defer os.Remove(tmpdir)
		// Convert the temporary file using --convert-to odt
		cmd := exec.Command("loffice", "--headless", "--convert-to", "ods:calc8", "--outdir", filepath.Join("documents", document.Directory), filepath.Join(tmpdir, document.Title))
		if err := cmd.Run(); err != nil {
			return err
		}
	}
	return nil
}

// UpdateItem updates current item to next item
func (d Documents) UpdateItem(currentItem, nextItem item.Item) error {
	current := toDocument(currentItem)
	next := toDocument(nextItem)
	// create next directory
	if err := os.MkdirAll(filepath.Join("documents", next.Directory), os.ModePerm); err != nil {
		return err
	}
	// check if file in the next directory already exists with that name
	if _, err := os.Stat(filepath.Join("documents", next.Directory, next.Title)); err == nil {
		return errors.New("a document already exists with that name")
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

// delete a document
func delete(document Document) error {
	// TODO: We should have a proper function to check if it exists
	if document.Title == "" {
		return errors.New("document does not exist")
	}
	// Delete the document if it exists
	if err := os.Remove(document.Path()); err != nil {
		return err
	}
	// If directory is empty, delete directory
	contents, err := ioutil.ReadDir(filepath.Dir(document.Path()))
	if err != nil {
		return err
	}
	if len(contents) > 0 {
		// Directory is not empty
		return nil
	}
	// Recursively check if directory and its parents are empty, if so, delete them
	dir := filepath.Dir(document.Path())
	for {
		if err := os.Remove(dir); err != nil {
			// TODO: this is not the strongest option
			return nil
		}
		// TODO: This is a bug: what if there is a directory called "documents"?
		// TODO: This bug exists in "notes" module as well
		dir = filepath.Dir(dir)
		if dir == "documents" {
			break
		}
	}
	return nil
}

// SaveItem saves the document
func (d Documents) SaveItem(item item.Item) error {
	// Convert to document
	document := toDocument(item)
	// Save it
	err := save(document)
	return err
}

// dirs returns a list of dirs
func dirs() ([]string, error) {
	documents, err := list()
	if err != nil {
		return []string{}, err
	}
	m := make(map[string]bool)
	for _, document := range documents {
		m[document.Directory] = true
	}
	dirs := []string{}
	for dir := range m {
		dirs = append(dirs, dir)
	}
	return dirs, nil
}

// list all items
func list() ([]Document, error) {
	// Read dir
	var documents []Document
	if _, err := os.Stat("documents"); os.IsNotExist(err) {
		return documents, err
	}
	if err := filepath.Walk("documents", func(path string, info os.FileInfo, err error) error {
		if info.IsDir() {
			return nil
		}
		document := Document{
			Title:     info.Name(),
			Size:      info.Size(),
			UpdatedAt: info.ModTime(),
		}
		dir := filepath.Dir(strings.TrimPrefix(path, "documents/"))
		if dir != "." {
			document.Directory = dir
		}
		documents = append(documents, document)
		return nil
	}); err != nil {
		return documents, err
	}
	return documents, nil
}

// OpenItem with xdg-open or alternative
func (d Documents) OpenItem(item item.Item) error {
	document := toDocument(item)
	if runtime.GOOS != "linux" {
		return errors.New("this feature is only available for linux at the moment")
	}
	// TODO: This should be configurable
	opener := "xdg-open"
	cmd := exec.Command(opener, document.Path())
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	err := cmd.Run()
	return err
}

// toDocument converts an item to a document
func toDocument(i item.Item) Document {
	document := Document{}
	document.Title = i["Title"]
	document.Directory = i["Directory"]
	document.filetype = availableTypes[i["Type"]]
	// Append suffix
	if document.filetype == filetypeLowriter && !strings.HasSuffix(".odt", document.Title) {
		document.Title = document.Title + ".odt"
	} else if document.filetype == filetypeLocalc && !strings.HasSuffix(".ods", document.Title) {
		document.Title = document.Title + ".ods"
	}
	return document
}

// toItem converts document to item.Item
func toItem(document Document) item.Item {
	i := make(item.Item)
	i["Title"] = document.Title
	i["Directory"] = document.Directory
	i["Name"] = filepath.Join(document.Directory, document.Title)
	return i
}

// all toItemAll converts all documents to item.Items
func toItemAll(documents []Document) []item.Item {
	items := []item.Item{}
	for _, document := range documents {
		items = append(items, toItem(document))
	}
	return items
}

// List items
func (d Documents) List() ([]item.Item, error) {
	documents, err := list()
	if err != nil {
		return nil, err
	}
	return toItemAll(documents), nil
}

// Module return the module
func Module() Documents {
	return Documents{}
}
