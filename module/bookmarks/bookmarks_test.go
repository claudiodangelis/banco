package bookmarks

import (
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func setFakeHome() string {
	wd, err := os.Getwd()
	if err != nil {
		panic(err)
	}
	fakehome := filepath.Join(filepath.Dir(filepath.Dir(wd)), "testdata", "config", "fakehome")
	os.Setenv("HOME", fakehome)
	return fakehome
}

func Test_getBrowserConfiguration(t *testing.T) {
	fakehome := setFakeHome()
	var NILSLICE []string
	tests := []struct {
		name               string
		browserEnvVariable string
		projectDir         string
		wantCmd            string
		wantArgs           []string
	}{
		{
			"$BROWSER variable set, configuration file not overridden",
			"chromium",
			"myproject",
			"chromium",
			NILSLICE,
		},
		{
			"$BROWSER variable set, configuration file overridden",
			"chromium",
			"myprojectcustomconfig",
			"firefox",
			[]string{"-p", "work"},
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
		os.Setenv("BROWSER", tt.browserEnvVariable)
		t.Run(tt.name, func(t *testing.T) {
			gotCmd, gotArgs := getBrowserConfiguration()
			if gotCmd != tt.wantCmd {
				t.Errorf("getBrowserConfiguration() gotCmd = %v, want %v", gotCmd, tt.wantCmd)
			}
			if !reflect.DeepEqual(gotArgs, tt.wantArgs) {
				t.Errorf("getBrowserConfiguration() gotArgs = %v, want %v", gotArgs, tt.wantArgs)
			}
		})
		os.Unsetenv("BROWSER")
		if err := os.Chdir(wd); err != nil {
			panic(err)
		}
	}
}
