package cmd

import (
	"log"

	"github.com/claudiodangelis/banco/item"
	"github.com/claudiodangelis/banco/ui"

	"github.com/claudiodangelis/banco/module"
	"github.com/spf13/cobra"
)

func update(module module.Module, current item.Item) error {
	// Ask for properties
	next := make(item.Item)
	for _, input := range module.UpdateItemParameters() {
		// TODO: Replace with enums
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
		}
		next[input.Name] = result
	}
	err := module.UpdateItem(current, next)
	return err
}

var updateCmd = &cobra.Command{
	Use:   "update",
	Short: "Update an item",
	Run: func(cmd *cobra.Command, args []string) {
		module, err := chooseModule()
		if err != nil {
			log.Fatalln(err)
		}
		item, err := chooseItem(module)
		if err != nil {
			log.Fatalln(err)
		}
		if err := update(module, item); err != nil {
			log.Fatalln(err)
		}
	},
}
