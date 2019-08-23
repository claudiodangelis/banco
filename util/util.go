package util

import (
	"io/ioutil"

	"github.com/manifoldco/promptui"
)

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
