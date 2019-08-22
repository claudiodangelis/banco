package cmd

import (
	"fmt"

	"github.com/claudiodangelis/banco/module"
	"github.com/spf13/cobra"
)

var modulesCmd = &cobra.Command{
	Use:   "modules",
	Short: "List available modules",
	Long:  "List available modules",
	Args:  cobra.MaximumNArgs(0),
	Run: func(cmd *cobra.Command, args []string) {
		for _, m := range module.All() {
			fmt.Println(m.Name())
		}
	},
}
