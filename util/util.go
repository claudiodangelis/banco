package util

import "io/ioutil"

// IsEmptyDir returns (true, nil) if dir is empty, (false, error) otherwise
// Credits: https://rosettacode.org/wiki/Empty_directory#Go
func IsEmptyDir(name string) (bool, error) {
	entries, err := ioutil.ReadDir(name)
	if err != nil {
		return false, err
	}
	return len(entries) == 0, nil
}
