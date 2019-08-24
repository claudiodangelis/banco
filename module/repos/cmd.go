package repos

import "github.com/spf13/cobra"
// CmdSummary returns a single line summary of the module's items
func (b Module) CmdSummary() *cobra.Command {
	return nil
}
// CmdRoot sets the root for this command (interactive searching note)
func (b Module) CmdRoot() *cobra.Command {
	return nil
}

// CmdUpdate updates a repo
func (b Module) CmdUpdate() *cobra.Command {
	return nil
}

// CmdNew creates a new repo
func (b Module) CmdNew() *cobra.Command {
	return nil
}

// CmdList lists repos
func (b Module) CmdList() *cobra.Command {
	return nil
}

// CmdDelete deletes a repo
func (b Module) CmdDelete() *cobra.Command {
	return nil
}

// CmdOpen open a repo
func (b Module) CmdOpen() *cobra.Command {
	return nil
}
