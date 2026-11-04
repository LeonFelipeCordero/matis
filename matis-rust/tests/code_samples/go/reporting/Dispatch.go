package reporting

import "fmt"

type DispatchFilter func(string) bool

type DispatchQueue struct {
	Notify func(string)
	Entries []string
}

var DispatchReady DispatchFilter = func(name string) bool { return name != "" }

func DispatchReceipts(queue *DispatchQueue, accept DispatchFilter) []string {
	ready := func(name string) bool { return DispatchReady(name) && accept(name) }
	label, archive := func(name string) string { return "pickup:" + name }, func(name string) string { return "closed:" + name }
	queue.Notify = func(name string) { fmt.Println(label(name)) }
	for _, name := range queue.Entries {
		if ready(name) {
			queue.Notify(archive(name))
		}
	}
	return DispatchCollect(queue.Entries, func(name string) string {
		normalize := func(value string) string { return label(value) }
		return normalize(name)
	})
}

func DispatchCollect(names []string, format func(string) string) []string {
	labels := make([]string, 0, len(names))
	for _, name := range names {
		labels = append(labels, format(name))
	}
	return labels
}
