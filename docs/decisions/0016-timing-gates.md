# 0016 Timing tests compare against a reference workload

Context: the "1,000 rows render under 100 ms" test took 27 ms on the development machine and 105 to 170 ms on shared
CI runners, so it failed about one run in three while measuring the runner.

Decision (owner, 2026-11-22): the test discards a warm-up run, takes the median of five, and scales the 100 ms budget
by a fixed DOM workload timed in the same run (42 ms on the development machine). On a reference-speed machine the
limit is still 100 ms. The test prints the median, the reference and the budget.

Why not raise the limit: that loosens the check everywhere. Why not rerun: reruns hid the cost and taught nothing.
