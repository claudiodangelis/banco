package cmd

import (
	"fmt"

	"github.com/claudiodangelis/banco/module"
	"github.com/spf13/cobra"
)

var checkCmd = &cobra.Command{
	Use:   "check",
	Short: "Run a healthcheck",
	Long:  "Run a healthcheck",
	Args:  cobra.MaximumNArgs(0),
	Run: func(cmd *cobra.Command, args []string) {
		for _, m := range module.All() {
			if err := m.Check(); err != nil {
				fmt.Println(m.Name(), "ERROR:", err)
				continue
			}
			fmt.Println(m.Name(), "OK")
		}
	},
}
