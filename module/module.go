package module

import (
	"github.com/claudiodangelis/banco/module/notes"
	"github.com/claudiodangelis/banco/module/tasks"
	"github.com/spf13/cobra"
)

// Module is a module
type Module interface {
	CmdRoot() *cobra.Command
	CmdSummary() *cobra.Command
	CmdUpdate() *cobra.Command
	CmdList() *cobra.Command
	CmdDelete() *cobra.Command
	CmdOpen() *cobra.Command
	CmdNew() *cobra.Command
	Name() string
	Init() error
	Check() error
}

// All modules
func All() []Module {
	return []Module{
		notes.New(),
		// bookmarks.New(),
		// repos.New(),
		tasks.New(),
		// documents.New(),
	}
}
