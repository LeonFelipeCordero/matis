package reporting

import (
	"fmt"
	"strings"

	"example.com/coffee/recipe"
)

type Report struct {
	Names []string
}

func Total(names []string) int {
	result := 0
	for _, name := range names {
		result += recipe.Portions(name)
	}
	return result
}

func Render(names []string) string {
	entries := make([]string, 0, len(names))
	for _, name := range names {
		entries = append(entries, fmt.Sprintf("%s=%d", name, recipe.Portions(name)))
	}
	return fmt.Sprintf("coffee:%s;total=%d", strings.Join(entries, ","), Total(names))
}

func (r Report) Receipt() string {
	return Render(r.Names)
}

// Preview lives here so the recipe package never imports its consumers.
func Preview(name string) string {
	var report Report = Report{Names: []string{name}}
	return report.Receipt()
}
