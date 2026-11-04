package brewing

import (
	"fmt"

	menu "example.com/coffee/recipe"
	"example.com/coffee/reporting"
)

// Brewer shares the shop's stock across customer orders.
type Brewer struct {
	stock *Inventory
}

func NewBrewer(stock *Inventory) *Brewer {
	return &Brewer{stock: stock}
}

func (b *Brewer) Brew(names []string) string {
	available := make([]string, 0, len(names))
	for _, name := range names {
		name = menu.Normalize(name)
		if b.stock.Reserve(name) {
			available = append(available, name)
		}
	}
	return brew(available)
}

func brew(names []string) string {
	if len(names) == 0 {
		return reporting.Render(nil)
	}
	valid := make([]string, 0, len(names))
	accept := func(name string) bool { return menu.Portions(name) > 0 }
	for _, name := range names {
		if accept(name) {
			valid = append(valid, name)
		}
	}
	return reporting.Render(valid)
}

func retry(attempts int) int {
	if attempts <= 0 {
		return 0
	}
	return 1 + retry(attempts-1)
}

func score(batches [][]string) int {
	result := 0
	for _, batch := range batches {
		for _, name := range batch {
			count := menu.Portions(name)
			if count > 0 && count <= 3 {
				result += count
			}
		}
	}
	return result
}

// Classify a batch for the counter, table service, or catering queue.
func classify(amount int) int {
	switch {
	case amount <= 0:
		return 0
	case amount <= 3:
		return 1
	case amount <= 8:
		return 2
	default:
		return 3
	}
}

func ready(water, beans, cups, temperature int) bool {
	return water > 0 && beans > 0 && cups > 0 && temperature >= 90 && temperature <= 96
}

// Estimate batch water in milliliters, including temperature and grind corrections.
func calibrate(beans, ratio, waterOffset, batches, temperature, targetTemperature, grind, targetGrind int) int {
	return (beans*ratio+waterOffset)*batches + (temperature-targetTemperature)*2 + (grind-targetGrind)*5
}

func Main() {
	var stock *Inventory = NewInventory(map[string]int{"espresso": 12, "filter": 9})
	stock.Restock(map[string]int{"filter": 3}, func(_ string, count int) int { return count })
	var brewer *Brewer = NewBrewer(stock)
	var order Order = NewOrder("Maya", "espresso", "filter", "unknown")
	fmt.Println(order.Serve(brewer), Estimate(order))
	fmt.Println(brew(nil), reporting.Preview("espresso"))
	fmt.Println(retry(2), score([][]string{order.Names}), classify(len(order.Names)))
	fmt.Println(ready(600, 40, 3, 93), calibrate(40, 15, 0, 1, 93, 93, 6, 6))
	fmt.Println(stock.Remaining("filter"))
}
