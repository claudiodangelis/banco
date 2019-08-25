package tasks

import (
	"errors"
	"os"
	"path/filepath"
	"time"
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

// create a new task
func create(task Task) error {
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

// New module
func New() Module {
	return Module{
		statuses: []string{
			"backlog", "doing", "done",
		},
	}
}
