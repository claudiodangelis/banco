package cmd

import (
	"log"

	"github.com/claudiodangelis/banco/item"
	"github.com/claudiodangelis/banco/module"
	"github.com/spf13/cobra"
)

func open(module module.Module, item item.Item) error {
	return module.OpenItem(item)
}

var openCmd = &cobra.Command{
	Use:   "open",
	Short: "Open an item",
	Run: func(cmd *cobra.Command, args []string) {
		module, err := chooseModule()
		if err != nil {
			log.Fatalln(err)
		}
		item, err := chooseItem(module, false)
		if err != nil {
			log.Fatalln(err)
		}
		if err := open(module, item); err != nil {
			log.Fatalln(err)
		}
	},
}
