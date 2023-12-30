package cmd

import (
	"log"

	"github.com/claudiodangelis/banco/item"
	"github.com/claudiodangelis/banco/module"
	"github.com/claudiodangelis/banco/provider"
	"github.com/claudiodangelis/banco/ui"
	"github.com/spf13/cobra"
)

func create(m module.Module, prv provider.Provider) error {
	// Create empty
	item := item.Item{}
	params := make(map[string]string)
	for _, input := range prv.NewItemParameters() {
		var result string
		if input.InputType == ui.InputText {
			output, err := ui.Input(input.Name, input.Default)
			if err != nil {
				return err
			}
			result = output
		} else if input.InputType == ui.InputSelectWithAdd {
			output, err := ui.SelectWithAdd(input.Name, input.Default, input.Options, true)
			if err != nil {
				return err
			}
			result = output
		} else if input.InputType == ui.InputSelect {
			output, err := ui.Select(input.Name, input.Options, input.Default, false)
			if err != nil {
				return err
			}
			result = output
		}
		params[input.Name] = result
	}
	item.Parameters = params
	result, err := prv.SaveItem(item)
	if err != nil {
		return err
	}
	// Open it
	return m.OpenItem(result)
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
		provider := chooseProvider(module)
		if err := create(module, provider); err != nil {
			log.Fatalln(err)
		}
	},
}
