package bookmarks

import (
	"errors"
	"io/ioutil"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/claudiodangelis/banco/ui"

	"github.com/claudiodangelis/banco/item"
	"github.com/otiai10/copy"
)

// Bookmarks is the module
type Bookmarks struct{}

// Bookmark is a bookmark
type Bookmark struct {
	Title string
	// TODO: this should be type URL
	URL       string
	Size      int64
	UpdatedAt time.Time
	Group     string
}

// Path of the bookmark
func (b Bookmark) Path() string {
	return filepath.Join("bookmarks", b.Group, b.Title)
}

// Name of the module
func (b Bookmarks) Name() string {
	return "bookmarks"
}

// Singular name of the module
func (b Bookmarks) Singular() string {
	return "bookmark"
}

// Init the module
func (b Bookmarks) Init() error {
	// Create "notes" directory
	if err := os.Mkdir("bookmarks", os.ModePerm); err != nil {
		return err
	}
	return nil
}

// UpdateItemParameters when updating a note
func (b Bookmarks) UpdateItemParameters(current item.Item) []item.Parameter {
	parameters := []item.Parameter{}
	for _, parameter := range b.NewItemParameters() {
		parameter.Default = current[parameter.Name]
		parameters = append(parameters, parameter)
	}
	return parameters
}

// NewItemParameters for a new bookmark
func (b Bookmarks) NewItemParameters() []item.Parameter {
	allGroups, err := groups()
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
			Name:      "URL",
			Default:   "",
			InputType: ui.InputText,
		},
		item.Parameter{
			Name:      "Group",
			Default:   "",
			InputType: ui.InputSelectWithAdd,
			Options:   allGroups,
		},
	}
}

// DeleteItem from disk
func (b Bookmarks) DeleteItem(item item.Item) error {
	return delete(toBookmark(item))
}

// save bookmark to disk
func save(bookmark Bookmark) error {
	if bookmark.Group != "" {
		if err := os.MkdirAll("bookmarks/"+bookmark.Group, os.ModePerm); err != nil {
			return err
		}
	}
	filename := filepath.Join("bookmarks", bookmark.Group, bookmark.Title)
	if _, err := os.Stat(filename); err == nil {
		return errors.New("file already exists")
	} else if !os.IsNotExist(err) {
		return err
	}
	f, err := os.OpenFile(filename, os.O_RDWR|os.O_CREATE, 0644)
	if err != nil {
		return err
	}
	defer f.Close()
	if _, err := f.WriteString(bookmark.URL); err != nil {
		return err
	}
	return nil
}

// UpdateItem updates current item to next item
func (b Bookmarks) UpdateItem(currentItem, nextItem item.Item) error {
	current := toBookmark(currentItem)
	next := toBookmark(nextItem)
	// create next directory
	if err := os.MkdirAll(filepath.Join("bookmarks", next.Group), os.ModePerm); err != nil {
		return err
	}
	// check if file in the next directory already exists with that name
	if _, err := os.Stat(filepath.Join("bookmarks", next.Group, next.Title)); err == nil {
		return errors.New("a bookmark already exists with that name")
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

// delete a bookmark
func delete(bookmark Bookmark) error {
	// TODO: We should have a proper function to check if it exists
	if bookmark.Title == "" {
		return errors.New("bookmark does not exist")
	}
	// Delete the bookmark if it exists
	if err := os.Remove(bookmark.Path()); err != nil {
		return err
	}
	// If directory is empty, delete directory
	contents, err := ioutil.ReadDir(filepath.Dir(bookmark.Path()))
	if err != nil {
		return err
	}
	if len(contents) > 0 {
		// Directory is not empty
		return nil
	}
	// Recursively check if group and its parents are empty, if so, delete them
	dir := filepath.Dir(bookmark.Path())
	for {
		if err := os.Remove(dir); err != nil {
			// TODO: this is not the strongest option
			return nil
		}
		// TODO: This is a bug: what if there is a group called "bookmarks"?
		// TODO: This bug exists in "notes" module as well
		dir = filepath.Dir(dir)
		if dir == "bookmarks" {
			break
		}
	}
	return nil
}

// SaveItem saves the bookmark
func (b Bookmarks) SaveItem(item item.Item) error {
	// Convert to bookmark
	bookmark := toBookmark(item)
	// Save it
	err := save(bookmark)
	return err
}

// groups returns a list of groups
func groups() ([]string, error) {
	bookmarks, err := list()
	if err != nil {
		return []string{}, err
	}
	m := make(map[string]bool)
	for _, bookmark := range bookmarks {
		m[bookmark.Group] = true
	}
	groups := []string{}
	for group := range m {
		groups = append(groups, group)
	}
	return groups, nil
}

// list all items
func list() ([]Bookmark, error) {
	// Read dir
	var bookmarks []Bookmark
	if _, err := os.Stat("bookmarks"); os.IsNotExist(err) {
		return bookmarks, err
	}
	if err := filepath.Walk("bookmarks", func(path string, info os.FileInfo, err error) error {
		if info.IsDir() {
			return nil
		}
		bookmark := Bookmark{
			Title:     info.Name(),
			Size:      info.Size(),
			UpdatedAt: info.ModTime(),
		}
		group := filepath.Dir(strings.TrimPrefix(path, "bookmarks/"))
		if group != "." {
			bookmark.Group = group
		}
		// Read URL
		url, err := ioutil.ReadFile(bookmark.Path())
		if err != nil {
			return err
		}
		bookmark.URL = string(url)
		bookmarks = append(bookmarks, bookmark)
		return nil
	}); err != nil {
		return bookmarks, err
	}
	return bookmarks, nil
}

// OpenItem with $BROWSER
func (b Bookmarks) OpenItem(item item.Item) error {
	bookmark := toBookmark(item)
	browser := os.Getenv("BROWSER")
	if browser == "" {
		return errors.New("$BROWSER is not defined")
	}
	cmd := exec.Command(browser, bookmark.URL)
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	err := cmd.Run()
	return err
}

// toBookmark converts an item to a bookmark
func toBookmark(i item.Item) Bookmark {
	bookmark := Bookmark{}
	bookmark.Title = i["Title"]
	bookmark.Group = i["Group"]
	bookmark.URL = i["URL"]
	return bookmark
}

// toItem converts Bookmark to item.Item
func toItem(bookmark Bookmark) item.Item {
	i := make(item.Item)
	i["Title"] = bookmark.Title
	i["Group"] = bookmark.Group
	i["URL"] = bookmark.URL
	i["Name"] = filepath.Join(bookmark.Group, bookmark.Title)
	return i
}

// all toItemAll converts all Bookmarks to item.Items
func toItemAll(bookmarks []Bookmark) []item.Item {
	items := []item.Item{}
	for _, bookmark := range bookmarks {
		items = append(items, toItem(bookmark))
	}
	return items
}

// List items
func (b Bookmarks) List() ([]item.Item, error) {
	bookmarks, err := list()
	if err != nil {
		return nil, err
	}
	return toItemAll(bookmarks), nil
}

// Module return the module
func Module() Bookmarks {
	return Bookmarks{}
}
