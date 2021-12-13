package cmd

import (
	"errors"
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"

	"github.com/claudiodangelis/banco/util"

	"github.com/claudiodangelis/banco/item"
	"github.com/claudiodangelis/banco/module"
	"github.com/claudiodangelis/banco/ui"

	"github.com/spf13/cobra"
)

func init() {
	rootCmd.AddCommand(initCmd)
	rootCmd.AddCommand(initConfigCmd)
	modules := []string{}
	// Append sub commands
	for _, m := range module.All() {
		module := m
		modules = append(modules, module.Name())
		// List commands
		listCmd.AddCommand(&cobra.Command{
			Use:     module.Name(),
			Aliases: module.Aliases(),
			Short:   fmt.Sprintf("List %s", module.Name()),
			Run: func(cmd *cobra.Command, args []string) {
				if err := list(module); err != nil {
					log.Fatalln(err)
				}
			},
		})
		// New commands
		newCmd.AddCommand(&cobra.Command{
			Use:     module.Singular(),
			Aliases: module.Aliases(),
			Short:   fmt.Sprintf("Create a new %s", module.Singular()),
			Run: func(cmd *cobra.Command, args []string) {
				if err := create(module); err != nil {
					log.Fatalln(err)
				}
			},
		})
		// Update commands
		updateCmd.AddCommand(&cobra.Command{
			Use:     module.Singular(),
			Aliases: module.Aliases(),
			Short:   fmt.Sprintf("Update a %s", module.Singular()),
			Run: func(cmd *cobra.Command, args []string) {
				item, err := chooseItem(module, false)
				if err != nil {
					log.Fatalln(err)
				}
				if err := update(module, item); err != nil {
					log.Fatalln(err)
				}
			},
		})
		// Delete commands
		deleteCmd.AddCommand(&cobra.Command{
			Use:     module.Singular(),
			Aliases: module.Aliases(),
			Short:   fmt.Sprintf("Delete a %s", module.Singular()),
			Run: func(cmd *cobra.Command, args []string) {
				item, err := chooseItem(module, false)
				if err != nil {
					log.Fatalln(err)
				}
				if err := delete(module, item); err != nil {
					log.Fatalln(err)
				}
			},
		})
		// Open commands
		openCmd.AddCommand(&cobra.Command{
			Use:     module.Singular(),
			Aliases: module.Aliases(),
			Short:   fmt.Sprintf("Open a %s", module.Singular()),
			Run: func(cmd *cobra.Command, args []string) {
				item, err := chooseItem(module, false)
				if err != nil {
					log.Fatalln(err)
				}
				if err := open(module, item); err != nil {
					log.Fatalln(err)
				}
			},
		})
		rootCmd.AddCommand(&cobra.Command{
			Use:     module.Name(),
			Aliases: module.Aliases(),
			Short:   fmt.Sprintf("Show banco for %s", module.Name()),
			Run: func(cmd *cobra.Command, args []string) {
				if err := root(module); err != nil {
					log.Fatalln(err)
				}
			},
		})
	}
	rootCmd.AddCommand(&cobra.Command{
		Use:   "modules",
		Short: "Show available modules",
		Run: func(cmd *cobra.Command, args []string) {
			for _, module := range modules {
				fmt.Println(module)
			}
		},
	})
	rootCmd.AddCommand(listCmd)
	rootCmd.AddCommand(newCmd)
	rootCmd.AddCommand(updateCmd)
	rootCmd.AddCommand(deleteCmd)
	rootCmd.AddCommand(openCmd)
}

// chooseItem is an utility function that prompts a list of available
// items for the given module
// TODO: This must be refactored because it's very hard to read
func chooseItem(module module.Module, withAdd bool) (item.Item, error) {
	items, err := module.List()
	if err != nil {
		return item.Item{}, err
	}
	// Create a map of items
	itemsDict := make(map[string]item.Item)
	names := []string{}
	for _, item := range items {
		itemsDict[item.Name()] = item
		names = append(names, item.Name())
	}
	if withAdd {
		names = append([]string{"+ Create a new one"}, names...)
	}
	// Prompt list of items
	result, err := ui.Select(fmt.Sprintf("Choose a %s or create one",
		module.Singular()), names, "", true)
	if err != nil {
		return item.Item{}, err
	}
	item := itemsDict[result]
	return item, nil
}

// chooseModuleWithSummary is an utility function that prompts
// a list of available modules with a short summary
func chooseModuleWithSummary() (module.Module, error) {
	// TODO: This function can be merged into chooseModule()
	modules := module.All()
	dict := make(map[string]module.Module)
	summaries := make([]string, 0, len(modules))
	for _, module := range modules {
		dict[module.Summary()] = module
		summaries = append(summaries, module.Summary())
	}
	choice, err := ui.Select("Choose module", summaries, "", false)
	if err != nil {
		return nil, err
	}
	return dict[choice], nil
}

// chooseModule is an utility function that prompts
// a list of available modules
func chooseModule() (module.Module, error) {
	modules := module.All()
	dict := make(map[string]module.Module)
	names := make([]string, 0, len(modules))
	for _, module := range modules {
		dict[module.Name()] = module
		names = append(names, module.Name())
	}
	choice, err := ui.Select("Choose module", names, "", false)
	if err != nil {
		return nil, err
	}
	return dict[choice], nil
}

func root(module module.Module) error {
	// TODO: Since the `documents` module is at an early stage, let user
	// choose if they want to use an external file manager for documents
	if fm := os.Getenv("FILEMANAGER"); fm != "" && module.Name() == "documents" {
		cmd := exec.Command(fm, module.Name())
		cmd.Stdin = os.Stdin
		cmd.Stdout = os.Stdout
		if err := cmd.Run(); err != nil {
			fmt.Println(err)
			return err
		}
		return nil
	}
	item, err := chooseItem(module, true)
	if err != nil {
		return err
	}
	// TODO: This must be refactored because it's impossible to understand
	// `chooseItem()` returns an blank item if the user chooses to create
	// a new one. Since an item is a type of map, if len == 0, then item
	// is empty
	if len(item) == 0 {
		return create(module)
	}
	// List what you want to do with the item
	action, err := ui.Select("What you want to do?", ui.ActionsAll, "", false)
	if err != nil {
		return err
	}
	switch action {
	case ui.ActionOpen:
		return open(module, item)
	case ui.ActionUpdate:
		return update(module, item)
	case ui.ActionDelete:
		return delete(module, item)
	}
	return errors.New("invalid choice")
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
			module, err := chooseModuleWithSummary()
			if err != nil {
				log.Fatalln(err)
			}
			switch err := root(module); err {
			case ui.ErrInterrupt:
				ui.ClearScreen()
				continue
			default:
				log.Fatalln(err)
			}
		}
	},
}

// Execute the root command
func Execute() {
	if err := rootCmd.Execute(); err != nil {
		log.Fatalln(err)
	}
}
