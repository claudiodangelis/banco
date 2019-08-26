package tasks

import (
	"errors"
	"fmt"
	"io/ioutil"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/manifoldco/promptui"
)

// Module module
type Module struct {
	// TODO: Not sure I want to have this here
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

// Path to the task
func (t Task) Path() string {
	return filepath.Join("tasks", t.Status, t.Title)
}

// Name of this module
func (b Module) Name() string {
	return "tasks"
}

// Init initializes the module
func (b Module) Init() error {
	// Create "tasks" directory
	if err := os.Mkdir("tasks", os.ModePerm); err != nil {
		return err
	}
	// Create statuses directories
	for _, status := range b.statuses {
		if err := os.Mkdir(filepath.Join("tasks", status), os.ModePerm); err != nil {
			return err
		}
	}
	return nil
}

// Check checks module's sanity
func (b Module) Check() error {
	return nil
}

// TODO: Move this function to tasks/util.go
func taskPicker() (Task, error) {
	// TODO: Lots of duplicate code here, investigate if there is a solution
	var items []string
	tasks, err := list()
	if err != nil {
		return Task{}, err
	}
	mapped := make(map[string]Task)
	for _, task := range tasks {
		p := fmt.Sprintf("[%s] %s", task.Status, task.Title)
		mapped[p] = task
		items = append(items, p)
	}
	prompt := promptui.Select{
		Label: "Choose task",
		Items: items,
		Size:  100,
		Searcher: func(input string, index int) bool {
			return strings.Contains(items[index], input)
		},
		StartInSearchMode: true,
	}
	_, result, err := prompt.Run()
	return mapped[result], err
}

// list tasks
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

// create a new task
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
	if err := os.Remove(task.Path()); err != nil {
		return err
	}
	// If directory is empty, delete directory
	contents, err := ioutil.ReadDir(filepath.Dir(task.Path()))
	if err != nil {
		return err
	}
	if len(contents) > 0 {
		// Directory is not empty
		return nil
	}
	// Recursively check if label and its parents are empty, if so, delete them
	dir := filepath.Dir(task.Path())
	for {
		if err := os.Remove(dir); err != nil {
			// TODO: this is not the strongest option
			return nil
		}
		dir = filepath.Dir(dir)
		if dir == "tasks" {
			break
		}
	}
	return nil
}

// New module
func New() Module {
	return Module{
		statuses: []string{
			"backlog", "doing", "done",
		},
	}
}
