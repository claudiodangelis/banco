package cmd

import (
	"fmt"
	"log"

	"github.com/claudiodangelis/banco/module"
	"github.com/spf13/cobra"
)

func list(m module.Module) error {
	items, err := m.List()
	if err != nil {
		log.Fatalln(err)
	}
	for _, item := range items {
		fmt.Println(item.Name())
	}
	return nil
}

var listCmd = &cobra.Command{
	Use:   "list",
	Short: "List items",
	Run: func(cmd *cobra.Command, args []string) {
		module, err := chooseModule()
		if err != nil {
			log.Fatalln(err)
		}
		if err := list(module); err != nil {
			log.Fatalln(err)
		}
	},
}
