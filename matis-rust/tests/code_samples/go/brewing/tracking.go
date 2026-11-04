package brewing

import "fmt"

// PickupQueue retains real orders until their pickup labels have been printed.
type PickupQueue struct { Orders []Order; Events []string }

func (queue *PickupQueue) Track(order Order, counter string, notes ...string) {
	queue.Orders = append(queue.Orders, order)
	queue.Events = append(queue.Events, TrackingFormat(order, counter))
	queue.Events = append(queue.Events, notes...)
}

var TrackingFormat = func(order Order, counter string) string {
	return fmt.Sprintf("%s: %d drinks at %s", order.Customer, len(order.Names), counter)
}

func (queue *PickupQueue) Collect() []Order {
	orders := queue.Orders
	queue.Orders = nil
	return orders
}
