package cmd

import (
	"fmt"
	"os"

	"github.com/claudiodangelis/banco/module"
	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{
	Use:   "banco",
	Short: "Launch banco",
	Long:  "Launch banco",
	Run: func(cmd *cobra.Command, args []string) {
	},
}

func init() {
	// Loop through modules
	for _, m := range module.All() {
		newCmd.AddCommand(m.CmdNew())
		listCmd.AddCommand(m.CmdList())
		rootCmd.AddCommand(m.CmdRoot())
		deleteCmd.AddCommand(m.CmdDelete())
	}
	rootCmd.AddCommand(initCmd)
	rootCmd.AddCommand(newCmd)
	rootCmd.AddCommand(checkCmd)
	rootCmd.AddCommand(openCmd)
	rootCmd.AddCommand(updateCmd)
	rootCmd.AddCommand(deleteCmd)
	rootCmd.AddCommand(listCmd)
	rootCmd.AddCommand(modulesCmd)
}

// Execute the root command
func Execute() {
	if err := rootCmd.Execute(); err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}
