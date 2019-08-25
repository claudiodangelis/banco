package tasks

import (
	"github.com/manifoldco/promptui"
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
	return &cobra.Command{}
}

// CmdUpdate updates a task
func (b Module) CmdUpdate() *cobra.Command {
	return &cobra.Command{}
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
			t := New()
			var task Task
			pTitle := promptui.Prompt{
				Label: "Title",
			}
			title, err := pTitle.Run()
			if err != nil {
				panic(err)
			}
			task.Title = title
			pStatus := promptui.Select{
				Label: "Status",
				Items: t.statuses,
			}
			_, status, err := pStatus.Run()
			if err != nil {
				panic(err)
			}
			task.Status = status
			pIsDir := promptui.Select{
				Label: "Is a directory",
				Items: []string{"No", "Yes"},
			}
			_, isDir, err := pIsDir.Run()
			if err != nil {
				panic(err)
			}
			if isDir == "Yes" {
				task.IsDir = true
			}
			if err := create(task); err != nil {
				panic(err)
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
	return &cobra.Command{}
}

// CmdOpen open a task
func (b Module) CmdOpen() *cobra.Command {
	return &cobra.Command{}
}
