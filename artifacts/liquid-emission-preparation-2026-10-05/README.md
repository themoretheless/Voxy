# Prepared midpoint emission

13 focused release controls pass. Shared immutable preparation produces particle
inputs for the midpoint estimate without mutating or copying liquid state. The
corrected finite-source path performs one explicit fluid clone and one canonical
exchange. Second-order position/velocity convergence, mass/momentum/energy
admission, species conservation and native late-frame rollback remain passing.

This structurally removes the first full-fluid clone/exchange but is not a
measured throughput claim. Two particle preparations and remaining transaction
copies still need performance qualification. No new GPU preview was generated.
