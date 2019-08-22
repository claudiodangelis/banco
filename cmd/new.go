package cmd

import (
	"github.com/spf13/cobra"
)

var newCmd = &cobra.Command{
	Use:   "new",
	Short: "Creates a new item",
	Long:  "Creates a new item from the passed module",
	Run: func(cmd *cobra.Command, args []string) {
	},
}
