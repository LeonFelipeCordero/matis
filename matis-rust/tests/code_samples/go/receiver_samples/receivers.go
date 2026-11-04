package receiver_samples

type LiteralLedger struct{}
type OtherLedger struct{}

func (ledger LiteralLedger) Record() {}
func (ledger *LiteralLedger) Flush() {}
func (ledger OtherLedger) Record() {}

func NewLedger() LiteralLedger { return LiteralLedger{} }
func LedgerPair() (LiteralLedger, OtherLedger) { return LiteralLedger{}, OtherLedger{} }

func DirectLedger() {
    ledger := LiteralLedger{}
    ledger.Record()
}

func PointerLedger() {
    ledger := &LiteralLedger{}
    ledger.Flush()
}

func PairedLiterals() {
    ledger, other := LiteralLedger{}, OtherLedger{}
    ledger.Record()
    other.Record()
}

func FactoryLedger() {
    ledger := NewLedger()
    ledger.Record()
}

func FactoryPair() {
    ledger, other := LedgerPair()
    ledger.Record()
    other.Record()
}

func ReassignedLedger() {
    ledger := LiteralLedger{}
    ledger = NewLedger()
    ledger.Record()
}

func ShadowedLedger() {
    ledger := LiteralLedger{}
    {
        ledger := NewLedger()
        ledger.Record()
    }
    ledger.Record()
}

func DereferencedLedger() {
    ledger := *(&LiteralLedger{})
    ledger.Record()
}
