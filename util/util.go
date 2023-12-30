package util

import (
	"os"

	"github.com/claudiodangelis/banco/module"
)

// IsEmptyDir returns (true, nil) if dir is empty, (false, error) otherwise
// Credits: https://rosettacode.org/wiki/Empty_directory#Go
func IsEmptyDir(name string) (bool, error) {
	entries, err := os.ReadDir(name)
	if err != nil {
		return false, err
	}
	return len(entries) == 0, nil
}

// IsBanco returns (true, nil) if the cwd is a banco directory
func IsBanco() (bool, error) {
	// TODO: this does not guarantee that the folder is an actual banco folder
	for _, m := range module.All() {
		if _, err := os.Stat(string(m.Name)); os.IsNotExist(err) {
			return false, err
		}
	}
	return true, nil
}
