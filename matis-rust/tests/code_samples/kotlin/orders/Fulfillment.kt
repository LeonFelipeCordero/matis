package coffee.orders

import coffee.brewing.portions

class Fulfillment {
    fun ready(paid: Boolean, packed: Boolean, cups: Int, courier: Boolean): Boolean {
        if (paid && packed || cups > 0 && courier) return true
        return false
    }

    fun route(active: Boolean, amount: Int): String {
        if (active) {
            if (amount > 8) return "courier"
            else if (amount > 3) return "table"
            else if (amount > 0) return "counter"
        }
        return "hold"
    }

    fun reassign(active: Boolean, ready: Boolean, courier: Boolean): String {
        if (active) {
            if (ready) return "counter"
            else {
                if (courier) return "courier"
            }
        }
        return "hold"
    }

    fun quote(amount: Int = if (portions("espresso") > 0) 2 else 1): Int {
        if (amount > 0) return amount
        return 0
    }
}
