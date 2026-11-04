package tasting

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
