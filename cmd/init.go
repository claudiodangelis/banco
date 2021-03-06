package cmd

import (
	"log"
	"os"

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
		if dirempty == false {
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
