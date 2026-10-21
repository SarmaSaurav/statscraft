* Phase 4 end-to-end: reliability, factor analysis, ROC, Welch and COMPUTE.
* A scale of six items in two clusters of three.
RELIABILITY i1 i2 i3 i4 i5 i6 /SCALE(Overall)
FACTOR i1 i2 i3 i4 i5 i6 /EXTRACTION=PC2
ONEWAY score BY group /STATISTICS=WELCH
ROC score BY status /POSITIVE(case)
COMPUTE centred = score - 50
DESCRIPTIVES centred
