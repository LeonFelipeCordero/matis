package coffee.safety

class SafetyCoffee<Int> {
    fun generic(value: Int) {}
    fun generic(value: Boolean) {}
    fun nullable(value: String?) {}
    fun nullable(value: Boolean) {}
    fun accepts(value: String) {}
    fun accepts(value: Boolean) {}
    fun uncertain(value: Missing) {}
    fun uncertain(value: Boolean) {}
    fun count() {}
    fun count(value: Boolean) {}
    fun run(ambiguous: Int, nullableValue: String?, missing: Missing, aliased: Float) {
        generic(true)
        accepts(ambiguous)
        nullable(nullableValue)
        uncertain(missing)
        accepts("coffee")
        count()
        count(true)
        accepts(aliased)
        nullable(true)
    }
}
