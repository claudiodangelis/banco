package config

import (
	"fmt"
	"io/ioutil"
	"os"
	"reflect"
	"testing"

	"github.com/claudiodangelis/banco/item"
)

func TestConfig_Get(t *testing.T) {
	t.Skip()
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

func TestConfig_GetDefaultTitle(t *testing.T) {
	// Write example file
	err := ioutil.WriteFile(".banco.yaml", []byte(
		`
title: hello 
notes:
    title: $timestamp
tasks:
    title: $id
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
			"0003",
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			c := New()
			if got := c.GetDefaultTitle(tt.args.module, tt.args.items); got != tt.want {
				t.Errorf("Config.GetDefaultTitle() = %v, want %v", got, tt.want)
			}
		})
	}
}
