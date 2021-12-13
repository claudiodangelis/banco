package config

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/spf13/viper"
)

func init() {
	initConfigFile()
}

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

func (c Config) Get(s string) string {
	return viperInstance.GetString(s)
}

// GetDefaultTitle returns the default title based on some values
func (c Config) GetDefaultTitle(module string) string {
	return c.Get(fmt.Sprintf("%s.title", module))
	// TODO: This should be moved inside each module package
	// title = strings.ReplaceAll(title, "$id", fmt.Sprintf("%04d", len(items)+1))
	// title = strings.ReplaceAll(title, "$timestamp", time.Now().Format("20060102"))
}

// Returns the path to the template, if it exists, otherwise empty string
func (c Config) GetTemplatePath(module, label string) (string, bool) {
	// Check if template exists
	parts := strings.Split(label, "/")
	path := append([]string{filepath.Dir(c.Path), "templates", module}, parts...)
	path = append(path, "template")
	if _, err := os.Stat(filepath.Join(path...)); errors.Is(err, os.ErrNotExist) {
		return "", false
	}
	return filepath.Join(path...), true
}

var viperInstance *viper.Viper

func New() Config {
	cfg := Config{}
	viperInstance = viper.New()
	viperInstance.SetConfigName("config.yml")
	viperInstance.SetConfigType("yaml")
	viperInstance.AddConfigPath(".banco")
	viperInstance.AddConfigPath("$HOME/.config/banco")
	if err := viperInstance.ReadInConfig(); err != nil {
		panic(fmt.Errorf("fatal error config file: %s", err))
	}
	cfg.Path = viperInstance.ConfigFileUsed()
	return cfg
}
