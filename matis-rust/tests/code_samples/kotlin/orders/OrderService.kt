package coffee.orders

import coffee.brewing.Brewer as CoffeeBrewer
import coffee.inventory.Inventory

interface CoffeeOrders {
    fun fulfill(order: Order): String
}

class OrderService(private val inventory: Inventory, private val brewer: CoffeeBrewer) : CoffeeOrders {
    private val orders = mutableListOf<Order>()
    private val events = mutableListOf<String>()
    private val audit: (Order) -> Unit = { order -> events.add(order.customer) }

    constructor(inventory: Inventory) : this(inventory, CoffeeBrewer())

    fun place(customer: String, vararg lines: OrderLine): Order {
        return place(customer, emptyMap(), audit, *lines)
    }

    fun place(customer: String, notes: Map<String, String>,
              onReady: (Order) -> Unit, vararg lines: OrderLine): Order {
        val order = Order(customer, lines.toList(), notes)
        orders.add(order)
        if (fulfill(order).isNotEmpty()) onReady(order)
        return order
    }

    override fun fulfill(order: Order): String {
        if (!inventory.reserve(order)) return ""
        val receipt = brewer.brew(order.names())
        order.transition(OrderStatus.BREWED)
        return receipt
    }

    fun preview(station: CoffeeBrewer, order: Order): String = station.brew(order.names())

    fun receipts(formatter: (Order) -> String = { it.receipt() }): List<String> {
        fun formatReady(order: Order): String {
            return if (order.status == OrderStatus.BREWED) formatter(order) else "pending"
        }
        return orders.map(::formatReady)
    }

    suspend fun notify(send: suspend (String) -> Unit) {
        for (receipt in receipts()) send(receipt)
    }

    @Deprecated("Use place with OrderLine for explicit quantity and price")
    fun place(customer: String, drink: String): Order = place(customer, OrderLine(drink))

    companion object {
        fun stocked(espresso: Int = 40, filter: Int = 60): OrderService {
            val stock = Inventory.stocked(espresso, filter)
            return OrderService(stock)
        }
    }
}
