package coffee.selection

class OverloadCoffee {
    constructor(value: String) {
        pour(value)
    }
    constructor(value: Int) {
        pour(value)
    }
    fun pour(value: String) {}
    fun pour(value: Int) {}
    fun count() {}
    fun count(value: Int) {}
    fun defaulted(value: String, cups: Int = 1) {}
    fun batch(label: String, vararg cups: Int) {}
    fun uncertain(value: String) {}
    fun uncertain(value: Any) {}
    fun reference() {}
    fun <T> generic(value: T) {}
    fun generic(value: String) {}
    fun optional(value: String) {}
    fun optional(value: String, cups: Int = 1) {}

    fun run(receiver: OverloadCoffee, label: String, unknown: Any) {
        receiver.pour("espresso")
        receiver.pour(2)
        receiver.pour(label)
        receiver.pour(unknown)
        receiver.count()
        receiver.count(1)
        receiver.count(1, 2)
        receiver.defaulted("coffee")
        receiver.defaulted(cups = 2, value = "coffee")
        receiver.batch("coffee", 1, 2)
        receiver.uncertain("coffee")
        receiver.generic("coffee")
        receiver.optional("coffee")
        receiver.batch("coffee")
        receiver.batch("coffee", "wrong")
        receiver.defaulted(missing = "coffee")
        val known = receiver::reference
        val ambiguous = receiver::pour
    }
}
