package cmd

import "github.com/claudiodangelis/banco/module"

// isBanco checks if the current directory is a banco directory
func isBanco() (bool, error) {
    for _, m := range module.All() {
        if err := m.Check(); err != nil {
            return false, err
        }
    }
    return true, nil
}
