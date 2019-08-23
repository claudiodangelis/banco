package notes

import (
	"fmt"
	"log"
	"strings"

	"github.com/claudiodangelis/banco/util"
	"github.com/spf13/cobra"
)

// CmdRoot sets the root for this command (interactive searching note)
func (b Module) CmdRoot() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "notes",
		Short: "manage notes",
		Long:  "manage notes",
		Run: func(cmd *cobra.Command, args []string) {
			note, err := notePicker()
			if err != nil {
				panic(err)
			}
			open(note)
		},
	}
	return cmd
}

// CmdUpdate updates a note
func (b Module) CmdUpdate() *cobra.Command {
	return nil
}

// CmdList lists notes
func (b Module) CmdList() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "notes",
		Short: "list notes",
		Long:  "list notes",
		Run: func(cmd *cobra.Command, args []string) {
			notes, err := list()
			if err != nil {
				panic(err)
			}
			mapped := make(map[string][]Note)
			for _, note := range notes {
				label := note.Label
				if label == "" {
					label = "."
				}
				if _, ok := mapped[label]; !ok {
					mapped[label] = []Note{}
				}
				mapped[label] = append(mapped[label], note)
			}
			// Show root notes first
			rootnotes, ok := mapped["."]
			if ok {
				for _, note := range rootnotes {
					fmt.Println(note.Title)
				}
			}
			// Show other notes
			for label, notes := range mapped {
				if label == "." {
					continue
				}
				fmt.Println("[" + label + "]")
				for _, note := range notes {
					fmt.Println(" ", note.Title)
				}
			}
		},
	}
	return cmd
}

// CmdDelete deletes a note
func (b Module) CmdDelete() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "note",
		Short: "deletes a note",
		Long:  "Deletes a note. If the note has a label and it's the only note with that label, label is deleted",
		Run: func(cmd *cobra.Command, args []string) {
			note, err := notePicker()
			if err != nil {
				panic(err)
			}
			if err := delete(note); err != nil {
				log.Fatalln(err)
			}
		},
	}
	return cmd
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
			// TODO: It should always be interactive
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
