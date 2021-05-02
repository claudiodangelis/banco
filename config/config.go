package config

import (
	"fmt"
	"strings"
	"time"

	"github.com/claudiodangelis/banco/item"
	"github.com/spf13/viper"
)

type Config struct{}

func (c Config) Get(s string) interface{} {
	return viper.Get(s)
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
	viper.SetConfigName(".banco.yaml")
	viper.SetConfigType("yaml")
	viper.AddConfigPath("$HOME/.config/banco")
	viper.AddConfigPath(".")
	err := viper.ReadInConfig()
	if err != nil {
		panic(fmt.Errorf("Fatal error config file: %s \n", err))
	}
	return Config{}
}
