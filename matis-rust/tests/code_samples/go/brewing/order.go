package brewing

import (
	text "fmt"

	"example.com/coffee/recipe"
)

type Order struct {
	Customer string
	Names    []string
}

func NewOrder(customer string, names ...string) Order {
	return Order{Customer: customer, Names: names}
}

func (o Order) Serve(brewer *Brewer) string {
	return text.Sprintf("%s: %s", o.Customer, brewer.Brew(o.Names))
}

func Estimate(order Order) int {
	var drink recipe.Recipe = recipe.New("filter", len(order.Names))
	scaled := drink.Scale(len(order.Names), 0)
	return scaled.Dose()
}
