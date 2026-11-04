package brewing

import (
    "time"
    clock "fmt"
    _ "net/http/pprof"
    . "math"
)

// ScheduledPickup rounds preparation time up to the next whole minute.
func ScheduledPickup(order Order, now time.Time) string {
    minutes := int(Ceil(float64(len(order.Names)) / 2))
    ready := now.Add(time.Duration(minutes) * time.Minute)
    return clock.Sprintf("%s: %s", order.Customer, ready.Format("15:04"))
}
