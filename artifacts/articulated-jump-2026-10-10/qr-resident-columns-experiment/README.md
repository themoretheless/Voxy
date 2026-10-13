# Experimental resident ordered QR columns

The candidate executes all remaining QR columns in one workgroup invocation with the original per-column double-single arithmetic and 64-lane reduction order. A storage/workgroup barrier and uniform failure check separate columns. Invalid columns stop the loop without advancing the cursor. Production shader remains unchanged.

Actual Metal/M4 Max passed the complete captured 174-row, 14-system, 1764-coordinate physical operator at original tolerance 1e-14 without fallback. Selected 39-row active equality: nine alternating paired GPU intervals, complete storage bitwise identical; boundary widths 1919/1920/1921/4096, tiny positive QR pivot and exact dependence checks passed. Baseline median QR 5.564000 ms, candidate 5.153708 ms (1.07961x), nine faster pairs. Concurrent load uncontrolled.

Current encoder still submits 39 QR dispatches for both programs; after the candidate first dispatch finishes all columns, later candidate dispatches are no-ops. Thus this is not yet a one-dispatch end-to-end backend. Next step: test an explicit candidate dispatch-count contract while baseline keeps one dispatch per column, then paired complete physical-projection timing and broader safety/trajectory checks. Active-set selection/readbacks remain host-driven; this experiment does not solve their scheduling overhead or establish rendered FPS.
