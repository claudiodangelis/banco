package notes

import (
	"fmt"
	"log"

	"github.com/claudiodangelis/banco/util"
	"github.com/manifoldco/promptui"
	"github.com/spf13/cobra"
)

// Entrypoint of the module
func (b Module) Entrypoint() error {
	// TODO: Explore if this can be abstracted
	p := promptui.Select{
		Items: []string{
			"Create a note",
			"Open a note",
			"Update a note",
		},
		Label: "What you want to do?",
	}
	_, result, err := p.Run()
	if err != nil {
		log.Fatalln(err)
	}
	if result == "Create a note" {
		b.CmdNew().Execute()
	} else if result == "Open a note" {
		b.CmdOpen().Execute()
	} else if result == "Update a note" {
		b.CmdUpdate().Execute()
	}
	return nil
}

// CmdSummary returns a single line summary of the module's items
func (b Module) CmdSummary() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "summary",
		Short: "print a quick summary",
		Run: func(cmd *cobra.Command, args []string) {
			s, err := summary()
			if err != nil {
				log.Fatalln(err)
			}
			fmt.Println(s)
		},
	}
	return cmd
}

// CmdRoot sets the root for this command (interactive searching note)
func (b Module) CmdRoot() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "notes",
		Short: "manage notes",
		Long:  "manage notes",
		Run: func(cmd *cobra.Command, args []string) {
			util.ClearScreen()
			if err := b.Entrypoint(); err != nil {
				panic(err)
			}
		},
	}
	return cmd
}

// CmdUpdate updates a note
func (b Module) CmdUpdate() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "note",
		Short: "updates a note",
		Long:  "updates a note",
		Run: func(cmd *cobra.Command, args []string) {
			// Prompt which note you want to update
			note, err := pick()
			if err != nil {
				log.Fatalln(err)
			}
			// Prompt what kind of update you want to make
			prompt := promptui.Select{
				Label: "What you want to do?",
				Items: []string{"rename"},
			}
			_, result, err := prompt.Run()
			if err != nil {
				log.Fatalln(err)
			}
			if result == "rename" {
				// Prompt new title
				pTitle := promptui.Prompt{
					Label:     "Title",
					Default:   note.Title,
					AllowEdit: true,
				}
				title, err := pTitle.Run()
				if err != nil {
					log.Fatalln(err)
				}
				// Prompt new label
				pLabel := promptui.Prompt{
					Label:     "Label",
					Default:   note.Label,
					AllowEdit: true,
				}
				label, err := pLabel.Run()
				if err != nil {
					log.Fatalln(err)
				}
				newNote := Note{
					Title: title,
					Label: label,
				}
				if err := rename(note, newNote); err != nil {
					log.Fatalln(err)
				}
			}
		},
	}
	return cmd
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
				log.Fatalln(err)
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
			note, err := pick()
			if err != nil {
				log.Fatalln(err)
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
	cmd := &cobra.Command{
		Use:   "note",
		Short: "open a note",
		Long:  "open a note",
		Run: func(cmd *cobra.Command, args []string) {
			note, err := pick()
			if err != nil {
				log.Fatalln(err)
			}
			if err := open(note); err != nil {
				log.Fatalln(err)
			}
		},
	}
	return cmd
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
			title, err := util.AskInput("Title")
			if err != nil {
				log.Fatalln(err)
			}
			label, err := labelPicker()
			if err != nil {
				log.Fatalln(err)
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
