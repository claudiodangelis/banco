package ui

import (
	"strings"

	"github.com/buger/goterm"

	"github.com/manifoldco/promptui"
)

// ClearScreen clears the screen
func ClearScreen() {
	// TODO: This will go away when we have a proper gui
	// goterm.Clear()
	// goterm.MoveCursor(1, 1)
	// goterm.Flush()
}

// Select one of the items
func Select(label string, items []string, search bool) (string, error) {
	prompt := promptui.Select{
		HideHelp:          true,
		Label:             label,
		Items:             items,
		StartInSearchMode: search,
		Searcher: func(input string, index int) bool {
			return strings.Contains(items[index], input)
		},
		Size: goterm.Height() - 3,
	}
	_, result, err := prompt.Run()
	return result, err
}

// Input string
func Input(label, value string) (string, error) {
	prompt := promptui.Prompt{
		Label:     label,
		AllowEdit: true,
		Default:   value,
	}
	result, err := prompt.Run()
	return result, err
}

// SelectWithAdd prompts a list and adds the ability of create a new entry
func SelectWithAdd(label, value string, options []string) (string, error) {
	options = append([]string{"+ Create new"}, options...)
	typed := ""
	prompt := promptui.Select{
		HideHelp:          true,
		Items:             options,
		Label:             label,
		StartInSearchMode: true,
		Searcher: func(input string, index int) bool {
			if index == 0 {
				return true
			}
			// Store the query so you can reuse it for the "create new" choice
			typed = input
			return strings.Contains(options[index], input)
		},
		Size: goterm.Height() - 3,
	}
	i, result, err := prompt.Run()
	if err != nil {
		return result, err
	}
	if i == 0 {
		// New item
		result, err = Input(label, typed)
	}
	return result, err
}
