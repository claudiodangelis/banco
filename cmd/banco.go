package cmd

import (
	"errors"
	"fmt"
	"log"

	"github.com/claudiodangelis/banco/item"
	"github.com/claudiodangelis/banco/module"
	"github.com/claudiodangelis/banco/ui"

	"github.com/spf13/cobra"
)

func init() {
	rootCmd.AddCommand(initCmd)
	// Append sub commands
	for _, module := range module.All() {
		// List commands
		fmt.Println("appending sub command for", module.Name())
		listCmd.AddCommand(&cobra.Command{
			Use:   module.Name(),
			Short: fmt.Sprintf("List %s", module.Name()),
			Run: func(cmd *cobra.Command, args []string) {
				if err := list(module); err != nil {
					log.Fatalln(err)
				}
			},
		})
		// New commands
		newCmd.AddCommand(&cobra.Command{
			Use:   module.Singular(),
			Short: fmt.Sprintf("Create a new %s", module.Singular()),
			Run: func(cmd *cobra.Command, args []string) {
				if err := create(module); err != nil {
					log.Fatalln(err)
				}
			},
		})
		// Update commands
		updateCmd.AddCommand(&cobra.Command{
			Use:   module.Singular(),
			Short: fmt.Sprintf("Update a %s", module.Singular()),
			Run: func(cmd *cobra.Command, args []string) {
				item, err := chooseItem(module)
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
			Use:   module.Singular(),
			Short: fmt.Sprintf("Delete a %s", module.Singular()),
			Run: func(cmd *cobra.Command, args []string) {
				item, err := chooseItem(module)
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
			Use:   module.Singular(),
			Short: fmt.Sprintf("Open a %s", module.Singular()),
			Run: func(cmd *cobra.Command, args []string) {
				item, err := chooseItem(module)
				if err != nil {
					log.Fatalln(err)
				}
				if err := open(module, item); err != nil {
					log.Fatalln(err)
				}
			},
		})
		rootCmd.AddCommand(&cobra.Command{
			Use:   module.Name(),
			Short: fmt.Sprintf("Show banco for %s", module.Name()),
			Run: func(cmd *cobra.Command, args []string) {
				if err := root(module); err != nil {
					log.Fatalln(err)
				}
			},
		})
	}
	rootCmd.AddCommand(listCmd)
	rootCmd.AddCommand(newCmd)
	rootCmd.AddCommand(updateCmd)
	rootCmd.AddCommand(deleteCmd)
	rootCmd.AddCommand(openCmd)
}

// chooseItem is an utility function that prompts a list of available
// items
func chooseItem(module module.Module) (item.Item, error) {
	// Pick item
	// TODO: Lots of duplicate code here, refactor
	items, err := module.List()
	if err != nil {
		return item.Item{}, err
	}
	// Create a map of items
	// TODO: Research improvement
	itemsDict := make(map[string]item.Item)
	names := []string{}
	for _, item := range items {
		itemsDict[item.Name()] = item
		names = append(names, item.Name())
	}
	// Prompt list of items
	result, err := ui.Select(fmt.Sprintf("Choose a %s", module.Singular()), names, true)
	if err != nil {
		return item.Item{}, err
	}
	item := itemsDict[result]
	return item, nil
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
	choice, err := ui.Select("Choose module", names, false)
	if err != nil {
		return nil, err
	}
	return dict[choice], nil
}

func root(module module.Module) error {
	item, err := chooseItem(module)
	if err != nil {
		return err
	}
	// List what you want to do with the item
	// TODO: Use enums
	actions := []string{
		"Open", "Update", "Delete",
	}
	action, err := ui.Select("What you want to do?", actions, false)
	if err != nil {
		return err
	}
	if action == "Open" {
		return open(module, item)
	} else if action == "Delete" {
		return delete(module, item)
	} else if action == "Update" {
		return update(module, item)
	}
	return errors.New("invalid choice")
}

var rootCmd = &cobra.Command{
	Use:   "banco",
	Short: "Launch banco",
	Run: func(cmd *cobra.Command, args []string) {
		for {
			ui.ClearScreen()
			module, err := chooseModule()
			if err != nil {
				log.Fatalln(err)
			}
			if err := root(module); err != nil {
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
