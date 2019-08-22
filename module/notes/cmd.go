package notes

import (
	"log"
	"strings"

	"github.com/claudiodangelis/banco/util"
	"github.com/spf13/cobra"
)

// CmdUpdate updates a note
func (b Module) CmdUpdate() *cobra.Command {
	return nil
}

// CmdList lists notes
func (b Module) CmdList() *cobra.Command {
	return nil
}

// CmdDelete deletes a note
func (b Module) CmdDelete() *cobra.Command {
	return nil
}

// CmdOpen open a note
func (b Module) CmdOpen() *cobra.Command {
	return nil
}

// CmdCheck checks module's sanity
func (b Module) CmdCheck() *cobra.Command {
	return nil
}

var interactive bool
var title string
var label string

// CmdNew creates a new note
func (b Module) CmdNew() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "note",
		Short: "creates new note",
		Long:  "creates new note",
		Run: func(cmd *cobra.Command, args []string) {
			// Check if the `--interactive` flag is passed
			if interactive {
				label, title = "", ""
			}
			// TODO: Not quite sure this is the right place to put this
			// TODO: Document this "feature"
			label = strings.TrimPrefix(label, "notes/")
			if title == "" {
				result, err := util.AskInput("Title")
				if err != nil {
					log.Fatalln(err)
				}
				title = result
			}
			if interactive {
				result, err := util.AskInput("Label (subfolder)")
				if err != nil {
					log.Fatalln(err)
				}
				label = result
			}
			// Validate label
			if _, err := validateLabel(label); err != nil {
				log.Fatal(err)
			}
			if err := create(title, label); err != nil {
				log.Fatalln(err)
			}
			// TODO: there should be a flag if you do not want to open it
			// Open note
			note, err := get(title, label)
			if err != nil {
				log.Fatalln(err)
			}
			if err := open(note); err != nil {
				log.Fatalln(err)
			}
		},
	}
	cmd.Flags().BoolVarP(&interactive, "interactive", "i", false, "run interactively")
	cmd.Flags().StringVarP(&title, "title", "t", "", "note title")
	cmd.Flags().StringVarP(&label, "label", "l", "", "label (subfolder)")
	return cmd
}
