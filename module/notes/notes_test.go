package notes

import (
	"io/ioutil"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"testing"
	"time"

	"github.com/otiai10/copy"
)

func TestNew(t *testing.T) {
	t.Skip()
	tests := []struct {
		name string
		want Module
	}{
		{"new", Module{}},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Skip()
			if got := New(); !reflect.DeepEqual(got, tt.want) {
				t.Errorf("New() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestModule_Init(t *testing.T) {
	t.Skip()
	tmpdir, err := ioutil.TempDir("", "banconotes")
	if err != nil {
		panic(err)
	}
	os.Chdir(tmpdir)
	tests := []struct {
		name    string
		b       Module
		wantErr bool
	}{
		{"init", Module{}, false},
		{"init again, should fail", Module{}, true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Skip()
			b := Module{}
			if err := b.Init(); (err != nil) != tt.wantErr {
				t.Errorf("Module.Init() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
	if err := os.RemoveAll(tmpdir); err != nil {
		panic(err)
	}
}

func Test_create(t *testing.T) {
	t.Skip()
	tmpdir, err := ioutil.TempDir("", "banconotes-create")
	if err != nil {
		panic(err)
	}
	os.Chdir(tmpdir)
	if err := os.Mkdir("notes", os.ModePerm); err != nil {
		panic(err)
	}
	type args struct {
		title string
		label string
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{"no label", args{title: "hello", label: ""}, false},
		{"existing", args{title: "hello", label: ""}, true},
		{"with label", args{title: "hello", label: "subfolder"}, false},
		{"invalid", args{title: "il/legal", label: ""}, true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Skip()
			if err := create(tt.args.title, tt.args.label); (err != nil) != tt.wantErr {
				t.Errorf("create() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
	if err := os.RemoveAll(tmpdir); err != nil {
		panic(err)
	}
}

func Test_get(t *testing.T) {
	t.Skip()
	_, caller, _, _ := runtime.Caller(0)
	dir := filepath.Dir(caller)
	type args struct {
		title string
		label string
	}
	tests := []struct {
		testdir string
		name    string
		args    args
		want    Note
		wantErr bool
	}{
		{"test_data/test01", "it exists", args{title: "hello", label: ""}, Note{
			Label: "",
			Title: "hello",
			Size:  4,
		}, false},
		{"test_data/test02", "it exists in a subfolder", args{title: "sub.txt", label: "subfolder"}, Note{
			Label: "subfolder",
			Title: "sub.txt",
			Size:  3,
		}, false},
		{"test_data/test03", "it exists in a deep subfolder", args{title: "hiding", label: "sub/fol/der"}, Note{
			Label: "sub/fol/der",
			Title: "hiding",
			Size:  0,
		}, false},
		{"test_data/test03", "not existing", args{title: "missing", label: ""}, Note{
			Label: "sub/fol/der",
			Title: "hiding",
			Size:  0,
		}, true},
	}
	for _, tt := range tests {
		if err := os.Chdir(filepath.Join(dir, tt.testdir)); err != nil {
			panic(err)
		}
		t.Run(tt.name, func(t *testing.T) {
			t.Skip()
			got, err := get(tt.args.title, tt.args.label)
			if (err != nil) != tt.wantErr {
				t.Errorf("get() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			// Do not continue if you want an error
			if tt.wantErr {
				return
			}
			if tt.want.Label != got.Label {
				t.Errorf("get() = %v, want %v", got.Label, tt.want.Label)
			}
			if tt.want.Title != got.Title {
				t.Errorf("get() = %v, want %v", got.Title, tt.want.Title)
			}
			if tt.want.Size != got.Size {
				t.Errorf("get() = %v, want %v", got.Size, tt.want.Size)
			}
		})
	}
}

func Test_list(t *testing.T) {
	t.Skip()
	_, caller, _, _ := runtime.Caller(0)
	dir := filepath.Dir(caller)
	os.Chdir(filepath.Join(dir, "test_data/test04"))
	tests := []struct {
		name    string
		want    []Note
		wantErr bool
	}{
		{"first", []Note{
			Note{
				Title:     "hello",
				Label:     "",
				Size:      3,
				UpdatedAt: time.Unix(0, 0),
			},
			Note{
				Title:     "20190101-discussing-about-tools",
				Label:     "meetings",
				Size:      25,
				UpdatedAt: time.Unix(0, 0),
			},
			Note{
				Title:     "20190102-expenses",
				Label:     "meetings",
				Size:      19,
				UpdatedAt: time.Unix(0, 0),
			},
			Note{
				Title:     "things-to-do",
				Label:     "misc",
				Size:      19,
				UpdatedAt: time.Unix(0, 0),
			},
		}, false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Skip()
			got, err := list()
			if (err != nil) != tt.wantErr {
				t.Errorf("list() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			// TODO: Find a workaround for this
			for i := range got {
				got[i].UpdatedAt = time.Unix(0, 0)
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("list() = %+v, want %+v", got, tt.want)
			}
		})
	}
}

func Test_delete(t *testing.T) {
	t.Skip()
	_, caller, _, _ := runtime.Caller(0)
	dir := filepath.Dir(caller)
	// Create a temporary directory
	tmpdir, err := ioutil.TempDir("", "banconotes")
	if err != nil {
		panic(err)
	}
	// Copy test cases over
	if err := copy.Copy(filepath.Join(dir, "test_data/test05"), tmpdir); err != nil {
		panic(err)
	}
	// Change to the test directory
	os.Chdir(tmpdir)
	type args struct {
		note Note
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{"remove non existing note", args{note: Note{}}, true},
		{"remove root note", args{note: Note{Title: "hello"}}, false},
		{"remove nested note and empty parent", args{note: Note{Title: "things-to-do", Label: "misc"}}, false},
		{"remove nested note and empty parents of parent", args{note: Note{Title: "hiding", Label: "sub/fol/der"}}, false},
		{"remove nested note and empty parents of parent", args{note: Note{Title: "hiding", Label: "sub2/fol/der"}}, false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Skip()
			if err := delete(tt.args.note); (err != nil) != tt.wantErr {
				t.Errorf("delete() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
	// TODO: This is not the most consitent way of carrying out this test
	// Check if the notes/sub folder has been deleted
	if _, err := os.Stat("notes/sub"); !os.IsNotExist(err) {
		t.Errorf("notes/sub should have been deleted")
	}
	if _, err := os.Stat("notes/sub2/fol"); err != nil {
		t.Errorf("notes/sub2/fol should not have been deleted")
	}
	// Delete tmp directory
	if err := os.RemoveAll(tmpdir); err != nil {
		panic(err)
	}
}

func Test_summary(t *testing.T) {
	t.Skip()
	_, caller, _, _ := runtime.Caller(0)
	dir := filepath.Dir(caller)
	tests := []struct {
		testdir string
		name    string
		want    string
		wantErr bool
	}{
		{"test_data/test01", "no labels", "Notes: 1, Labels: 0", false},
		{"test_data/test02", "one label", "Notes: 1, Labels: 1", false},
		{"test_data/test03", "one nested label", "Notes: 1, Labels: 1", false},
	}
	for _, tt := range tests {
		if err := os.Chdir(filepath.Join(dir, tt.testdir)); err != nil {
			panic(err)
		}
		t.Run(tt.name, func(t *testing.T) {
			t.Skip()
			got, err := summary()
			if (err != nil) != tt.wantErr {
				t.Errorf("summary() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if got != tt.want {
				t.Errorf("summary() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_rename(t *testing.T) {
	t.Skip()
	_, caller, _, _ := runtime.Caller(0)
	dir := filepath.Dir(caller)
	tmpdir, err := ioutil.TempDir("", "banconotes")
	if err != nil {
		panic(err)
	}
	if err := copy.Copy(filepath.Join(dir, "test_data/test04"), tmpdir); err != nil {
		panic(err)
	}
	if err := os.Chdir(tmpdir); err != nil {
		panic(err)
	}
	type args struct {
		current Note
		next    Note
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{"move note to existing subfolder", args{current: Note{
			Title: "hello",
		}, next: Note{
			Title: "hello",
			Label: "meetings",
		}}, false},
		{"rename note and move to a non existing subfolder", args{current: Note{
			Title: "things-to-do",
			Label: "misc",
		}, next: Note{
			Title: "things-done",
			Label: "misc/done",
		}}, false},
		{"move nested note to existing parent", args{current: Note{
			Title: "things-done",
			Label: "misc/done",
		}, next: Note{
			Title: "things-done",
		}}, false},
		{"move note to a subfolder containing a note with the same name", args{current: Note{
			Title: "things-done",
		}, next: Note{
			Title: "hello",
			Label: "meetings",
		}}, true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Skip()
			if err := rename(tt.args.current, tt.args.next); (err != nil) != tt.wantErr {
				t.Errorf("rename() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestModule_Check(t *testing.T) {
	t.Skip()
	tmpdir1, err := ioutil.TempDir("", "banconotes")
	if err != nil {
		panic(err)
	}
	tmpdir2, err := ioutil.TempDir("", "banconotes")
	if err != nil {
		panic(err)
	}
	os.MkdirAll(filepath.Join(tmpdir1, "notes"), os.ModePerm)
	tests := []struct {
		testdir string
		name    string
		b       Module
		wantErr bool
	}{
		{tmpdir1, "existing", Module{}, false},
		{tmpdir2, "not existing", Module{}, true},
	}
	for _, tt := range tests {
		os.Chdir(tt.testdir)
		t.Run(tt.name, func(t *testing.T) {
			t.Skip()
			b := Module{}
			if err := b.Check(); (err != nil) != tt.wantErr {
				t.Errorf("Module.Check() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}
