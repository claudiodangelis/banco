package notes

import (
	"fmt"
	"strings"

	"github.com/manifoldco/promptui"
)

// Shows a select-with-add prompt to choose the label or add a new one
func labelPicker() (string, error) {
	l, err := labels()
	if err != nil {
		return "", err
	}
	prompt := promptui.SelectWithAdd{
		AddLabel: "Create label",
		Items:    l,
		Label:    "Choose label",
	}
	_, result, err := prompt.Run()
	return result, err
}

// Shows a select prompt to choose the note
func notePicker() (Note, error) {
	items := []string{}
	notes, err := list()
	if err != nil {
		return Note{}, err
	}
	mapped := make(map[string]Note)
	for _, note := range notes {
		p := fmt.Sprintf("[%s] %s", note.Label, note.Title)
		mapped[p] = note
		items = append(items, p)
	}
	prompt := promptui.Select{
		Label: "Choose note:",
		Items: items,
		// TODO: This size should fit window size
		Size: 100,
		// TODO: Move this searcher out of here
		Searcher: func(input string, index int) bool {
			return strings.Contains(items[index], input)
		},
		StartInSearchMode: true,
	}
	_, result, err := prompt.Run()
	return mapped[result], err
}
