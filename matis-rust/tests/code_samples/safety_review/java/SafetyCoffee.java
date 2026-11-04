package coffee.safety;

class SafetyCoffee<String> {
    void generic(String value) {}
    void generic(boolean value) {}
    void primitive(int value) {}
    void primitive(boolean value) {}
    void widened(long value) {}
    void widened(double value) {}
    void boxed(Integer value) {}
    void boxed(boolean value) {}
    void unknown(Missing value) {}
    void unknown(boolean value) {}
    void count(int value) {}
    void count(int first, int second) {}
    void run(String ambiguous, Integer boxedValue, Missing missing) {
        generic(true);
        primitive(ambiguous);
        primitive(1);
        widened(1);
        boxed(boxedValue);
        boxed(true);
        unknown(missing);
        count(1);
        count(1, 2);
    }
}
