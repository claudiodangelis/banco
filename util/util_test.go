package util

import (
	"io/ioutil"
	"os"
	"testing"

	"github.com/claudiodangelis/banco/item"
)

func TestGetDefaultTitle(t *testing.T) {
	// Write example file
	err := ioutil.WriteFile(".banco.yaml", []byte(
		`
	title: hello 
	notes:
	  title: hi
	tasks:
	  title: $id
	  columns:
		- backlog
		- doing
		- done
	`), os.ModePerm)
	if err != nil {
		t.Skip()
	}
	defer func() {
		os.Remove(".banco.yaml")
	}()
	type args struct {
		module string
		items  []item.Item
	}
	tests := []struct {
		name string
		args args
		want string
	}{
		{
			"first",
			args{
				module: "tasks",
				items: []item.Item{
					item.Item{},
					item.Item{},
				},
			},
			"3",
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := GetDefaultTitle(tt.args.module, tt.args.items); got != tt.want {
				t.Errorf("GetDefaultTitle() = %v, want %v", got, tt.want)
			}
		})
	}
}
