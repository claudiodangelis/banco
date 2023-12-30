package item

// Item of a banco module
type Item struct {
	Parameters map[string]string
	// Resource can be a path or URL
	Resource string
}

// Parameter of the item
type Parameter struct {
	Name      string
	InputType string
	Default   string
	Options   []string
}
