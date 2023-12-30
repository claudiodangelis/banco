package module

import (
	"os"
	"reflect"
	"testing"

	"github.com/claudiodangelis/banco/config"
	"github.com/claudiodangelis/banco/item"
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
				Providers: map[string]provider.Provider{
					"local": localtasks.New("tasks/local", config.ProviderConfig{
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

func TestModule_ListItems(t *testing.T) {
	type fields struct {
		Name      ModuleName
		Providers map[string]provider.Provider
	}
	tests := []struct {
		name    string
		fields  fields
		want    []item.Item
		wantErr bool
	}{
		{
			"#1",
			fields{
				Name: ModuleTasks,
				Providers: map[string]provider.Provider{
					"local": localtasks.New("tasks/local", config.ProviderConfig{
						Provider: "local",
						Disabled: false,
					}),
					"customlocal": localtasks.New("tasks/customlocal", config.ProviderConfig{
						Provider: "local",
						Disabled: false,
					}),
				},
			},
			[]item.Item{
				{
					Parameters: map[string]string{
						"Title":  "0001 - TEST",
						"Status": "backlog",
						"IsDir":  "No",
					},
					Resource: "tasks/local/backlog/0001 - TEST",
				},
				{
					Parameters: map[string]string{
						"Title":  "0002 - WIP",
						"Status": "doing",
						"IsDir":  "Yes",
					},
					Resource: "tasks/local/doing/0002 - WIP",
				},
				{
					Parameters: map[string]string{
						"Title":  "Make it work",
						"Status": "done",
						"IsDir":  "No",
					},
					Resource: "tasks/customlocal/done/Make it work",
				},
			},
			false,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			m := Module{
				Name:      tt.fields.Name,
				Providers: tt.fields.Providers,
			}
			got, err := m.ListItems()
			if (err != nil) != tt.wantErr {
				t.Errorf("Module.ListItems() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("Module.ListItems() = %v, want %v", got, tt.want)
			}
		})
	}
}
