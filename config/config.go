package config

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/claudiodangelis/banco/item"
	"github.com/spf13/viper"
)

func initConfigFile() {
	home, _ := os.UserHomeDir()
	path := filepath.Join(home, ".config/banco")
	if err := os.MkdirAll(path, os.ModePerm); err != nil {
		panic(err)
	}
	// TODO: Add a template for the config file
	if _, err := os.OpenFile(filepath.Join(path, "config.yml"), os.O_RDONLY|os.O_CREATE, 0666); err != nil {
		panic(err)
	}
}

type Config struct {
	Path string
}

var instances map[string]*viper.Viper

func init() {
	instances = make(map[string]*viper.Viper)
}

func (c Config) Get(s string) interface{} {
	fmt.Println(len(instances))
	for k, v := range instances {
		fmt.Println("k", k)
		fmt.Println("v", v)
	}
	fmt.Printf(".Get %p", &c)
	return instances[fmt.Sprintf("%p", &c)].Get(s)
}

// GetDefaultTitle returns the default title based on some values
func (c Config) GetDefaultTitle(module string, items []item.Item) string {
	if c.Get(fmt.Sprintf("%s.title", module)) == nil {
		return ""
	}
	title := c.Get(fmt.Sprintf("%s.title", module)).(string)
	title = strings.ReplaceAll(title, "$id", fmt.Sprintf("%04d", len(items)+1))
	title = strings.ReplaceAll(title, "$timestamp", time.Now().Format("20060102"))

	return title
}

func New() Config {
	initConfigFile()
	cfg := Config{}
	viperInstance := viper.New()
	viperInstance.SetConfigName("config.yml")
	viperInstance.SetConfigType("yaml")
	viperInstance.AddConfigPath(".banco")
	viperInstance.AddConfigPath("$HOME/.config/banco")
	if err := viperInstance.ReadInConfig(); err != nil {
		panic(fmt.Errorf("fatal error config file: %s", err))
	}
	cfg.Path = viperInstance.ConfigFileUsed()
	fmt.Println("about to add", fmt.Sprintf("%p", &cfg), "with value", viperInstance)
	instances[fmt.Sprintf("%p", &cfg)] = viperInstance
	return cfg
}
