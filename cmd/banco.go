package cmd

import (
	"fmt"
	"log"
	"os"
	"path/filepath"

	"github.com/claudiodangelis/banco/module"
	"github.com/claudiodangelis/banco/provider"
	"github.com/claudiodangelis/banco/ui"
	"github.com/claudiodangelis/banco/util"
	"github.com/spf13/cobra"
)

func init() {
	rootCmd.AddCommand(initCmd)
	rootCmd.AddCommand(newCmd)
}

var rootCmd = &cobra.Command{
	Use:   "banco",
	Short: "Launch banco",
	PersistentPreRun: func(cmd *cobra.Command, args []string) {
		// If you are going to initialize, no need to check if this
		// is a banco directory
		if cmd.Use == "init" {
			return
		}
		if _, err := util.IsBanco(); err != nil {
			log.Fatalln("This is not a banco directory:", err)
		}
	},
	Run: func(cmd *cobra.Command, args []string) {
		for {
			ui.ClearScreen()
			wd, err := os.Getwd()
			if err != nil {
				panic(err)
			}
			fmt.Printf("Welcome to Banco! [Project: %s]\n", filepath.Base(wd))
			module, err := chooseModule()
			if err != nil {
				log.Fatalln(err)
			}
			fmt.Println("you want", module)
			// TODO: review the following
			// switch err := root(module); err {
			// case ui.ErrInterrupt:
			// 	ui.ClearScreen()
			// 	continue
			// default:
			// 	log.Fatalln(err)
			// }
		}
	},
}

// Execute the root command
func Execute() {
	if err := rootCmd.Execute(); err != nil {
		log.Fatalln(err)
	}
}

// chooseModule is an utility function that prompts
// a list of available modules with a short summary
func chooseModule() (module.Module, error) {
	modules := module.All()
	dict := make(map[module.ModuleName]module.Module)
	summaries := make([]string, 0, len(modules))
	for _, module := range modules {
		dict[module.Name] = module
		summaries = append(summaries, string(module.Name))
	}
	choice, err := ui.Select("Choose module", summaries, "", false)
	if err != nil {
		return module.Module{}, err
	}
	return dict[module.ModuleName(choice)], nil
}

// chooseProvider is an utility fuynction that prompts
// a list of available providers for the given module.
// If only one provider is available, no list is prompted
func chooseProvider(m module.Module) provider.Provider {
	// TODO: only list providers with proper capability
	// i.e.: create, delete, etc
	if len(m.Providers) == 1 {
		for _, provider := range m.Providers {
			return provider
		}
	}
	var providers []string
	for alias := range m.Providers {
		providers = append(providers, alias)
	}
	choice, err := ui.Select("Choose provider", providers, "", false)
	if err != nil {
		// TODO: handle this properly
		panic(err)
	}
	return m.Providers[choice]
}
