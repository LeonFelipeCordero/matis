package brewing

import menu "example.com/coffee/recipe"

func seasonalOrders(names []string, replacement func(string) bool) []string {
    accept := func(name string) bool { return menu.Portions(name) > 0 }
    accept = replacement
    valid := make([]string, 0, len(names))
    for _, name := range names {
        if accept(name) { valid = append(valid, name) }
    }
    return valid
}
