package bookmarks

import "github.com/spf13/cobra"
// CmdSummary returns a single line summary of the module's items
func (b Module) CmdSummary() *cobra.Command {
	return nil
}
// CmdRoot sets the root for this command (interactive searching note)
func (b Module) CmdRoot() *cobra.Command {
	return nil
}

// CmdUpdate updates a bookmark
func (b Module) CmdUpdate() *cobra.Command {
	return nil
}

// CmdNew creates a new bookmark
func (b Module) CmdNew() *cobra.Command {
	return nil
}

// CmdList lists bookmarks
func (b Module) CmdList() *cobra.Command {
	return nil
}

// CmdDelete deletes a bookmark
func (b Module) CmdDelete() *cobra.Command {
	return nil
}

// CmdOpen open a bookmark
func (b Module) CmdOpen() *cobra.Command {
	return nil
}
