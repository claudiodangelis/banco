package tasks

import (
	"io/ioutil"
	"os"
	"path/filepath"
	"testing"

	"github.com/claudiodangelis/banco/util"
)

func TestModule_Init(t *testing.T) {
	tmpdir, err := ioutil.TempDir("", "bancotasks")
	if err != nil {
		panic(err)
	}
	os.Chdir(tmpdir)
	tests := []struct {
		name    string
		b       Module
		wantErr bool
	}{
		{"create folders", New(), false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			b := tt.b
			if err := b.Init(); (err != nil) != tt.wantErr {
				t.Errorf("Module.Init() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
		// Check if folders have been created
		for _, status := range tt.b.statuses {
			if _, err := util.IsEmptyDir(filepath.Join(tmpdir, "tasks", status)); err != nil {
				t.Error(err)
			}
		}
	}
	if err := os.RemoveAll(tmpdir); err != nil {
		panic(err)
	}
}

func Test_create(t *testing.T) {
	type args struct {
		task Task
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if err := create(tt.args.task); (err != nil) != tt.wantErr {
				t.Errorf("create() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}
