package cmd

import (
	"fmt"
	"log"
	"os"

	"github.com/claudiodangelis/banco/config"
	"github.com/claudiodangelis/banco/module"
	"github.com/claudiodangelis/banco/util"
	"github.com/spf13/cobra"
)

var initCmd = &cobra.Command{
	Use:   "init",
	Short: "Initializes a new Banco project",
	Long:  "Initializes a new Banco project",
	Args:  cobra.MaximumNArgs(0),
	Run: func(cmd *cobra.Command, args []string) {
		// Check if the current directory is empty
		dir, err := os.Getwd()
		if err != nil {
			log.Fatalln(err)
		}
		dirempty, err := util.IsEmptyDir(dir)
		if !dirempty {
			log.Fatalf("directory %s is not empty", dir)
		}
		if err != nil {
			log.Fatalln(err)
		}
		// You can initialize modules now
		for _, m := range module.All() {
			if err := m.Init(); err != nil {
				// TODO: Replace with proper logging
				log.Println(err)
			}
		}
	},
}

var initConfigCmd = &cobra.Command{
	Use:   "init-config",
	Short: "Initializes a new configuration directory in the current Banco project",
	Long:  "Initializes a new configuration directory in the current Banco project",
	Run: func(cmd *cobra.Command, args []string) {
		if ok, err := util.IsBanco(); !ok {
			fmt.Println(err)
			os.Exit(1)
		}
		config.InitCustomConfigDirectory(module.AllNamesWithTemplates())
	},
}
