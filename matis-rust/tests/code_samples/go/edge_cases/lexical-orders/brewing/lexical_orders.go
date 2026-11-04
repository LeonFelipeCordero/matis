package brewing

import Drink "../../../recipe"

type SeasonalRecipe interface {
	Portions(string) int
}

func rangeOrders(recipes []SeasonalRecipe, name, local string) int {
	for _, Drink := range recipes {
		println(Drink.Portions(local))
	}
	return Drink.Portions(name)
}

func declaredOrders(recipes map[SeasonalRecipe]bool, name, local string) int {
	for Drink := range recipes {
		println(Drink.Portions(local))
	}
	return Drink.Portions(name)
}

func callbackOrders(recipes []SeasonalRecipe, name string) int {
	inspect := func(Drink SeasonalRecipe) int { return Drink.Portions(name) }
	for _, recipe := range recipes {
		println(inspect(recipe))
	}
	return Drink.Portions(name)
}

func pairedCallbacks(recipes []SeasonalRecipe, name string) int {
	inspect := func(other, Drink SeasonalRecipe) int { return other.Portions(name) + Drink.Portions(name) }
	for _, recipe := range recipes {
		println(inspect(recipe, recipe))
	}
	return Drink.Portions(name)
}
