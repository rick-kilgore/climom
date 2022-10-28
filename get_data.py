#!/usr/bin/env python3
# pyre-strict

from subprocess import DEVNULL, Popen, PIPE
import sys

pcmfile = sys.argv[1]
maxlen = 90000
if len(sys.argv) > 2:
  maxlen = int(sys.argv[2])

with Popen(f"od -f -An {pcmfile}".split(), text=True, stdout=PIPE, stderr=DEVNULL) as od:
  nline = 0
  for line in od.stdout:
    vals = line.strip().split()
    if len(vals) == 4:
      print(f"{vals[0]}  {vals[1]}")
      print(f"{vals[2]}  {vals[3]}")
      nline += 2
    else:
      continue

    if maxlen > 0 and nline >= maxlen:
      break

