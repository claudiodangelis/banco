package documents

import (
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"testing"
	"time"
)

func Test_list(t *testing.T) {
	_, caller, _, _ := runtime.Caller(0)
	dir := filepath.Dir(caller)
	os.Chdir(filepath.Join(dir, "test_data"))
	tests := []struct {
		name    string
		want    []Document
		wantErr bool
	}{
		{
			"first", []Document{
				Document{
					Name:      "Document.pdf",
					Directory: "",
					MimeType:  "",
					CreatedAt: time.Time{},
					UpdatedAt: time.Unix(1569172713, 951109430),
					Size:      8733,
				},
				Document{
					Name:      "Spreadsheet.ods",
					Directory: "",
					MimeType:  "",
					CreatedAt: time.Time{},
					UpdatedAt: time.Unix(1569172713, 951109430),
					Size:      7676,
				},
				Document{
					Name:      "Word document.odt",
					Directory: "",
					MimeType:  "",
					CreatedAt: time.Time{},
					UpdatedAt: time.Unix(1569172713, 951109430),
					Size:      8209,
				},
				Document{
					Name:      "file.txt",
					Directory: "sub",
					MimeType:  "",
					CreatedAt: time.Time{},
					UpdatedAt: time.Unix(1569173551, 594712544),
					Size:      22,
				},
			}, false,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := list()
			if (err != nil) != tt.wantErr {
				t.Errorf("list() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("list() = %v, want %v", got, tt.want)
			}
		})
	}
}
