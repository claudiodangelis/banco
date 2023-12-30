package local

import (
	"os"
	"reflect"
	"testing"

	"github.com/claudiodangelis/banco/item"
	"github.com/claudiodangelis/banco/testutil"
)

func TestMain(m *testing.M) {
	testutil.SetFakeHome()
	code := m.Run()
	os.Exit(code)
}

func TestTaskProvider_List(t *testing.T) {
	tests := []struct {
		name string
		l    LocalTaskProvider
		want []item.Item
	}{
		{
			"#1",
			LocalTaskProvider{
				Entrypoint: "tasks/local",
			},
			[]item.Item{
				{
					Parameters: map[string]string{
						"Title":  "0001 - TEST",
						"Status": "backlog",
						"IsDir":  "No",
					},
				},
				{
					Parameters: map[string]string{
						"Title":  "0002 - WIP",
						"Status": "doing",
						"IsDir":  "Yes",
					},
				},
			},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got, _ := tt.l.List(); !reflect.DeepEqual(got, tt.want) {
				t.Errorf("TaskProvider.List() = %v, want %v", got, tt.want)
			}
		})
	}
}
