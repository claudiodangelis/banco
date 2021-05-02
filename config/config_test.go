package config

import (
	"fmt"
	"io/ioutil"
	"os"
	"reflect"
	"testing"
)

func TestConfig_Get(t *testing.T) {
	// Write example file
	err := ioutil.WriteFile(".banco.yaml", []byte(
		`
title: hello 
notes:
  title: hi
tasks:
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
		s string
	}
	tests := []struct {
		name string
		c    Config
		args args
		want interface{}
	}{
		{
			"first",
			New(),
			args{
				s: "title",
			},
			"hello",
		},
		{
			"second",
			New(),
			args{
				s: "notes.title",
			},
			"hi",
		},
		{
			"third",
			New(),
			args{
				s: "i.do.not.exist",
			},
			nil,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			c := Config{}
			if got := c.Get(tt.args.s); !reflect.DeepEqual(got, tt.want) {
				fmt.Printf("%T%T\n", got, tt.want)
				t.Errorf("Config.Get() = %v, want %v", got, tt.want)
			}
		})
	}
}
