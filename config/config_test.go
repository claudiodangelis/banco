package config

import (
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"testing"

	"github.com/claudiodangelis/banco/item"
)

func setFakeHome() string {
	wd, err := os.Getwd()
	if err != nil {
		panic(err)
	}
	fakehome := filepath.Join(filepath.Dir(wd), "testdata", "config", "fakehome")
	os.Setenv("HOME", fakehome)
	return fakehome
}

func Test_initConfigFile(t *testing.T) {
	if runtime.GOOS == "windows" {
		fmt.Println("this test is only available on unix")
		t.Skip()
	}
	tmpdir, err := os.MkdirTemp("", "banco-*")
	if err != nil {
		panic(err)
	}

	tests := []struct {
		name string
		home string
	}{
		{
			name: "temp home",
			home: tmpdir,
		},
	}
	for _, tt := range tests {
		// NOTE: this will probably only work Linux
		os.Setenv("HOME", tmpdir)
		t.Run(tt.name, func(t *testing.T) {
			initDefaultConfigFile()
		})

	}
}

func TestNew(t *testing.T) {
	fakehome := setFakeHome()
	initDefaultConfigFile()
	tests := []struct {
		name       string
		projectDir string
		want       Config
	}{
		{
			"global config",
			"myproject",
			Config{
				Path: filepath.Join(fakehome, ".config", "banco", "config.yml"),
			},
		},
		{
			"custom config",
			"myprojectcustomconfig",
			Config{
				Path: filepath.Join(fakehome, "myprojectcustomconfig", ".banco", "config.yml"),
			},
		},
	}
	for _, tt := range tests {
		wd, err := os.Getwd()
		if err != nil {
			panic(err)
		}
		if err := os.Chdir(filepath.Join(fakehome, tt.projectDir)); err != nil {
			panic(err)
		}
		t.Run(tt.name, func(t *testing.T) {
			if got := New(); !reflect.DeepEqual(got, tt.want) {
				t.Errorf("New() = %v, want %v", got, tt.want)
			}
		})
		if err := os.Chdir(wd); err != nil {
			panic(err)
		}
	}
}

func TestConfig_GetDefaultTitle(t *testing.T) {
	fakehome := setFakeHome()
	type args struct {
		module string
	}
	tests := []struct {
		name        string
		itemsLength int
		projectDir  string
		args        args
		want        string
	}{
		{
			"global title",
			0,
			"myproject",
			args{
				module: "notes",
			},
			"hello",
		},
		{
			"custom title",
			0,
			"myprojectcustomconfig",
			args{
				module: "notes",
			},
			"you are awesome",
		},
		{
			"title variable",
			0,
			"myprojectcustomconfig",
			args{
				module: "tasks",
			},
			"0001",
		},
		{
			"more title variable",
			10,
			"myprojectcustomconfig",
			args{
				module: "tasks",
			},
			"0011",
		},
	}
	for _, tt := range tests {
		wd, err := os.Getwd()
		if err != nil {
			panic(err)
		}
		if err := os.Chdir(filepath.Join(fakehome, tt.projectDir)); err != nil {
			panic(err)
		}
		t.Run(tt.name, func(t *testing.T) {
			c := New()
			if got := c.GetDefaultTitle(tt.args.module, make([]item.Item, tt.itemsLength)); got != tt.want {
				t.Errorf("Config.GetDefaultTitle() = %v, want %v", got, tt.want)
			}
		})
		if err := os.Chdir(wd); err != nil {
			panic(err)
		}
	}
}

func TestConfig_GetTemplatePath(t *testing.T) {
	fakehome := setFakeHome()
	type args struct {
		module string
		label  string
	}
	tests := []struct {
		name       string
		args       args
		projectDir string
		want       string
		want1      bool
	}{
		{
			"no template existing",
			args{
				module: "tasks",
				label:  "",
			},
			"myproject",
			"",
			false,
		},
		{
			"template existing for global project",
			args{
				module: "notes",
				label:  "",
			},
			"myproject",
			filepath.Join(fakehome, ".config", "banco", "templates", "notes", "template"),
			true,
		},
		{
			"template existing for custom project",
			args{
				module: "notes",
				label:  "meetings",
			},
			"myprojectcustomconfig",
			filepath.Join(fakehome, "myprojectcustomconfig", ".banco", "templates", "notes", "meetings", "template"),
			true,
		},
	}
	for _, tt := range tests {
		wd, err := os.Getwd()
		if err != nil {
			panic(err)
		}
		if err := os.Chdir(filepath.Join(fakehome, tt.projectDir)); err != nil {
			panic(err)
		}
		t.Run(tt.name, func(t *testing.T) {
			c := New()
			got, got1 := c.GetTemplatePath(tt.args.module, tt.args.label)
			if got != tt.want {
				t.Errorf("Config.GetTemplatePath() got = %v, want %v", got, tt.want)
			}
			if got1 != tt.want1 {
				t.Errorf("Config.GetTemplatePath() got1 = %v, want %v", got1, tt.want1)
			}
		})
		if err := os.Chdir(wd); err != nil {
			panic(err)
		}
	}
}

func TestGetNew(t *testing.T) {
	setFakeHome()
	New()
	tests := []struct {
		name string
		want NewConfig
	}{
		{
			"#1",
			NewConfig{
				Notes: struct{ Title string }{"hello"},
				Tasks: struct {
					Title     string
					Providers []TasksProvider
				}{
					Providers: []TasksProvider{
						{
							Provider: "github",
							Name:     "myrepo",
							Parameters: map[string]string{
								"apikey": "abc",
							},
						},
					},
				},
			},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := GetNew(); !reflect.DeepEqual(got, tt.want) {
				t.Errorf("GetNew() = %v, want %v", got, tt.want)
			}
		})
	}
}
