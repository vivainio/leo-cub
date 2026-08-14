package demos

import "fmt"

type Greeter interface {
	Greet(name string) string
}

type FriendlyGreeter struct {
	Prefix string
}

func (g FriendlyGreeter) Greet(name string) string {
	return fmt.Sprintf("%s, %s!", g.Prefix, name)
}

func NewGreeter(prefix string) FriendlyGreeter {
	return FriendlyGreeter{Prefix: prefix}
}
