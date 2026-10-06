#!/usr/bin/env python3
"""Seeds initial accounts the transfer driver moves value between."""

import sys

from helper_dbdb import Dbdb, DbdbError, DbdbGone


def main():
    print("first_create_keys: started", flush=True)
    with Dbdb() as db:
        # One transaction: both accounts appear together or not at all, so a
        # driver can never observe one seeded and the other missing.
        db.set("accountA", "full")
        db.set("accountB", "empty")
        db.commit()
    print("first_create_keys: finished - accountA(full) accountB(empty)", flush=True)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (DbdbGone, DbdbError) as e:
        # Seeding is the one thing that must work: without it every driver
        # invocation fails its existence assertions, so this is worth a non-zero
        # exit. No faults are injected during `first_`, so there is no transient
        # explanation for getting here.
        print("first_create_keys: FAILED to seed accounts: %s" % e, file=sys.stderr, flush=True)
        sys.exit(1)
