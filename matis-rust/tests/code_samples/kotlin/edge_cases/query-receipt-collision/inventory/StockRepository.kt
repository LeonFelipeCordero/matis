package coffee.inventory
fun lookup(query: Any) { query.select().from("stock").fetch() }
