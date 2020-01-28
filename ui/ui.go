package ui

import (
	"strings"

	"github.com/buger/goterm"
	"github.com/manifoldco/promptui"
)

// ErrInterrupt is thrown when user presses ctrl+c
var ErrInterrupt = promptui.ErrInterrupt

// initialPosition is an utility function that returns the position of the default
// value in the list
func initialPosition(options []string, value string) int {
	if value == "" {
		return 0
	}
	for position, item := range options {
		if value == item {
			return position
		}
	}
	return 0
}

// ClearScreen clears the screen
func ClearScreen() {
	// TODO: This will go away when we have a proper gui
	goterm.Clear()
	goterm.MoveCursor(1, 1)
	goterm.Flush()
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

// Select one of the items
func Select(label string, options []string, value string, search bool) (string, error) {
	if value != "" {
		// NOTE: Addressing a possible manifoldco/promptui bug:
		// https://github.com/manifoldco/promptui/issues/105
		search = false
	}
	prompt := promptui.Select{
		HideHelp:          true,
		Label:             label,
		Items:             options,
		StartInSearchMode: search,
		Searcher: func(input string, index int) bool {
			return strings.Contains(strings.ToLower(options[index]), strings.ToLower(input))
		},
		Size: goterm.Height() - 3,
	}
	_, result, err := prompt.RunCursorAt(initialPosition(options, value), 0)
	return result, err
}

// SelectWithAdd prompts a list and adds the ability of create a new entry
func SelectWithAdd(label, value string, options []string, search bool) (string, error) {
	if value != "" {
		// NOTE: Addressing a possible manifoldco/promptui bug:
		// https://github.com/manifoldco/promptui/issues/105
		search = false
	}
	options = append([]string{"+ Create new"}, options...)
	typed := ""
	prompt := promptui.Select{
		HideHelp:          true,
		Items:             options,
		Label:             label,
		StartInSearchMode: false,
		Searcher: func(input string, index int) bool {
			if index == 0 {
				return true
			}
			// Store the query so you can reuse it for the "create new" choice
			typed = input
			return strings.Contains(strings.ToLower(options[index]),
				strings.ToLower(input))
		},
		Size: goterm.Height() - 3,
	}
	i, result, err := prompt.RunCursorAt(initialPosition(options, value), 0)
	if err != nil {
		return result, err
	}
	if i == 0 {
		// New item
		result, err = Input(label, typed)
	}
	return result, err
}
