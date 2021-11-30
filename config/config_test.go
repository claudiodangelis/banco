package config

import (
	"fmt"
	"io/ioutil"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

// func TestNewLocal(t *testing.T) {
// 	localdir := filepath.Join(os.TempDir(), "bancotest/myproject")
// 	configpath := filepath.Join(localdir, ".banco", "config.yml")
// 	if err := os.MkdirAll(filepath.Dir(configpath), os.ModePerm); err != nil {
// 		panic(err)
// 	}
// 	if _, err := os.OpenFile(configpath, os.O_RDONLY|os.O_CREATE, 0666); err != nil {
// 		panic(err)
// 	}
// 	if err := os.Chdir(localdir); err != nil {
// 		panic(err)
// 	}
// 	tests := []struct {
// 		name string
// 		want Config
// 	}{
// 		{
// 			name: "local project",
// 			want: Config{
// 				Path: configpath,
// 			},
// 		},
// 	}
// 	for _, tt := range tests {
// 		t.Run(tt.name, func(t *testing.T) {
// 			if got := New(); !reflect.DeepEqual(got, tt.want) {
// 				t.Errorf("New() = %v, want %v", got, tt.want)
// 			}
// 		})
// 	}
// }

// func TestNewGlobal(t *testing.T) {
// 	tmpdir := os.TempDir()
// 	tmphome := filepath.Join(tmpdir, "bancotest")
// 	randomdir := filepath.Join(tmpdir, "randomdir")
// 	if err := os.MkdirAll(tmphome, os.ModePerm); err != nil {
// 		panic(err)
// 	}
// 	if err := os.MkdirAll(randomdir, os.ModePerm); err != nil {
// 		panic(err)
// 	}

// 	if err := os.Setenv("HOME", tmphome); err != nil {
// 		panic(err)
// 	}
// 	os.Chdir(randomdir)
// 	tests := []struct {
// 		name string
// 		want Config
// 	}{
// 		{
// 			name: "global config file",
// 			want: Config{
// 				Path: filepath.Join(tmphome, ".config/banco/config.yml"),
// 			},
// 		},
// 	}
// 	for _, tt := range tests {
// 		t.Run(tt.name, func(t *testing.T) {
// 			if got := New(); !reflect.DeepEqual(got, tt.want) {
// 				t.Errorf("New() = %v, want %v", got, tt.want)
// 			}
// 		})
// 	}
// }

func TestConfig_Get(t *testing.T) {
	// Set a local directory for the tests
	localdir := filepath.Join(os.TempDir(), "bancotest/myproject2")
	configpath := filepath.Join(localdir, ".banco", "config.yml")
	if err := os.MkdirAll(filepath.Dir(configpath), os.ModePerm); err != nil {
		panic(err)
	}
	if err := ioutil.WriteFile(configpath, []byte(
		`
notes:
  title: hi
tasks:
  columns:
    - backlog
    - doing
    - done
 `), 0666); err != nil {
		panic(err)
	}
	if err := os.Chdir(localdir); err != nil {
		panic(err)
	}
	type args struct {
		s string
	}
	cfg := New()
	tests := []struct {
		name string
		c    Config
		args args
		want interface{}
	}{
		{
			"second",
			cfg,
			args{
				s: "notes.title",
			},
			"hi",
		},
		// {
		// 	"third",
		// 	New(),
		// 	args{
		// 		s: "i.do.not.exist",
		// 	},
		// 	nil,
		// },
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {

			if got := cfg.Get(tt.args.s); !reflect.DeepEqual(got, tt.want) {
				fmt.Printf("%T%T\n", got, tt.want)
				t.Errorf("Config.Get() = %v, want %v", got, tt.want)
			}
		})
	}

}

// func TestConfig_Get(t *testing.T) {
// 	t.Skip()
// 	// Write example file
// 	err := ioutil.WriteFile(".banco.yaml", []byte(
// 		`
// title: hello
// notes:
//   title: hi
// tasks:
//   columns:
//     - backlog
//     - doing
//     - done
// `), os.ModePerm)
// 	if err != nil {
// 		t.Skip()
// 	}
// 	defer func() {
// 		os.Remove(".banco.yaml")
// 	}()

// }
