package coffee.orders

class ReceiptApi {
    companion object {
        fun from(value: String): String = "Coffee delivery receipt: ${value.trim()}"
    }
}
