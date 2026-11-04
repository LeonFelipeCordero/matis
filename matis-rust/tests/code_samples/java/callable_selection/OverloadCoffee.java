package coffee.selection;

class OverloadCoffee {
    void pour(String value) {}
    void pour(int value) {}
    void count() {}
    void count(int value) {}
    void batch(String label, int... cups) {}
    void uncertain(String value) {}
    void uncertain(Object value) {}
    void reference() {}
    <T> void generic(T value) {}
    void generic(String value) {}
    void numeric(int value) {}
    void numeric(long value) {}

    void run(OverloadCoffee receiver, String label, Object unknown) {
        receiver.pour("espresso");
        receiver.pour(2);
        receiver.pour(label);
        receiver.pour(unknown);
        receiver.count();
        receiver.count(1);
        receiver.count(1, 2);
        receiver.batch("coffee", 1, 2);
        receiver.uncertain("coffee");
        receiver.generic("coffee");
        receiver.numeric(1);
        receiver.batch("coffee");
        receiver.batch("coffee", "wrong");
        Runnable known = receiver::reference;
        java.util.function.Consumer<String> ambiguous = receiver::pour;
    }
}
