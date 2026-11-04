package recipe

import "strings"

// Normalize is a bound callable, not a function-type declaration.
var Normalize = func(name string) string { return strings.ToLower(strings.TrimSpace(name)) }

type Recipe struct {
	Name string
	Cups int
}

func New(name string, cups int) Recipe {
	return Recipe{Name: Normalize(name), Cups: cups}
}

func Portions(name string) int {
	switch name {
	case "espresso":
		return 2
	case "filter":
		return 3
	default:
		return 0
	}
}

func (r Recipe) Dose() int {
	return Portions(r.Name) * r.Cups
}

// Grouped inputs and the variadic label each count as declared inputs.
func (r Recipe) Scale(cups, extra int, labels ...string) Recipe {
	r.Cups = cups + extra
	if len(labels) > 0 {
		r.Name = Normalize(labels[0])
	}
	return r
}
