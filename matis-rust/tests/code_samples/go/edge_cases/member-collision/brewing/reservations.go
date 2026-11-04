package brewing

import (
	"../../../brewing"
	"fmt"
	"strings"
)

func Reserve(order brewing.Order) string {
	return fmt.Sprintf("Reserved for %s: %s", order.Customer, strings.Join(order.Names, ", "))
}

func QueueOrder(inventory *brewing.Inventory, order brewing.Order) string {
	for _, name := range order.Names {
		if !inventory.Reserve(name) {
			return fmt.Sprintf("Waiting for stock: %s", order.Customer)
		}
	}
	return Reserve(order)
}

func RequestSupplier(supplier interface{}, order brewing.Order) string {
	if !supplier.Reserve(order) {
		return fmt.Sprintf("Supplier cannot fill: %s", order.Customer)
	}
	return supplier.DeliveryLabel(order)
}
