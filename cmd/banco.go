package cmd

import (
	"fmt"
	"os"

	"github.com/claudiodangelis/banco/util"

	"github.com/claudiodangelis/banco/module"
	"github.com/manifoldco/promptui"
	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{
	Use:   "banco",
	Short: "Launch banco",
	Long:  "Launch banco",
	Run: func(cmd *cobra.Command, args []string) {
		util.ClearScreen()
		// Show summaries
		modules := make(map[string]module.Module)
		modulesSlice := []string{}
		for _, m := range module.All() {
			modules[m.Name()] = m
			modulesSlice = append(modulesSlice, m.Name())
			if err := m.CmdSummary().Execute(); err != nil {
				panic(err)
			}
		}
		// Prompt modules
		prompt := promptui.Select{
			Label: "Modules",
			Items: modulesSlice,
		}
		_, result, err := prompt.Run()
		if err != nil {
			panic(err)
		}
		util.ClearScreen()
		modules[result].CmdRoot().Execute()
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
