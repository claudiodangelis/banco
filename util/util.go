package util

import (
	"io/ioutil"
	"github.com/buger/goterm"
	"github.com/manifoldco/promptui"
)
// IsBanco checks if the current directory is a banco directory
//func IsBanco() (bool, error) {
//    for _, m := range module.All() {
//        if _, err := m.Check(); err != nil {
//            return false, err
//        }
//    }
//    return true, nil
//}
//
// ClearScreen clears the screen
func ClearScreen() {
	// TODO: This will go away when we have a proper gui
	goterm.Clear()
	goterm.MoveCursor(1, 1)
	goterm.Flush()
}

// IsEmptyDir returns (true, nil) if dir is empty, (false, error) otherwise
// Credits: https://rosettacode.org/wiki/Empty_directory#Go
func IsEmptyDir(name string) (bool, error) {
	entries, err := ioutil.ReadDir(name)
	if err != nil {
		return false, err
	}
	return len(entries) == 0, nil
}

var templates = &promptui.PromptTemplates{
	Prompt:  "{{ . }} ",
	Valid:   "{{ . }} ",
	Invalid: "{{ . }} ",
	Success: "{{ . }} ",
}

// AskInput asks the user to input a string
func AskInput(label string) (string, error) {
	prompt := promptui.Prompt{
		Label:     label + ":",
		AllowEdit: true,
		IsVimMode: true,
		Templates: templates,
	}
	result, err := prompt.Run()
	if err != nil {
		return "", err
	}
	return result, nil
}
