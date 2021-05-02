package config

import (
	"fmt"

	"github.com/spf13/viper"
)

type Config struct{}

func (c Config) Get(s string) interface{} {
	return viper.Get(s)
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
