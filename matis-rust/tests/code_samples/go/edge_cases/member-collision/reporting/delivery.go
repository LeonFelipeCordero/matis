package reporting

import (
	"example.com/coffee/brewing"
	"fmt"
)

func DeliveryLabel(order brewing.Order) string {
	return fmt.Sprintf("Deliver %d drinks to %s", len(order.Names), order.Customer)
}

func Manifest(orders []brewing.Order) []string {
	labels := make([]string, 0, len(orders))
	for _, order := range orders {
		labels = append(labels, DeliveryLabel(order))
	}
	return labels
}
