package repos

// Module module
type Module struct{}

// Name of this module
func (b Module) Name() string {
	return "repos"
}

// Init initializes the module
func (b Module) Init() error {
	return nil
}

// Check checks module's sanity
func (b Module) Check() error {
	return nil
}

// New module
func New() Module {
	return Module{}
}
