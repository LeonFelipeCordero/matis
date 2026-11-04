package brewing

import "example.com/coffee/recipe"

type Adjustment func(string, int) int

type Inventory struct {
	beans map[string]int
}

func NewInventory(beans map[string]int) *Inventory {
	return &Inventory{beans: beans}
}

func (i Inventory) Remaining(name string) int {
	return i.beans[name]
}

func (i *Inventory) Reserve(name string) bool {
	dose := recipe.Portions(name)
	if dose == 0 || i.Remaining(name) < dose {
		return false
	}
	i.beans[name] -= dose
	return true
}

func (i *Inventory) Restock(delivery map[string]int, adjust Adjustment) {
	apply := func(name string, count int) {
		normalize := func(value int) int { return adjust(name, value) }
		i.beans[name] += normalize(count)
	}
	for name, count := range delivery {
		apply(name, count)
	}
}
