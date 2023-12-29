package module

import (
	"fmt"
	"os"
	"reflect"
	"testing"

	"github.com/claudiodangelis/banco/config"
	"github.com/claudiodangelis/banco/provider"
	localtasks "github.com/claudiodangelis/banco/provider/tasks/local"
	"github.com/claudiodangelis/banco/testutil"
)

func TestMain(m *testing.M) {
	testutil.SetFakeHome()
	code := m.Run()
	os.Exit(code)
}

func TestNew(t *testing.T) {
	fmt.Println(os.Getwd())
	type args struct {
		name ModuleName
	}
	tests := []struct {
		name string
		args args
		want Module
	}{
		{
			"#1",
			args{
				name: "tasks",
			},
			Module{
				Name: ModuleTasks,
				Providers: []provider.Provider{
					localtasks.New("tasks/local", config.ProviderConfig{
						Provider: "local",
						Disabled: true,
					}),
				},
			},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := New(tt.args.name); !reflect.DeepEqual(got, tt.want) {
				t.Errorf("New() = %v, want %v", got, tt.want)
			}
		})
	}
}
