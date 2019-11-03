package item

// Item of a banco module
type Item map[string]string

// Name is a string representation of the item
func (i Item) Name() string {
	return i["Name"]
}

// Parameter of the item
type Parameter struct {
	Name      string
	InputType string
	Default   string
	Options   []string
}
