package cmd

import (
	"log"

	"github.com/claudiodangelis/banco/item"
	"github.com/claudiodangelis/banco/module"
	"github.com/claudiodangelis/banco/ui"
	"github.com/spf13/cobra"
)

// create a new item. This should have been called new() for consistency,
// but that's a reserved word in Go ¯\_(ツ)_/¯
func create(m module.Module) error {
	// Create empty
	item := make(item.Item)
	for _, input := range m.NewItemParameters() {
		var result string
		if input.InputType == ui.InputText {
			output, err := ui.Input(input.Name, input.Default)
			if err != nil {
				return err
			}
			result = output
		} else if input.InputType == ui.InputSelectWithAdd {
			output, err := ui.SelectWithAdd(input.Name, input.Default, input.Options)
			if err != nil {
				return err
			}
			result = output
		} else if input.InputType == ui.InputSelect {
			output, err := ui.Select(input.Name, input.Options, false)
			if err != nil {
				return err
			}
			result = output
		}
		item[input.Name] = result
	}
	if err := m.SaveItem(item); err != nil {
		return err
	}
	// Open it
	err := m.OpenItem(item)
	return err
}

var newCmd = &cobra.Command{
	Aliases: []string{"create"},
	Use:     "new",
	Short:   "Create new item",
	Run: func(cmd *cobra.Command, args []string) {
		module, err := chooseModule()
		if err != nil {
			log.Fatalln(err)
		}
		if err := create(module); err != nil {
			log.Fatalln(err)
		}
	},
}
