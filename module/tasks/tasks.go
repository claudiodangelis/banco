package tasks

import (
	"errors"
	"fmt"
	"io/ioutil"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/claudiodangelis/banco/config"
	"github.com/claudiodangelis/banco/item"
	"github.com/claudiodangelis/banco/ui"
	"github.com/otiai10/copy"
)

// Tasks is the module
type Tasks struct {
	statuses []string
}

// Task is a task
type Task struct {
	Title     string
	Status    string
	IsDir     bool
	UpdatedAt time.Time
	Size      int64
}

// Aliases of the module
func (t Tasks) Aliases() []string {
	return []string{"t"}
}

// Path to the task
func (t Task) Path() string {
	return filepath.Join("tasks", t.Status, t.Title)
}

func list() ([]Task, error) {
	var tasks []Task
	allstatuses, err := statuses()
	if err != nil {
		return tasks, err
	}
	for _, status := range allstatuses {
		dir, err := ioutil.ReadDir(filepath.Join("tasks", status))
		if err != nil {
			return tasks, err
		}
		for _, info := range dir {
			tasks = append(tasks, Task{
				Title:  info.Name(),
				IsDir:  info.IsDir(),
				Status: status,
				// TODO: Implement other properties
			})
		}
	}
	return tasks, nil
}
func create(task Task) error {
	if task.Status == "" || task.Title == "" {
		// TODO: Add a proper error message
		return errors.New("invalid task")
	}
	// If it's a dir, create it
	filename := filepath.Join("tasks", task.Status, task.Title)
	if task.IsDir {
		if err := os.MkdirAll(filepath.Join("tasks", task.Status, task.Title), os.ModePerm); err != nil {
			return err
		}
		// TODO: What should be the name of the default file?
		filename = filepath.Join("tasks", task.Status, task.Title, "task")
	}
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

// open a task
func open(task Task) error {
	editor := os.Getenv("EDITOR")
	if editor == "" {
		return errors.New("$EDITOR is not defined")
	}
	cmd := exec.Command(editor, task.Path())
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	err := cmd.Run()
	return err
}

// delete a task
func delete(task Task) error {
	// TODO: Lots of duplicate code here
	if task.Title == "" {
		return errors.New("task does not exist")
	}
	// Delete the task if it exists
	if err := os.RemoveAll(filepath.Join("tasks", task.Status, task.Title)); err != nil {
		return err
	}
	return nil
}

// Name of the module
func (t Tasks) Name() string {
	return "tasks"
}

// Singular name of the module
func (t Tasks) Singular() string {
	return "task"
}

// statuses get the list of statuses
func statuses() ([]string, error) {
	var statuses []string
	contents, err := ioutil.ReadDir("tasks")
	if err != nil {
		return statuses, err
	}
	for _, content := range contents {
		if content.IsDir() {
			statuses = append(statuses, content.Name())
		}
	}
	return statuses, nil
}

// UpdateItemParameters when updating a note
func (t Tasks) UpdateItemParameters(current item.Item) []item.Parameter {
	parameters := []item.Parameter{}
	for _, parameter := range t.NewItemParameters() {
		parameter.Default = current[parameter.Name]
		parameters = append(parameters, parameter)
	}
	return parameters
}

// NewItemParameters to be input when creating an item
func (t Tasks) NewItemParameters() []item.Parameter {
	statuses, err := statuses()
	if err != nil {
		panic(err)
	}
	ts, err := t.List()
	if err != nil {
		panic(err)
	}
	cfg := config.New()
	return []item.Parameter{
		item.Parameter{
			Name:      "Title",
			InputType: ui.InputText,
			Default:   cfg.GetDefaultTitle(t.Name(), ts),
		},
		item.Parameter{
			Name:      "Status",
			InputType: ui.InputSelectWithAdd,
			Options:   statuses,
			Default:   statuses[0],
		},
		item.Parameter{
			Name:      "Is a directory",
			InputType: ui.InputSelect,
			Options:   []string{"Yes", "No"},
			Default:   "No",
		},
	}
}

func toTask(item item.Item) Task {
	task := Task{}
	task.Title = item["Title"]
	task.Status = item["Status"]
	task.IsDir = item["Is a directory"] == "Yes"
	return task
}

func toItem(task Task) item.Item {
	item := make(item.Item)
	item["Title"] = task.Title
	item["Status"] = task.Status
	if task.IsDir {
		item["Is a directory"] = "Yes"
	} else {
		item["Is a directory"] = "No"
	}
	item["Name"] = fmt.Sprintf("[%s] %s", task.Status, task.Title)
	return item
}

// SaveItem stores a new item
func (t Tasks) SaveItem(item item.Item) error {
	return create(toTask(item))
}

// OpenItem opens the item
func (t Tasks) OpenItem(item item.Item) error {
	return open(toTask(item))
}

// UpdateItem updates current item to next item
func (t Tasks) UpdateItem(currentItem, nextItem item.Item) error {
	current := toTask(currentItem)
	next := toTask(nextItem)
	if err := copy.Copy(current.Path(), next.Path()); err != nil {
		log.Fatalln(err)
	}
	// Delete old task
	if err := delete(current); err != nil {
		log.Fatalln(err)
	}
	return nil
}

// DeleteItem from the module folder
func (t Tasks) DeleteItem(item item.Item) error {
	return delete(toTask(item))
}

// Init initializes the module
func (t Tasks) Init() error {
	// Create "tasks" directory
	if err := os.Mkdir("tasks", os.ModePerm); err != nil {
		return err
	}
	// Create statuses directories
	for _, status := range t.statuses {
		if err := os.Mkdir(filepath.Join("tasks", status), os.ModePerm); err != nil {
			return err
		}
	}
	return nil
}

// List items
func (t Tasks) List() ([]item.Item, error) {
	tasks, err := list()
	if err != nil {
		return nil, err
	}
	items := []item.Item{}
	for _, task := range tasks {
		items = append(items, toItem(task))
	}
	return items, nil
}

// Summary of items
func (t Tasks) Summary() string {
	// TODO: Implement this
	statuses, err := statuses()
	if err != nil {
		panic(err)
	}
	var summary []string
	for _, status := range statuses {
		files, err := ioutil.ReadDir(filepath.Join("tasks", status))
		if err != nil {
			panic(err)
		}
		summary = append(summary, fmt.Sprintf("%s: %d", status, len(files)))
	}
	return fmt.Sprintf("tasks [%s]", strings.Join(summary, ","))
}

// Module return the module
func Module() Tasks {
	return Tasks{
		// TODO: This should be configurable
		statuses: []string{"backlog", "doing", "done"},
	}
}
