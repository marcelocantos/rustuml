# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,os,shutil,subprocess,time
from pathlib import Path
R=Path.cwd(); C=Path(__file__).parent
prior=R/'docs/parity-reviews/sequence-filtered-display-packet/checker-bce6bea-astra-f535/support/checker_build_gates.py'
s=prior.read_text(); s=s.replace("C=Path(__file__).parent; O=C/'independent'", "C=Path(__file__).parent; O=C/'independent'")
s=s.replace('0cde5d7e55139b63cb8e91f8e42acd473441f80a','61e6aeed6eaf686b3497fbc696aadc03a64e69a2').replace('bce6bea405d8b5efd55136f4768117115e7f634f','d491d4bda47faf90d4751484d45aa1387fb1b290').replace('review/sequence-packet-independent-f535ce04','review/sequence-consumer-independent-b4b6')
(C/'run_build_gates.py').write_text(s)
