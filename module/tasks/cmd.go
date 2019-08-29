package tasks

import (
	"log"

	"github.com/manifoldco/promptui"
	"github.com/otiai10/copy"
	"github.com/spf13/cobra"
)

// CmdSummary returns a single line summary of the module's items
func (b Module) CmdSummary() *cobra.Command {
	return &cobra.Command{
		Use:   "note",
		Short: "Summary",
		Long:  "Summary",
		Run: func(cmd *cobra.Command, args []string) {
		},
	}
}

// CmdRoot sets the root for this command (interactive searching note)
func (b Module) CmdRoot() *cobra.Command {
	return &cobra.Command{
		Use:   "tasks",
		Short: "Manage tasks",
		Long:  "Manage tasks",
		Run: func(cmd *cobra.Command, args []string) {
			task, err := pick()
			if err != nil {
				log.Fatalln(err)
			}
			if err := open(task); err != nil {
				log.Fatalln(err)
			}
		},
	}
}

// CmdUpdate updates a task
func (b Module) CmdUpdate() *cobra.Command {
	return &cobra.Command{
		Use:   "task",
		Short: "updates a task",
		Long:  "updates a task",
		Run: func(cmd *cobra.Command, args []string) {
			var newTask Task
			task, err := pick()
			if err != nil {
				log.Fatalln(err)
			}
			newTask = task
			prompt := promptui.Select{
				Label: "What you want to do?",
				Items: []string{"rename", "change status", "convert to dir"},
			}
			_, result, err := prompt.Run()
			if err != nil {
				log.Fatalln(err)
			}
			if result == "rename" {
				log.Fatalln("not implemented yet")
			} else if result == "change status" {
				// Prompt the new status
				allstatuses, err := statuses()
				if err != nil {
					log.Fatalln(err)
				}
				promptStatus := promptui.Select{
					Label: "Set the new status",
					Items: allstatuses,
				}
				_, status, err := promptStatus.Run()
				if err != nil {
					log.Fatalln(err)
				}
				newTask.Status = status
			}
			// Duplicate the task
			if err := copy.Copy(task.Path(), newTask.Path()); err != nil {
				log.Fatalln(err)
			}
			// Delete old task
			if err := delete(task); err != nil {
				log.Fatalln(err)
			}
		},
	}
}

// CmdNew creates a new task
func (b Module) CmdNew() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "task",
		Short: "Create a new task",
		Long:  "Create a new task",
		Run: func(cmd *cobra.Command, args []string) {
			// TODO: Validate inputs
			// TODO: Implement inputs
			var task Task
			pTitle := promptui.Prompt{
				Label: "Title",
			}
			title, err := pTitle.Run()
			if err != nil {
				log.Fatalln(err)
			}
			task.Title = title
			statuses, err := statuses()
			if err != nil {
				log.Fatalln(err)
			}
			pStatus := promptui.Select{
				Label: "Status",
				Items: statuses,
			}
			_, status, err := pStatus.Run()
			if err != nil {
				log.Fatalln(err)
			}
			task.Status = status
			pIsDir := promptui.Select{
				Label: "Is a directory",
				Items: []string{"No", "Yes"},
			}
			_, isDir, err := pIsDir.Run()
			if err != nil {
				log.Fatalln(err)
			}
			if isDir == "Yes" {
				task.IsDir = true
			}
			if err := create(task); err != nil {
				log.Fatalln(err)
			}
			if err := open(task); err != nil {
				log.Fatalln(err)
			}
		},
	}
	return cmd
}

// CmdList lists tasks
func (b Module) CmdList() *cobra.Command {
	return &cobra.Command{}
}

// CmdDelete deletes a task
func (b Module) CmdDelete() *cobra.Command {
	return &cobra.Command{
		Use:   "task",
		Short: "Deletes a task",
		Long:  "Deletes a task",
		Run: func(cmd *cobra.Command, args []string) {
			task, err := pick()
			if err != nil {
				log.Fatalln(err)
			}
			if err := delete(task); err != nil {
				log.Fatalln(err)
			}
		},
	}
}

// CmdOpen open a task
func (b Module) CmdOpen() *cobra.Command {
	return &cobra.Command{
		Use:   "task",
		Short: "Open a task",
		Long:  "Open a task",
		Run: func(cmd *cobra.Command, args []string) {
			task, err := pick()
			if err != nil {
				log.Fatalln(err)
			}
			if err := open(task); err != nil {
				log.Fatalln(err)
			}
		},
	}
}
