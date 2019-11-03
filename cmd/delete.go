package cmd

import (
	"log"

	"github.com/claudiodangelis/banco/item"
	"github.com/claudiodangelis/banco/module"
	"github.com/claudiodangelis/banco/ui"
	"github.com/spf13/cobra"
)

func delete(module module.Module, item item.Item) error {
	// Ask if users really want to delete
	confirm, err := ui.Select("Do you really want to delete this item?", []string{"Yes", "No"}, false)
	if err != nil {
		return err
	}
	if confirm == "No" {
		return nil
	}
	err = module.DeleteItem(item)
	return err
}

var deleteCmd = &cobra.Command{
	Use:   "delete",
	Short: "Delete an item",
	Run: func(cmd *cobra.Command, args []string) {
		module, err := chooseModule()
		if err != nil {
			log.Fatalln(err)
		}
		item, err := chooseItem(module)
		if err != nil {
			log.Fatalln(err)
		}
		if err := delete(module, item); err != nil {
			log.Fatalln(err)
		}
	},
}
