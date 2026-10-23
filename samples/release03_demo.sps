* The syntax buffer for the 0.3 release check.
DESCRIPTIVES score age response_time /STATISTICS=MEAN STDDEV
FREQUENCIES region /CHART=BARCHART
CROSSTABS region BY group /CELLS=COUNT ROW COLUMN EXPECTED
REGRESSION score WITH age
