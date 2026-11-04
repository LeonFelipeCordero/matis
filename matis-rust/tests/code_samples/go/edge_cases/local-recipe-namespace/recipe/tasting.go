package recipe

import (
	"fmt"
	recipe "../../../recipe"
)

func Tasting(name string) string {
	servings := recipe.Portions(name)
	return fmt.Sprintf("Tasting flight: %s, %d portions", name, servings)
}
