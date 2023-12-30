package testutil

import (
	"os"
	"path/filepath"
	"runtime"
)

func SetFakeHome() {
	_, thisfile, _, _ := runtime.Caller(0)
	root := filepath.Dir(filepath.Dir(thisfile))
	if err := os.Chdir(filepath.Join(root, "testdata")); err != nil {
		panic(err)
	}
}
