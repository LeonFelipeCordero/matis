package brewing

import "example.com/coffee/recipe"

func route(active bool, amount int) string {
	if active {
		if amount > 8 {
			return "courier"
		} else if amount > 3 {
			return "table"
		} else if amount > 0 {
			return "counter"
		}
	}
	return "hold"
}

func quote(name string) int {
	return recipe.Portions(name)
}
